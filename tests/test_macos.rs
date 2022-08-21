use soundcard::SoundCard;

#[test]
fn test_all_speakers() {
    let speakers = SoundCard::all_speakers().unwrap();
    assert_eq!(speakers.len() > 0, true);
}

#[test]
fn test_all_microphones() {
    let mics = SoundCard::all_microphones().unwrap();
    assert_eq!(mics.len() > 0, true);
}
