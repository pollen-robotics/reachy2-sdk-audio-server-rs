pub mod error {
    tonic::include_proto!("error");
}

pub mod component {
    tonic::include_proto!("component");
    pub mod audio {
        tonic::include_proto!("component.audio");
    }
}
