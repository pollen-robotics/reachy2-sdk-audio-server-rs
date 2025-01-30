use reachy_api::component::audio::audio_service_client::AudioServiceClient;
use reachy_api::component::audio::AudioFile;

use std::{thread, time::Duration};

#[tokio::test]
async fn test_playback_recording() {
    let mut client = AudioServiceClient::connect("http://0.0.0.0:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let unit_file_name = "test_SDK_recording.ogg";

    let audiofile = AudioFile {
        path: unit_file_name.to_string(),
        duration: Some(10.0f32),
    };

    client.record_audio_file(audiofile.clone()).await.unwrap();

    println!("recording for 4 seconds");
    thread::sleep(Duration::from_secs(4));

    println!("stopping recording");
    client.stop_recording(()).await.unwrap();

    println!("playing 2 secs of recording");
    client.play_audio_file(audiofile.clone()).await.unwrap();

    thread::sleep(Duration::from_secs(2));

    println!("stopping playback");
    client.stop_playing(()).await.unwrap();

    client.remove_audio_file(audiofile).await.unwrap();
}
