# Class Sign-in

A simple web-based sign-in system for classes, built with Rust and Axum.

## Features

- Generates a set of unique 8-digit tokens for each session.
- Students sign in by entering their token and student ID.
- Prevents token reuse and enforces basic rate limiting per IP.
- Records sign-ins to a timestamped CSV file.
- Responsive, mobile-friendly HTML forms.

## Usage

### Build and Run

```sh
cargo run --release -- [--tokens N] [--port PORT]
```

- `--tokens N`: Number of tokens to generate (default: 40)
- `--port PORT`: Port to listen on (default: 8888)

Example to listen port 80:

```sh
sudo setcap 'cap_net_bind_service=+ep' target/release/class-sign-in
cargo run --release -- --tokens 100 --port 80
```

### Workflow

1. On startup, the server generates `tokens-<timestamp>.txt` and `records-<timestamp>.txt`.
2. Students visit the root URL and enter their token.
3. If valid and unused, they enter their student ID and confirm.
4. Successful sign-ins are recorded in the records file.

## Output Files

- `tokens-<timestamp>.txt`: List of valid tokens for the session.
- `records-<timestamp>.txt`: CSV log of sign-ins (`token,student_id,timestamp`).

## Dependencies

- [axum](https://crates.io/crates/axum)
- [tokio](https://crates.io/crates/tokio)
- [chrono](https://crates.io/crates/chrono)
- [clap](https://crates.io/crates/clap)
- [serde](https://crates.io/crates/serde)
- [rand](https://crates.io/crates/rand)

## License

MIT (see [LICENSE](LICENSE)
