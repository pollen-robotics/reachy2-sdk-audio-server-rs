use gst::prelude::*;
use log::info;
use std::{thread, time::Duration};
mod gstreamer;

use gstreamer::gst_player::GstPlayer;
use gstreamer::gst_recorder::GstRecorder;

fn main() {
    info!("Starting SDK Audio server");
    env_logger::init();
    gst::init().unwrap();

    /*let mut player = GstPlayer::new("/home/fabien/Music/lets-play_2.0_.ogg");

    player.play();

    thread::sleep(Duration::from_secs(4));

    player.stop();*/

    let path = "/home/fabien/Music/test_SDK_recording.ogg";

    let mut recorder = GstRecorder::new(path);

    recorder.record();

    thread::sleep(Duration::from_secs(4));

    recorder.stop();

    info!("next song");

    let mut player = GstPlayer::new(path);

    player.play();

    thread::sleep(Duration::from_secs(4));

    player.stop();

    info!("Exit SDK Audio server");
}
