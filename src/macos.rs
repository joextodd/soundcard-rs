extern crate coreaudio;

use std::clone::Clone;
use std::sync::mpsc;

use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers;
use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{Element, Sample, SampleFormat, Scope, StreamFormat};
use coreaudio::sys::{kAudioUnitProperty_StreamFormat, AudioStreamBasicDescription};

use crate::{
    Config, Device, Microphone, SoundCard, SoundCardError, Speaker,
};

impl std::convert::From<coreaudio::Error> for SoundCardError {
    fn from(err: coreaudio::Error) -> SoundCardError {
        SoundCardError::CoreAudioError(err.to_string())
    }
}

impl SoundCard {
    fn _all_devices(scope: Scope, element: Element) -> Result<Vec<Device>, SoundCardError> {
        let mut devices: Vec<Device> = Vec::new();
        if let Ok(ids) = macos_helpers::get_audio_device_ids() {
            for id in ids {
                let audio_unit = macos_helpers::audio_unit_from_device_id(id, true).unwrap();
                if let Ok(desc) = audio_unit.get_property::<AudioStreamBasicDescription>(
                    kAudioUnitProperty_StreamFormat,
                    scope,
                    element,
                ) {
                    if desc.mChannelsPerFrame > 0 {
                        let name =
                            macos_helpers::get_device_name(id).unwrap_or_else(|_| "Unknown".to_string());
                        let device = Device { id, name };
                        devices.push(device);
                    }
                }
            }
            Ok(devices)
        } else {
            Err(SoundCardError::GenericError)
        }
    }

    pub fn all_speakers() -> Result<Vec<Device>, SoundCardError> {
        SoundCard::_all_devices(Scope::Output, Element::Output)
    }

    pub fn all_microphones() -> Result<Vec<Device>, SoundCardError> {
        SoundCard::_all_devices(Scope::Input, Element::Input)
    }
}

impl Microphone {
    pub fn new(id: u32, config: Config) -> Option<Self> {
        let audio_unit = macos_helpers::audio_unit_from_device_id(id, true).unwrap();
        let microphone = Self { audio_unit, config };
        Some(microphone)
    }

    pub fn default(config: Config) -> Option<Self> {
        let id = macos_helpers::get_default_device_id(true).unwrap();
        let audio_unit = macos_helpers::audio_unit_from_device_id(id, true).unwrap();
        let microphone = Self { audio_unit, config };
        Some(microphone)
    }

    pub fn start<T: 'static + Sample + Copy>(&mut self) -> mpsc::Receiver<T> {
        let (sender, receiver): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel();
        let pcm_flag = match T::sample_format() {
            SampleFormat::F32 => LinearPcmFlags::IS_FLOAT,
            _ => LinearPcmFlags::IS_SIGNED_INTEGER,
        };
        let format = StreamFormat {
            sample_rate: self.config.sample_rate,
            sample_format: T::sample_format(),
            flags: pcm_flag | LinearPcmFlags::IS_PACKED,
            channels: self.config.num_channels,
        };
        let desc = format.to_asbd();
        self.audio_unit
            .set_property(
                kAudioUnitProperty_StreamFormat,
                Scope::Output,
                Element::Input,
                Some(&desc),
            )
            .unwrap();

        let num_channels: usize = self.config.num_channels as usize;
        type Args<T> = render_callback::Args<data::Interleaved<T>>;
        self.audio_unit
            .set_input_callback(move |args: Args<T>| {
                let Args {
                    num_frames, data, ..
                } = args;
                for i in 0..num_frames {
                    for channel in 0..num_channels {
                        let idx = num_channels * i + channel;
                        sender.send(data.buffer[idx]).unwrap();
                    }
                }
                Ok(())
            })
            .unwrap();
        self.audio_unit.start().unwrap();
        receiver
    }

    pub fn stop(&mut self) {
        self.audio_unit.stop().unwrap();
    }
}

impl Speaker {
    pub fn new(id: u32, config: Config) -> Option<Self> {
        let audio_unit = macos_helpers::audio_unit_from_device_id(id, false).unwrap();
        let speaker = Self { audio_unit, config };
        Some(speaker)
    }

    pub fn default(config: Config) -> Option<Self> {
        let id = macos_helpers::get_default_device_id(false).unwrap();
        let audio_unit = macos_helpers::audio_unit_from_device_id(id, false).unwrap();
        let speaker = Self { audio_unit, config };
        Some(speaker)
    }

    pub fn start<T: 'static + Sample + Clone>(&mut self) -> mpsc::Sender<T> {
        let (sender, receiver): (mpsc::Sender<T>, mpsc::Receiver<T>) = mpsc::channel();
        let pcm_flag = match T::sample_format() {
            SampleFormat::F32 => LinearPcmFlags::IS_FLOAT,
            _ => LinearPcmFlags::IS_SIGNED_INTEGER,
        };
        let format = StreamFormat {
            sample_rate: self.config.sample_rate,
            sample_format: T::sample_format(),
            flags: pcm_flag | LinearPcmFlags::IS_PACKED,
            channels: self.config.num_channels,
        };
        let desc = format.to_asbd();
        self.audio_unit
            .set_property(
                kAudioUnitProperty_StreamFormat,
                Scope::Input,
                Element::Output,
                Some(&desc),
            )
            .unwrap();

        let num_channels: usize = self.config.num_channels as usize;
        type Args<T> = render_callback::Args<data::Interleaved<T>>;
        self.audio_unit
            .set_render_callback(move |args: Args<T>| {
                let Args {
                    num_frames, data, ..
                } = args;
                for i in 0..num_frames {
                    for channel in 0..num_channels {
                        let idx = num_channels * i + channel;
                        match receiver.try_recv() {
                            Ok(sample) => {
                                data.buffer[idx] = sample;
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                }
                Ok(())
            })
            .unwrap();
        self.audio_unit.start().unwrap();
        sender
    }

    pub fn stop(&mut self) {
        self.audio_unit.stop().unwrap();
    }
}
