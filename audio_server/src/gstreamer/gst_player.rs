// based on https://gitlab.freedesktop.org/gstreamer/gstreamer-rs/-/blob/main/examples/src/bin/decodebin.rs?ref_type=heads

use crate::gst_utils::set_pipeline_state;
use crate::gst_utils::setup_bus_watch;
use gst::{element_warning, prelude::*};
use log::error;

pub struct GstPlayer {
    pipeline: gst::Pipeline,
}

impl GstPlayer {
    pub fn new(path: &str) -> Self {
        let pipeline = gst::Pipeline::new();

        let filesrc = gst::ElementFactory::make("filesrc")
            .property("location", path)
            .build()
            .expect("failed to create filesrc element");

        let decodebin = gst::ElementFactory::make("decodebin")
            .build()
            .expect("failed to create decodebin element");

        let elements = &[&filesrc, &decodebin];
        pipeline.add_many(elements).unwrap();
        gst::Element::link_many(elements).unwrap();

        let pipeline_weak = pipeline.downgrade();

        decodebin.connect_pad_added(move |dbin, src_pad| {
            let Some(pipeline) = pipeline_weak.upgrade() else {
                return;
            };

            let (is_audio, is_video) = {
                let media_type = src_pad.current_caps().and_then(|caps| {
                    caps.structure(0).map(|s| {
                        let name = s.name();
                        (name.starts_with("audio/"), name.starts_with("video/"))
                    })
                });

                match media_type {
                    None => {
                        element_warning!(
                            dbin,
                            gst::CoreError::Negotiation,
                            ("Failed to get media type from pad {}", src_pad.name())
                        );

                        return;
                    }
                    Some(media_type) => media_type,
                }
            };

            if is_audio {
                let queue = gst::ElementFactory::make("queue")
                    .build()
                    .expect("failed to create queue element");
                let convert = gst::ElementFactory::make("audioconvert")
                    .build()
                    .expect("failed to create audioconvert element");
                let resample = gst::ElementFactory::make("audioresample")
                    .build()
                    .expect("failed to create audioresample element");
                let sink = gst::ElementFactory::make("autoaudiosink")
                    .build()
                    .expect("failed to create audioautosink element");

                let elements = &[&queue, &convert, &resample, &sink];
                pipeline.add_many(elements).unwrap();
                gst::Element::link_many(elements).unwrap();

                for e in elements {
                    e.sync_state_with_parent().unwrap();
                }

                let sink_pad = queue.static_pad("sink").expect("queue has no sinkpad");
                src_pad.link(&sink_pad).unwrap();
            } else if is_video {
                error!("Video stream detected. This player only supports audio streams.");
            }
        });

        setup_bus_watch(&pipeline);

        Self { pipeline }
    }

    pub fn play(&mut self) {
        set_pipeline_state(&self.pipeline, gst::State::Playing);
    }

    pub fn stop(&mut self) {
        set_pipeline_state(&self.pipeline, gst::State::Null);
    }
}
