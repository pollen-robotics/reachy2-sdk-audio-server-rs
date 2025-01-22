# Reachy 2 SDK Audio Server

## Testing

Records a sound during 4 sconds and replays it. Tests existence of sound file.

```bash
cargo test --test record_replay -- --nocapture
```

Basic test of grpc features
```bash
cargo test --test grpc
```

Test grpc and gstreamer features. The test is actually a client that sends a request to the server.

```bash
cargo test --test grpc_gst -- --nocapture
```
