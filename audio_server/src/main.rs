use log::{debug, info, warn};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use gst_wrapper::gst_player::GstPlayer;
use gst_wrapper::gst_recorder::GstRecorder;

use tonic::{transport::Server, Request, Response, Status};

use clap::Parser;
use reachy_api::component::audio::audio_service_server::{AudioService, AudioServiceServer};
use reachy_api::component::audio::{
    audio_file_request, AudioAck, AudioFile, AudioFileRequest, AudioFiles,
};
use reachy_api::error::Error;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// grpc server ip
    #[arg(long, default_value = "[::1]")]
    grpc_host: String,

    /// grpc server port
    #[arg(long, default_value_t = 50063)]
    grpc_port: u16,
}

enum GstStatus {
    Play,
    Record,
    StopPlaying,
    StopRecording,
}

pub struct SDKAudioService {
    sounds_path: PathBuf,
    tx: mpsc::Sender<(GstStatus, Option<String>, Option<f32>)>,
}

impl SDKAudioService {
    pub async fn new() -> Self {
        let mut sounds_path = env::temp_dir();
        sounds_path.push("Reachy_SDK_audio_server");

        let tx = SDKAudioService::spawn_sync_thread().await;

        Self { sounds_path, tx }
    }

    async fn spawn_sync_thread() -> mpsc::Sender<(GstStatus, Option<String>, Option<f32>)> {
        let mut player: Option<GstPlayer> = None;
        let mut recorder: Option<GstRecorder> = None;
        let (tx, mut rx) = mpsc::channel::<(GstStatus, Option<String>, Option<f32>)>(2);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let (status, path, duration) = message;
                match status {
                    GstStatus::Play => {
                        if let Some(path) = path {
                            let mut gst_player = GstPlayer::new(path.as_str());
                            gst_player.play();
                            player = Some(gst_player);
                        } else {
                            warn!("No path provided to play audio file");
                        }
                    }
                    GstStatus::Record => {
                        if let Some(path) = path {
                            let mut gst_recorder = GstRecorder::new(path.as_str());
                            if let Some(duration) = duration {
                                gst_recorder.record(Duration::from_secs_f32(duration));
                            } else {
                                gst_recorder.record(Duration::from_secs_f32(60f32));
                                warn!("Recording time unset. Recording one minute.");
                            }
                            recorder = Some(gst_recorder);
                        } else {
                            warn!("No path provided to record audio file");
                        }
                    }
                    GstStatus::StopPlaying => {
                        if let Some(p) = player.as_mut() {
                            p.stop();
                        }
                    }
                    GstStatus::StopRecording => {
                        if let Some(r) = recorder.as_mut() {
                            r.stop();
                        }
                    }
                }
            }
        });
        tx
    }

    pub fn list_audio_files(&self) -> Vec<AudioFile> {
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.sounds_path) {
            for entry in entries {
                let path = entry.unwrap().path();
                if let Some(extension) = path.extension() {
                    if extension == "mp3" || extension == "wav" || extension == "ogg" {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                files.push(AudioFile {
                                    path: file_name_str.to_string(),
                                    duration: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        files
    }
}

#[tonic::async_trait]
impl AudioService for SDKAudioService {
    async fn get_audio_files(&self, request: Request<()>) -> Result<Response<AudioFiles>, Status> {
        debug!(
            "Got a get_audio_files request from {:?}",
            request.remote_addr()
        );

        let files = self.list_audio_files();

        let reply = AudioFiles { files };
        Ok(Response::new(reply))
    }

    async fn upload_audio_file(
        &self,
        request: Request<tonic::Streaming<AudioFileRequest>>,
    ) -> Result<Response<AudioAck>, Status> {
        debug!(
            "Got a upload_audio_file request from {:?}",
            request.remote_addr()
        );

        let mut stream = request.into_inner();
        let mut file: Option<File> = None;

        while let Some(audiofile_request) = stream.next().await {
            match audiofile_request {
                Ok(audiofile_request) => match audiofile_request.data {
                    Some(audio_file_request::Data::Info(info)) => {
                        let mut path = self.sounds_path.clone();
                        path.push(info.path);

                        file = Some(File::create(path).map_err(|e| {
                            Status::internal(format!("Failed to create file: {}", e))
                        })?);
                    }
                    Some(audio_file_request::Data::ChunkData(chunk_data)) => {
                        if let Some(file) = file.as_mut() {
                            file.write_all(&chunk_data).map_err(|e| {
                                Status::internal(format!("Failed to write to file: {}", e))
                            })?;
                        } else {
                            return Ok(Response::new(AudioAck {
                                success: Some(false),
                                error: Some(Error {
                                    details: "File not initialized".to_string(),
                                }),
                            }));
                        }
                    }
                    None => {
                        return Ok(Response::new(AudioAck {
                            success: Some(false),
                            error: Some(Error {
                                details: "No data provided".to_string(),
                            }),
                        }));
                    }
                },
                Err(e) => {
                    return Err(Status::internal(format!("Error receiving stream: {}", e)));
                }
            }
        }

        Ok(Response::new(AudioAck {
            success: Some(true),
            error: None,
        }))
    }

    type DownloadAudioFileStream = ReceiverStream<Result<AudioFileRequest, Status>>;

    async fn download_audio_file(
        &self,
        request: Request<AudioFile>,
    ) -> Result<Response<Self::DownloadAudioFileStream>, Status> {
        debug!(
            "Got a download_audio_file request from {:?}",
            request.remote_addr()
        );

        let name = request.into_inner().path;
        let mut path = self.sounds_path.clone();
        path.push(&name);

        let mut file = File::open(&path)
            .map_err(|e| Status::internal(format!("Failed to open file: {}", e)))?;

        let mut buffer = vec![0; 64 * 1024]; //64KB buffer
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tx.send(Ok(AudioFileRequest {
            data: Some(audio_file_request::Data::Info(AudioFile {
                path: name.to_string(),
                duration: None,
            })),
        }))
        .await
        .expect("Failed to send file name");

        tokio::spawn(async move {
            loop {
                let n = file
                    .read(&mut buffer)
                    .map_err(|e| Status::internal(format!("Failed to read file: {}", e)))
                    .unwrap_or(0);

                if n == 0 {
                    break;
                }

                let chunk = buffer[..n].to_vec();
                if tx
                    .send(Ok(AudioFileRequest {
                        data: Some(audio_file_request::Data::ChunkData(chunk)),
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::from(rx)))
    }

    async fn remove_audio_file(
        &self,
        request: Request<AudioFile>,
    ) -> Result<Response<AudioAck>, Status> {
        debug!(
            "Got a remove_audio_file request from {:?}",
            request.remote_addr()
        );

        let mut path = self.sounds_path.clone();
        path.push(request.into_inner().path);

        if path.exists() {
            fs::remove_file(path)
                .map_err(|e| Status::internal(format!("Failed to remove file: {}", e)))?;
        } else {
            return Ok(Response::new(AudioAck {
                success: Some(false),
                error: Some(Error {
                    details: "File not found".to_string(),
                }),
            }));
        }

        Ok(Response::new(AudioAck {
            success: Some(true),
            error: None,
        }))
    }

    async fn play_audio_file(&self, request: Request<AudioFile>) -> Result<Response<()>, Status> {
        debug!(
            "Got a play_audio_file request from {:?}",
            request.remote_addr()
        );
        let mut path = self.sounds_path.clone();
        path.push(request.into_inner().path);

        let _ = self
            .tx
            .send((
                GstStatus::Play,
                Some(path.to_str().unwrap().to_string()),
                None,
            ))
            .await;
        Ok(Response::new(()))
    }

    async fn stop_playing(&self, request: Request<()>) -> Result<Response<()>, Status> {
        debug!(
            "Got a stop_audio_file request from {:?}",
            request.remote_addr()
        );
        let _ = self.tx.send((GstStatus::StopPlaying, None, None)).await;
        Ok(Response::new(()))
    }

    async fn record_audio_file(&self, request: Request<AudioFile>) -> Result<Response<()>, Status> {
        debug!(
            "Got a record_audio_file request from {:?}",
            request.remote_addr()
        );
        let audiofile = request.into_inner();
        let mut path = self.sounds_path.clone();
        path.push(audiofile.path);

        let _ = self
            .tx
            .send((
                GstStatus::Record,
                Some(path.to_str().unwrap().to_string()),
                audiofile.duration,
            ))
            .await;
        Ok(Response::new(()))
    }

    async fn stop_recording(&self, request: Request<()>) -> Result<Response<()>, Status> {
        debug!(
            "Got a stop_audio_file request from {:?}",
            request.remote_addr()
        );
        let _ = self.tx.send((GstStatus::StopRecording, None, None)).await;
        Ok(Response::new(()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting SDK Audio server");
    env_logger::init();
    gst::init().unwrap();

    let args = Args::parse();
    let grpc_address = format!("{}:{}", args.grpc_host, args.grpc_port)
        .parse()
        .unwrap();

    //let addr = "[::1]:50063".parse().unwrap();
    let audioservice = SDKAudioService::new().await;

    info!("AudioService listening on {}", grpc_address);

    Server::builder()
        .add_service(AudioServiceServer::new(audioservice))
        .serve(grpc_address)
        .await?;

    info!("Exit SDK Audio server");

    Ok(())
}
