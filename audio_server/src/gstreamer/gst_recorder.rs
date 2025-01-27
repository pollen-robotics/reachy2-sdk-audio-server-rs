use crate::gst_utils::add_element_by_name;
use crate::gst_utils::set_pipeline_state;
use crate::gst_utils::setup_bus_watch;
use gst::prelude::*;
use log::debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub struct GstRecorder {
    pipeline: gst::Pipeline,
    auto_stop_thread: Option<thread::JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl GstRecorder {
    pub fn new(path: &str) -> Self {
        let pipeline = gst::Pipeline::new();

        let autoaudiosrc = add_element_by_name("autoaudiosrc");
        let queue = add_element_by_name("queue");
        let audioconvert = add_element_by_name("audioconvert");
        let audioresample = add_element_by_name("audioresample");
        let opusenc = add_element_by_name("opusenc");
        let oggmux = add_element_by_name("oggmux");
        let filesink = add_element_by_name("filesink");
        filesink.set_property("location", path);

        let elements = &[
            &autoaudiosrc,
            &queue,
            &audioconvert,
            &audioresample,
            &opusenc,
            &oggmux,
            &filesink,
        ];
        pipeline.add_many(elements).unwrap();
        gst::Element::link_many(elements).unwrap();

        setup_bus_watch(&pipeline);

        Self {
            pipeline,
            auto_stop_thread: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn record(&mut self, duration: Duration) {
        set_pipeline_state(&self.pipeline, gst::State::Playing);

        self.stop_flag.store(false, Ordering::Relaxed);
        let pipeline_ref = self.pipeline.downgrade();
        let end_time = Instant::now() + duration;
        let stop_flag = Arc::clone(&self.stop_flag);
        let handle = thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) && Instant::now() < end_time {
                thread::sleep(Duration::from_millis(100));
            }
            if !stop_flag.load(Ordering::Relaxed) {
                let pipeline = pipeline_ref.upgrade().unwrap();
                set_pipeline_state(&pipeline, gst::State::Null);
                debug!("recording auto stopped");
            }
        });

        self.auto_stop_thread = Some(handle);
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.auto_stop_thread.take() {
            self.stop_flag.store(true, Ordering::Relaxed);
            handle.join().unwrap();
            self.auto_stop_thread = None;
        }
        set_pipeline_state(&self.pipeline, gst::State::Null);
    }
}
