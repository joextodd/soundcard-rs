//!
//! CoreAudio module
//!
extern crate coreaudio;

use std::clone::Clone;
use std::collections::VecDeque;
use std::fmt;
use std::sync::{mpsc, Arc, Mutex};

use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers;
use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{Element, Sample, SampleFormat, Scope, StreamFormat};
use coreaudio::sys::{kAudioUnitProperty_StreamFormat, AudioStreamBasicDescription};

use crate::{Config, Device, Microphone, SoundCard, SoundCardError, Speaker};

impl SoundCard {

    pub fn get_device(id: u32, scope: Scope, element: Element) -> Result<Device, SoundCardError> {
        let audio_unit = macos_helpers::audio_unit_from_device_id(id, true)?;
        if let Ok(desc) = audio_unit.get_property::<AudioStreamBasicDescription>(
            kAudioUnitProperty_StreamFormat,
            scope,
            element,
        ) {
            let name = macos_helpers::get_device_name(id)
                .unwrap_or_else(|_| "Unknown".to_string());
            let device = Device {
                id,
                name,
                channels: desc.mChannelsPerFrame,
                sample_rate: desc.mSampleRate,
            };
            Ok(device)
        } else {
            Err(SoundCardError::NoDevicesFound)
        }
    }

    /// Returns an array of the available output devices
    /// on the soundcard.
    ///
    /// ```
    /// use soundcard::SoundCard;
    /// let speakers = SoundCard::all_speakers();
    /// ```
    pub fn all_speakers() -> Result<Vec<Device>, SoundCardError> {
        let mut devices: Vec<Device> = Vec::new();
        if let Ok(ids) = macos_helpers::get_audio_device_ids() {
            for id in ids {
                let device = SoundCard::get_device(id, Scope::Output, Element::Output)?;
                if device.channels > 0 {
                    devices.push(device)
                }
            }
            Ok(devices)
        }
        else {
            Err(SoundCardError::NoDevicesFound)
        }
    }

    /// Returns an array of the available input devices
    /// on the soundcard.
    ///
    /// ```
    /// use soundcard::SoundCard;
    /// let microphones = SoundCard::all_microphones();
    /// ```
    pub fn all_microphones() -> Result<Vec<Device>, SoundCardError> {
        let mut devices: Vec<Device> = Vec::new();
        if let Ok(ids) = macos_helpers::get_audio_device_ids() {
            for id in ids {
                let device = SoundCard::get_device(id, Scope::Input, Element::Input)?;
                if device.channels > 0 {
                    devices.push(device)
                }
            }
            Ok(devices)
        }
        else {
            Err(SoundCardError::NoDevicesFound)
        }
    }
}

impl Microphone {
    /// Create a new microphone instance using its ID
    /// from SoundCard::all_devices.
    pub fn new(id: u32, config: Config) -> Option<Self> {
        match macos_helpers::audio_unit_from_device_id(id, true) {
            Ok(audio_unit) => {
                let device = SoundCard::get_device(id, Scope::Input, Element::Input).unwrap();
                let microphone = Self { audio_unit, config, device };
                Some(microphone)
            }
            Err(_) => None,
        }
    }

    /// Create a new microphone instance using
    /// the system default input device.
    ///
    /// ```
    /// use soundcard::{Config, Microphone};
    /// let config = Config::default();
    /// let default_mic = Microphone::default(config);
    /// ```
    pub fn default(config: Config) -> Option<Self> {
        match macos_helpers::get_default_device_id(true) {
            Some(id) => Self::new(id, config),
            None => None,
        }
    }

    pub fn start<T: 'static + Sample + Clone>(
        &mut self,
    ) -> Result<mpsc::Receiver<Vec<T>>, SoundCardError> {
        let (tx, rx): (mpsc::Sender<Vec<T>>, mpsc::Receiver<Vec<T>>) = mpsc::channel();
        let pcm_flag = match T::sample_format() {
            SampleFormat::F32 => LinearPcmFlags::IS_FLOAT,
            _ => LinearPcmFlags::IS_SIGNED_INTEGER,
        };
        let format = StreamFormat {
            sample_rate: self.config.sample_rate.unwrap_or(self.device.sample_rate),
            sample_format: T::sample_format(),
            flags: pcm_flag | LinearPcmFlags::IS_PACKED,
            channels: self.config.num_channels.unwrap_or(self.device.channels),
        };
        let desc = format.to_asbd();
        self.audio_unit.set_property(
            kAudioUnitProperty_StreamFormat,
            Scope::Output,
            Element::Input,
            Some(&desc),
        )?;

        type Args<T> = render_callback::Args<data::Interleaved<T>>;
        self.audio_unit.set_input_callback(move |args: Args<T>| {
            let Args { data, .. } = args;
            match tx.send(data.buffer.to_vec()) {
                Ok(()) => {},
                Err(_) => {},
            }
            Ok(())
        })?;
        self.audio_unit.start()?;
        Ok(rx)
    }

    /// Stop audio processing
    pub fn stop(&mut self) -> Result<(), SoundCardError> {
        self.audio_unit.stop()?;
        Ok(())
    }
}

impl Speaker {
    pub fn new(id: u32, config: Config) -> Option<Self> {
        match macos_helpers::audio_unit_from_device_id(id, false) {
            Ok(audio_unit) => {
                let device = SoundCard::get_device(id, Scope::Output, Element::Output).unwrap();
                let speaker = Self { audio_unit, config, device };
                Some(speaker)
            }
            Err(_) => None,
        }
    }

    /// Create a new speaker instance using
    /// the system default output device.
    ///
    /// ```
    /// use soundcard::{Config, Speaker};
    /// let config = Config::default();
    /// let default_speaker = Speaker::default(config);
    /// ```
    pub fn default(config: Config) -> Option<Self> {
        match macos_helpers::get_default_device_id(false) {
            Some(id) => Self::new(id, config),
            None => None,
        }
    }

    pub fn start<T: 'static + Sample + Copy + Send + Sync>(
        &mut self,
    ) -> Result<Arc<Mutex<VecDeque<T>>>, SoundCardError> {
        let buffer = VecDeque::<T>::new();
        let buffer_ref = Arc::new(Mutex::new(buffer));
        let pcm_flag = match T::sample_format() {
            SampleFormat::F32 => LinearPcmFlags::IS_FLOAT,
            _ => LinearPcmFlags::IS_SIGNED_INTEGER,
        };
        let format = StreamFormat {
            sample_rate: self.config.sample_rate.unwrap_or(self.device.sample_rate),
            sample_format: T::sample_format(),
            flags: pcm_flag | LinearPcmFlags::IS_PACKED,
            channels: self.config.num_channels.unwrap_or(self.device.channels),
        };
        let desc = format.to_asbd();
        self.audio_unit.set_property(
            kAudioUnitProperty_StreamFormat,
            Scope::Input,
            Element::Output,
            Some(&desc),
        )?;

        let num_channels: usize = format.channels as usize;
        let render_buffer = buffer_ref.clone();
        type Args<T> = render_callback::Args<data::Interleaved<T>>;
        self.audio_unit.set_render_callback(move |args: Args<T>| {
            let Args {
                num_frames, data, ..
            } = args;
            match render_buffer.lock() {
                Ok(mut buffer) => {
                    let num_samples = std::cmp::min(buffer.len(), num_frames * num_channels);
                    let samples: VecDeque<T> = buffer.drain(..num_samples).collect::<VecDeque<T>>();
                    for (i, sample) in samples.iter().enumerate() {
                        data.buffer[i] = *sample;
                    }
                }
                Err(_) => {}
            }
            Ok(())
        })?;
        self.audio_unit.start()?;
        Ok(buffer_ref)
    }

    /// Stop audio processing
    pub fn stop(&mut self) -> Result<(), SoundCardError> {
        match self.audio_unit.stop() {
            Ok(()) => Ok(()),
            Err(e) => Err(SoundCardError::CoreAudioError(e.to_string())),
        }
    }
}

impl std::convert::From<coreaudio::Error> for SoundCardError {
    fn from(err: coreaudio::Error) -> SoundCardError {
        SoundCardError::CoreAudioError(err.to_string())
    }
}

impl fmt::Display for SoundCard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut display = String::new();
        display.push_str("Speakers: \n");
        match SoundCard::all_speakers() {
            Ok(speakers) => {
                for speaker in speakers {
                    display.push_str(&format!(
                        "  {}: {} ({} out)\n",
                        speaker.id, speaker.name, speaker.channels
                    ));
                }
            }
            Err(_) => {}
        }
        display.push_str("\n");
        display.push_str("Microphones: \n");
        match SoundCard::all_microphones() {
            Ok(microphones) => {
                for mic in microphones {
                    display.push_str(&format!(
                        "  {}: {} ({} in)\n",
                        mic.id, mic.name, mic.channels
                    ));
                }
            }
            Err(_) => {}
        }
        write!(f, "{}", display)
    }
}
