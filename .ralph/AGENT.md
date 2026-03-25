# Ralph Agent Configuration

## Build Instructions

```bash
cargo build
```

## Check Instructions

```bash
cargo check
```

## Test Instructions

```bash
cargo test
```

## Run Instructions

```bash
cargo run -p codeforge-app
```

## Lint Instructions

```bash
cargo clippy -- -D warnings
```

## Notes
- This is a Rust project using iced GUI framework
- Cargo workspace with 3 crates: app, session, persistence
- Reference implementation at /tmp/t3code
- Update this file when build process changes
