# Nodal

Nodal is an open-source circuit simulator: a Rust simulation core (`nodal-core`), a Tauri v2 desktop shell (`src-tauri`) and a React + TypeScript UI (`ui`). Licensed under MIT OR Apache-2.0.

## Testing

The solver speed test only runs in release builds: `cargo test --release -p nodal-core --test speed`.
