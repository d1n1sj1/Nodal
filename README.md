# Nodal

Nodal is an open-source circuit simulator: a Rust simulation core (`nodal-core`), a Tauri v2 desktop shell (`src-tauri`) and a React + TypeScript UI (`ui`). Licensed under MIT OR Apache-2.0.

## Testing

The solver speed test only runs in release builds: `cargo test --release -p nodal-core --test speed`.

## Build

Prerequisites: Rust (stable), Node 22+, and on Linux `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`.

```
npm ci
npm ci --prefix ui
npm run tauri dev     # run the app with hot reload
npm run tauri build   # produce installers in target/release/bundle
```

Pushing a `v*` tag runs `.github/workflows/release.yml`, which builds Windows, macOS and Linux installers and attaches them to a draft GitHub Release.

Installers are not code-signed in v1, so Windows SmartScreen and macOS Gatekeeper will show warnings.

## Scope

Nodal has Kids mode and Engineer mode over the same circuit file.

- Parts: resistor, capacitor, inductor, voltage source, current source, ground, diode, LED, switch, NPN/PNP transistor, N/P-MOSFET, op-amp.
- Analyses: DC operating point, transient, AC sweep (Engineer mode).
- Later: SPICE import/export.

## Licence

MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`).
