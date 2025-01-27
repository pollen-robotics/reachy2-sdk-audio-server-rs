use gst::glib;
use gst::prelude::*;
use log::{error, info};

pub fn add_element_by_name(name: &str) -> gst::Element {
    let element = gst::ElementFactory::make(name)
        .build()
        .unwrap_or_else(|_| panic!("failed to build {name} element"));
    element
}

pub fn setup_bus_watch(pipeline: &gst::Pipeline) {
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

pub fn set_pipeline_state(pipeline: &gst::Pipeline, state: gst::State) {
    let ret = pipeline.set_state(state);
    match ret {
        Ok(gst::StateChangeSuccess::Success) | Ok(gst::StateChangeSuccess::Async) => {
            // Pipeline state changed successfully
        }
        Ok(gst::StateChangeSuccess::NoPreroll) => {
            error!("Failed to transition pipeline to PLAYING: No preroll data available");
        }
        Err(err) => {
            error!("Failed to transition pipeline to {:?}: {:?}", state, err);
        }
    }
}
