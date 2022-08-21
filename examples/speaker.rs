use std::fs::File;
use std::path::Path;

use soundcard::{Config, Speaker};

fn main() {
    let mut input_file = File::open(Path::new("data/piano.wav")).unwrap();
    let (header, data) = wav::read(&mut input_file).unwrap();
    let audio = data.try_into_sixteen().unwrap();

    let config = Config {
        sample_rate: header.sampling_rate as f64,
        num_channels: header.channel_count as u32,
    };
    let mut speaker = Speaker::default(config).unwrap();
    let tx = speaker.start::<i16>();
    for sample in audio {
        tx.send(sample).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_secs(6));
    speaker.stop();
}
