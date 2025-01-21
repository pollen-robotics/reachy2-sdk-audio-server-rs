# Reachy 2 SDK Audio Server

## Testing

Records a sound during 4 sconds and replays it. Tests existence of sound file.

```bash
cargo test --test record_replay -- --nocapture
```

Basic tests of grpc features. The test is actually a client that sends a request to the server.

```bash
cargo test --test grpc -- --nocapture
```
