use gst::prelude::*;
use log::{debug, info, warn};
use std::path::PathBuf;
use std::{env, fs, path::Path, thread, time::Duration};
mod gstreamer;
use tokio::sync::mpsc;

use gstreamer::gst_player::GstPlayer;
use gstreamer::gst_recorder::GstRecorder;

use tonic::{transport::Server, Request, Response, Status};

use reachy_api::component::audio::audio_service_server::{AudioService, AudioServiceServer};
use reachy_api::component::audio::{AudioFile, AudioFiles};

enum GstStatus {
    Play,
    Record,
    StopPlaying,
    StopRecording,
}

pub struct SDKAudioService {
    sounds_path: PathBuf,

    tx: mpsc::Sender<(GstStatus, Option<String>)>,
}

impl SDKAudioService {
    pub async fn new() -> Self {
        let mut sounds_path = env::temp_dir();
        sounds_path.push("Reachy_SDK_audio_server");

        let tx = SDKAudioService::spawn_sync_thread().await;

        Self { sounds_path, tx }
    }

    async fn spawn_sync_thread() -> mpsc::Sender<(GstStatus, Option<String>)> {
        let mut player: Option<GstPlayer> = None;
        let mut recorder: Option<GstRecorder> = None;
        let (tx, mut rx) = mpsc::channel::<(GstStatus, Option<String>)>(2);
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let (status, path) = message;
                match status {
                    GstStatus::Play => {
                        if let Some(path) = path {
                            let mut gst_player = GstPlayer::new(&path.as_str());
                            gst_player.play();
                            player = Some(gst_player);
                        } else {
                            warn!("No path provided to play audio file");
                        }
                    }
                    GstStatus::Record => {
                        if let Some(path) = path {
                            let mut gst_recorder = GstRecorder::new(&path.as_str());
                            gst_recorder.record();
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

    pub fn list_audio_files(&self) -> Vec<String> {
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.sounds_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(extension) = path.extension() {
                        if extension == "mp3" || extension == "wav" || extension == "ogg" {
                            if let Some(file_name) = path.file_name() {
                                if let Some(file_name_str) = file_name.to_str() {
                                    files.push(file_name_str.to_string());
                                }
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

    async fn play_audio_file(&self, request: Request<AudioFile>) -> Result<Response<()>, Status> {
        debug!(
            "Got a play_audio_file request from {:?}",
            request.remote_addr()
        );
        let mut path = self.sounds_path.clone();
        path.push(request.into_inner().path);

        let _ = self
            .tx
            .send((GstStatus::Play, Some(path.to_str().unwrap().to_string())))
            .await;
        Ok(Response::new(()))
    }

    async fn stop_playing(&self, request: Request<()>) -> Result<Response<()>, Status> {
        debug!(
            "Got a stop_audio_file request from {:?}",
            request.remote_addr()
        );
        let _ = self.tx.send((GstStatus::StopPlaying, None)).await;
        Ok(Response::new(()))
    }

    async fn record_audio_file(&self, request: Request<AudioFile>) -> Result<Response<()>, Status> {
        debug!(
            "Got a record_audio_file request from {:?}",
            request.remote_addr()
        );
        let mut path = self.sounds_path.clone();
        path.push(request.into_inner().path);

        let _ = self
            .tx
            .send((GstStatus::Record, Some(path.to_str().unwrap().to_string())))
            .await;
        Ok(Response::new(()))
    }

    async fn stop_recording(&self, request: Request<()>) -> Result<Response<()>, Status> {
        debug!(
            "Got a stop_audio_file request from {:?}",
            request.remote_addr()
        );
        let _ = self.tx.send((GstStatus::StopRecording, None)).await;
        Ok(Response::new(()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting SDK Audio server");
    env_logger::init();
    gst::init().unwrap();

    let addr = "[::1]:50051".parse().unwrap();
    let audioservice = SDKAudioService::new().await;

    info!("AudioService listening on {}", addr);

    Server::builder()
        .add_service(AudioServiceServer::new(audioservice))
        .serve(addr)
        .await?;

    info!("Exit SDK Audio server");

    Ok(())
}
