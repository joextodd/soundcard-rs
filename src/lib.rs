use thiserror::Error;

/// The machines soundcard
///
/// ```
/// use soundcard::SoundCard;
/// println!("Available devices: {}", SoundCard);
/// ```
pub struct SoundCard;

#[cfg(target_os = "macos")]
pub mod macos;

/// An audio device can be identified by
/// its ID or its readable name
#[derive(Debug, PartialEq)]
pub struct Device {
    /// The device ID, used to create a new [Speaker] or [Microphone]
    pub id: u32,
    /// Human readable name of the device
    pub name: String,
    /// Maximum number of channels in the device
    pub channels: u32,
    /// Default sample rate of device
    pub sample_rate: f64,
}

pub enum Format {
    I16(i16),
    I32(i32),
    F32(f32),
}

/// Define the config parameters for
/// the [Device].
#[derive(Debug, Default)]
pub struct Config {
    /// The audio sample rate, e.g., 44100
    /// If not specified the device default is used
    pub sample_rate: Option<f64>,
    /// The number of channels to play/record
    /// If not specified the device default is used
    pub num_channels: Option<u32>,
    /// The size of each block of audio, if
    /// not specified then the OS decides
    pub block_size: Option<u32>,
}

/// A Microphone records audio from the
/// specified audio device and returns each
/// block of audio to the caller via a channel.
pub struct Microphone {
    device: Device,
    config: Config,
    #[cfg(target_os = "macos")]
    audio_unit: coreaudio::audio_unit::AudioUnit,
}

/// A Speaker will play audio out of the
/// specified audio device.
pub struct Speaker {
    device: Device,
    config: Config,
    #[cfg(target_os = "macos")]
    audio_unit: coreaudio::audio_unit::AudioUnit,
}

#[derive(Debug, Error, PartialEq)]
pub enum SoundCardError {
    #[error("No audio devices found")]
    NoDevicesFound,
    #[cfg(target_os = "macos")]
    #[error("CoreAudio error: `{0}`")]
    CoreAudioError(String),
}
