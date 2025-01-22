use crate::gst_utils::add_element_by_name;
use crate::gst_utils::set_pipeline_state;
use crate::gst_utils::setup_bus_watch;
use gst::prelude::*;

pub struct GstRecorder {
    pipeline: gst::Pipeline,
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

        Self { pipeline }
    }

    pub fn record(&mut self) {
        set_pipeline_state(&self.pipeline, gst::State::Playing);
    }

    pub fn stop(&mut self) {
        set_pipeline_state(&self.pipeline, gst::State::Null);
    }
}
