use reachy_api::component::audio::audio_service_client::AudioServiceClient;
use reachy_api::component::audio::AudioFile;

use std::env;
use std::{thread, time::Duration};

#[tokio::test]
async fn test_playback_recording() {
    let mut client = AudioServiceClient::connect("http://[::1]:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let unit_file_name = "test_SDK_recording.ogg";
    let mut path = env::temp_dir();
    path.push("Reachy_SDK_audio_server");
    path.push(unit_file_name);

    let audiofile = AudioFile {
        path: path.to_str().unwrap().to_string(),
    };

    client.record_audio_file(audiofile.clone()).await.unwrap();

    println!("recording for 4 seconds");
    thread::sleep(Duration::from_secs(4));

    println!("stopping recording");
    client.stop_recording(()).await.unwrap();

    println!("playing 2 secs of recording");
    client.play_audio_file(audiofile).await.unwrap();

    thread::sleep(Duration::from_secs(2));

    println!("stopping playback");
    client.stop_playing(()).await.unwrap();

    std::fs::remove_file(path).unwrap();
}
