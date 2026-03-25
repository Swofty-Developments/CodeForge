# Conventions

## Rust Style
- Follow standard Rust conventions (rustfmt defaults)
- Use `thiserror` for error types
- Use `tracing` macros for logging (not println!)
- Prefer `Result<T, E>` over panics
- Use `#[derive(Debug, Clone)]` liberally

## Naming
- Crates: lowercase with hyphens in Cargo.toml, underscores in code
- Modules: snake_case
- Types: PascalCase
- Functions: snake_case
- Constants: SCREAMING_SNAKE_CASE

## Architecture
- iced Messages: `Message::Category(SubMessage)` nesting pattern
- Keep view functions pure — no side effects
- Side effects only in `update()` via `Command`
- Database access through persistence crate's public API only

## Error Handling
- Use `anyhow` at application boundaries
- Use `thiserror` for library-level errors
- Log errors with `tracing::error!` before propagating

## Dependencies
- Minimize dependency count
- Prefer well-maintained, widely-used crates
- Pin major versions in Cargo.toml
