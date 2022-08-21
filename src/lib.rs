use thiserror::Error;

pub struct SoundCard;

#[cfg(target_os = "macos")]
pub mod macos;

pub struct Device {
    pub id: u32,
    pub name: String,
}

pub struct Config {
    pub sample_rate: f64,
    pub num_channels: u32,
}

pub struct Microphone {
    config: Config,
    #[cfg(target_os = "macos")]
    audio_unit: coreaudio::audio_unit::AudioUnit,
}

pub struct Speaker {
    config: Config,
    #[cfg(target_os = "macos")]
    audio_unit: coreaudio::audio_unit::AudioUnit,
}

#[derive(Debug, Error, PartialEq)]
pub enum SoundCardError {
    #[error("Some generic error occurred")]
    GenericError,
    #[error("CoreAudio error: `{0}`")]
    CoreAudioError(String),
}
