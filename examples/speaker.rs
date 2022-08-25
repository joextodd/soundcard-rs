use std::fs::File;
use std::path::Path;

use soundcard::{Config, Speaker};

fn main() {
    let mut input_file = File::open(Path::new("data/piano.wav")).unwrap();
    let (header, data) = wav::read(&mut input_file).unwrap();
    let audio = data.try_into_sixteen().unwrap();

    let config = Config {
        sample_rate: Some(header.sampling_rate as f64),
        num_channels: Some(header.channel_count as u32),
        block_size: None,
    };
    let duration_ms: u64 =
        (audio.len() as u32 / header.channel_count as u32 / header.sampling_rate * 1000)
            .try_into()
            .unwrap();

    let mut speaker = Speaker::default(config).unwrap();
    let tx = speaker.start::<i16>().unwrap();
    {
        let mut buffer = tx.lock().unwrap();
        buffer.extend(audio);
    }

    std::thread::sleep(std::time::Duration::from_millis(duration_ms));
    speaker.stop().unwrap();
}
