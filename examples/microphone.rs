use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use soundcard::{Config, Microphone};
use wav::bit_depth::BitDepth;
use wav::header::Header;

fn main() {
    let audio: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
    let header = Header::new(wav::WAV_FORMAT_PCM, 1, 44100, 16);
    let mut wav_file = File::create(Path::new("output.wav")).unwrap();

    let config = Config {
        sample_rate: 44100.0,
        num_channels: 1,
    };
    let mut mic = Microphone::default(config).unwrap();
    let rx = mic.start::<i16>();

    let thread_audio = audio.clone();
    std::thread::spawn(move || {
        while let Ok(sample) = rx.recv() {
            let mut audio = thread_audio.lock().unwrap();
            audio.push(sample);
        }
    });

    std::thread::sleep(std::time::Duration::from_secs(5));
    mic.stop();

    let audio_data = audio.lock().unwrap().clone();
    let track = BitDepth::Sixteen(audio_data);
    wav::write(header, &track, &mut wav_file).unwrap();
}
