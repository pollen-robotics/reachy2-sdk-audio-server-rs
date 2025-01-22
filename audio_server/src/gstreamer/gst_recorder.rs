use gst::prelude::*;
use log::{error, info};

pub struct GstRecorder {
    pipeline: gst::Pipeline,
}

impl GstRecorder {
    pub fn new(path: &str) -> Self {
        let pipeline = gst::Pipeline::new();

        let autoaudiosrc = GstRecorder::add_element_by_name("autoaudiosrc");
        let queue = GstRecorder::add_element_by_name("queue");
        let audioconvert = GstRecorder::add_element_by_name("audioconvert");
        let audioresample = GstRecorder::add_element_by_name("audioresample");
        let opusenc = GstRecorder::add_element_by_name("opusenc");
        let oggmux = GstRecorder::add_element_by_name("oggmux");
        let filesink = GstRecorder::add_element_by_name("filesink");
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

        GstRecorder::setup_bus_watch(&pipeline);

        Self { pipeline }
    }

    fn add_element_by_name(name: &str) -> gst::Element {
        let element = gst::ElementFactory::make(name)
            .build()
            .expect(format!("failed to build {name} element").as_str());

        element
    }

    fn setup_bus_watch(pipeline: &gst::Pipeline) {
        let bus = pipeline.bus().unwrap();
        let _bus_watch = bus
            .add_watch(move |_bus, message| {
                use gst::MessageView;
                match message.view() {
                    MessageView::Error(err) => {
                        error!(
                            "Error received from element {:?} {}",
                            err.src().map(|s| s.path_string()),
                            err.error()
                        );
                        error!("Debugging information: {:?}", err.debug());
                        glib::ControlFlow::Break
                    }
                    MessageView::Eos(..) => {
                        info!("Reached end of stream");
                        glib::ControlFlow::Break
                    }
                    _ => glib::ControlFlow::Continue,
                }
            })
            .unwrap();
    }

    pub fn record(&mut self) {
        let ret = self.pipeline.set_state(gst::State::Playing);
        match ret {
            Ok(gst::StateChangeSuccess::Success) | Ok(gst::StateChangeSuccess::Async) => {
                // Pipeline state changed successfully
            }
            Ok(gst::StateChangeSuccess::NoPreroll) => {
                error!("Failed to transition pipeline to PLAYING: No preroll data available");
            }
            Err(err) => {
                error!("Failed to transition pipeline to PLAYING: {:?}", err);
            }
        }
    }

    pub fn stop(&mut self) {
        self.pipeline.set_state(gst::State::Null).unwrap();
    }
}
