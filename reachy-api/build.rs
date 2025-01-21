fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../deps/reachy2-sdk-api/protos/component.proto").unwrap();
    tonic_build::compile_protos("../deps/reachy2-sdk-api/protos/audio.proto").unwrap();
    Ok(())
}
