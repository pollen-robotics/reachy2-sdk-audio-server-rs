use reachy_api::component::audio::audio_service_client::AudioServiceClient;
use reachy_api::component::audio::AudioFile;
use reachy_api::component::audio::{audio_file_request, AudioFileRequest};
use std::env;
use std::fs::File;
use std::io::Read;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

fn is_file_in_list(files: Vec<AudioFile>, file_name: &str) -> bool {
    let mut file_found = false;

    for audiofile in files {
        if audiofile.path == file_name {
            file_found = true;
        }
    }

    file_found
}

#[tokio::test]
async fn test_grpc() {
    let mut client = AudioServiceClient::connect("http://0.0.0.0:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let unit_file_name = "unit_test.ogg";
    let mut path = env::temp_dir();
    path.push("Reachy_SDK_audio_server");
    std::fs::create_dir_all(&path).unwrap();
    path.push(unit_file_name);

    let _ = File::create(path.to_str().unwrap()).unwrap();

    let response = client.get_audio_files(()).await.unwrap();

    let files = response.into_inner().files;

    assert!(is_file_in_list(files, unit_file_name));

    std::fs::remove_file(path).unwrap();

    let response = client.get_audio_files(()).await.unwrap();
    let files = response.into_inner().files;

    assert!(!is_file_in_list(files, unit_file_name));
}

#[tokio::test]
async fn test_upload_download_file() {
    let mut client = AudioServiceClient::connect("http://0.0.0.0:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let mut file_path = env::current_dir().unwrap();
    file_path.push("../data/");
    file_path.push("sample-3.ogg");
    println!("{}", file_path.to_str().unwrap());

    let mut file = File::open(&file_path).expect("Failed to open file");

    let mut buffer = vec![0; 64 * 1024];
    let (tx, rx) = mpsc::channel(1);

    tx.send(AudioFileRequest {
        data: Some(audio_file_request::Data::Info(AudioFile {
            path: "sample-3.ogg".to_string(),
            duration: None,
        })),
    })
    .await
    .expect("Failed to send file name");

    tokio::spawn(async move {
        loop {
            let n = file.read(&mut buffer).expect("Failed to read file");

            if n == 0 {
                break;
            }

            let chunk = buffer[..n].to_vec();
            tx.send(AudioFileRequest {
                data: Some(audio_file_request::Data::ChunkData(chunk)),
            })
            .await
            .expect("Failed to send chunk");
        }
    });

    let stream = ReceiverStream::new(rx);
    let response = client
        .upload_audio_file(stream)
        .await
        .expect("Failed to upload file");

    let ack = response.into_inner();
    assert!(ack.success.unwrap());
    assert!(ack.error.is_none());

    let mut file = File::open(file_path).expect("Failed to open file");
    let mut original_data = Vec::new();
    file.read_to_end(&mut original_data)
        .expect("Failed to read file");

    let response = client
        .download_audio_file(AudioFile {
            path: "sample-3.ogg".to_string(),
            duration: None,
        })
        .await
        .expect("Failed to send download request");

    let mut stream = response.into_inner();
    let mut received_data = Vec::new();
    let mut file_name = None;

    while let Some(audiofile_request) = stream.message().await.unwrap() {
        match audiofile_request.data {
            Some(audio_file_request::Data::Info(info)) => {
                assert!(file_name.is_none());
                file_name = Some(info.path);
            }
            Some(audio_file_request::Data::ChunkData(chunk_data)) => {
                received_data.extend(chunk_data);
            }
            None => {
                assert!(false);
            }
        }
    }

    assert_eq!(file_name, Some("sample-3.ogg".to_string()));
    assert_eq!(received_data, original_data);
}

#[tokio::test]
async fn test_upload_file_no_data() {
    let mut client = AudioServiceClient::connect("http://0.0.0.0:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let (tx, rx) = mpsc::channel(1);

    tx.send(AudioFileRequest { data: None })
        .await
        .expect("Failed to send file name");

    let stream = ReceiverStream::new(rx);
    let response = client
        .upload_audio_file(stream)
        .await
        .expect("Failed to upload file");

    let ack = response.into_inner();
    assert!(!ack.success.unwrap());
    assert!(ack.error.is_some());
}

#[tokio::test]
async fn test_upload_file_before_name() {
    let mut client = AudioServiceClient::connect("http://0.0.0.0:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let (tx, rx) = mpsc::channel(1);

    let dummy_vec = vec![0; 64];

    tx.send(AudioFileRequest {
        data: Some(audio_file_request::Data::ChunkData(dummy_vec)),
    })
    .await
    .expect("Failed to send file name");

    let stream = ReceiverStream::new(rx);
    let response = client
        .upload_audio_file(stream)
        .await
        .expect("Failed to upload file");

    let ack = response.into_inner();
    assert!(!ack.success.unwrap());
    assert!(ack.error.is_some());
}

#[tokio::test]
async fn test_remove_file() {
    let mut client = AudioServiceClient::connect("http://0.0.0.0:50063")
        .await
        .expect("Failed to connect to server. Make sure that server is running for this test!");

    let audiofile = AudioFile {
        path: "dummy".to_string(),
        duration: None,
    };

    let response = client.remove_audio_file(audiofile).await.unwrap();
    let ack = response.into_inner();
    assert!(!ack.success.unwrap());
    assert!(ack.error.is_some());

    let unit_file_name = "unit_test2.ogg";
    let mut path = env::temp_dir();
    path.push("Reachy_SDK_audio_server");
    std::fs::create_dir_all(&path).unwrap();
    path.push(unit_file_name);

    File::create(path.to_str().unwrap()).unwrap();

    let audiofile = AudioFile {
        path: unit_file_name.to_string(),
        duration: None,
    };

    let response = client.remove_audio_file(audiofile).await.unwrap();
    let ack = response.into_inner();
    assert!(ack.success.unwrap());
    assert!(ack.error.is_none());
}
