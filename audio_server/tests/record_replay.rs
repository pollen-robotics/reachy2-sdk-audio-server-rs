use gst_wrapper::gst_player::GstPlayer;
use gst_wrapper::gst_recorder::GstRecorder;

use std::{thread, time::Duration};

use std::path::Path;

use std::env;

#[test]
fn test_record_n_replay() {
    println!("starting recording test");
    env_logger::init();
    gst::init().unwrap();

    let mut path = env::temp_dir();
    path.push("Reachy_SDK_audio_server");

    std::fs::create_dir_all(&path).unwrap();

    path.push("test_SDK_recording.ogg");
    let path_str = path.to_str().unwrap();

    let mut recorder = GstRecorder::new(path_str);

    println!("recording for 4 seconds");

    recorder.record(Duration::from_secs(10));

    thread::sleep(Duration::from_secs(4));

    recorder.stop();

    println!("testing file");
    assert!(Path::new(path_str).exists());

    println!("playing back recording");

    let mut player = GstPlayer::new(path_str);

    player.play();

    thread::sleep(Duration::from_secs(4));

    player.stop();
}
