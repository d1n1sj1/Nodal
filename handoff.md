# Nodal handoff

Date: 2026-11-02. Status: v0.1.0 tagged and released as a draft. All 14 plan tasks built, reviewed and committed on `main`. A final whole-branch review found one Critical and four Important issues; they were fixed in `10d8f95` and `e7f75ed`, and a scoped re-review passed. The app was then launched in a real window, checked, and further UI bugs were fixed (`6cddc1d`, `1fd52fd`, `4eff4fe`, `41751a3`). Remote: `https://github.com/d1n1sj1/Nodal` (public); `main` and tag `v0.1.0` are pushed.

## What Nodal is

An open-source desktop circuit simulator for students and engineers, for planning projects and testing circuits before building them. It aims for LTspice-class speed with a modern UI that is friendly to learners.

One app, two looks selected by a mode switch: Engineer mode (dark, schematic symbols, scope, readouts) and Kids mode (bright white and sky blue, colorful picture tiles, friendly wording). Both show the Nodal logo and name, and every part shows an icon and a name. A part that would be damaged gets a warning and a red highlight.

## Layout

- `crates/nodal-core`: Rust solver (model, MNA stamper with dense and faer sparse LU, DC op, trapezoidal transient, diode/LED Newton, damage detection).
- `src-tauri`: Tauri v2 shell (`nodal-app`), thin commands `run_simulation`, `save_circuit`, `load_circuit`, dialog plugin, CSP set.
- `ui`: React + TypeScript (Vite, Vitest). Mode store, themes, parts registry and palette, SVG schematic editor, properties panel, run bar, scope, readouts, warning banner.
- Docs: design `docs/superpowers/specs/2026-10-31-nodal-design.md`, plan `docs/superpowers/plans/2026-10-31-nodal-v1.md`, visual reference `.superpowers/brainstorm/1776-1793472491/content/two-modes-v4.html`.
- Packaging: `.github/workflows/ci.yml`, `release.yml` (tags `v*`, draft release, unsigned), icons generated from `src-tauri/icons/source.svg`, root `package.json` holds the Tauri CLI.

## Verified

- `cargo test --workspace` and `npm test --prefix ui` (90 tests) pass; `npm run build --prefix ui` passes.
- Speed: `cargo test --release -p nodal-core --test speed` passes. The 1,000-node, 1,000-step linear ladder takes about 47 ms; the same ladder with one diode takes about 0.4 s. Target was under 1 s.
- `npm run tauri build` produced an MSI and an NSIS installer in `target/release/bundle/`.

- RC filter (5 V, 1 kΩ, 1 µF) checked through the app's `run_simulation` command against 5(1−e^(−t/τ)): worst error 15 µV over 501 points, no warnings. The UI-driven run of the same wiring used default part values, so the curve itself was not checked on the scope.
- Kids bulb glow checked in the real window (battery + bulb loop: amber halo, opacity 0.55, no console errors); canvas test covers the glow circle.
- Real-window checks passed (driven over WebView2 CDP, `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222" npm run tauri dev`): app loads, simulation runs over IPC under the CSP with no console errors, LED across 5 V shows banner and red halo in Engineer and Kids mode, Open and Save work (confirmed by the owner).

## Fixed after first launch

- Stale results cleared on any schematic edit; a failed run clears the old result.
- Contrast bumps (muted, accent, kids tag); page title `Nodal`.
- Voltage source symbol has + and - marks (pin 1 left is +; wiring the wrong way gave -5 V).
- Kids tile fallback for engineer-placed parts, plus an LED tile.
- Right-angle wires that leave each pin outward (checked in the real window on the Kids battery and bulb loop), label halo.
- Atomic `save_circuit` (temp file then rename); `parseSI` accepts `μ` and `K`; loaded ground id is `GND`.

## Still for the owner

- Owner checks done 2026-11-01 ("everything is fine"): kids tile artwork, wording, contrast colours, close prompt, RC curve on the scope.
- Publish the draft release `v0.1.0` (Releases tab, only visible to the owner) when happy with it.
- Wire routing (`ui/src/editor/route.ts`) avoids parts but not other wires. Checked in a browser against the Vite server (wire dips under a part in its way), not in the Tauri window.
- Installers are unsigned, so macOS and Windows will warn (SmartScreen: More info, Run anyway).

## Release

- `v0.1.0` tag at `2d62a86`. `release.yml` ran green on all four jobs (Windows, Linux, macOS arm64, macOS x86_64) and made a draft release; CI is green on `main` and the tag. Asset names were not listed (the API hides drafts); check them in the Releases tab.
- Release fixes made before tagging: Intel macOS build, `src-tauri/LICENSE.txt` (MIT + Apache) as `licenseFile`, actions pinned by SHA. The "Education" category passed the build.

## Decisions

Confirmed by the owner earlier: one app with a mode switch, Rust solver core, no transistors or op-amps in v1, warning plus red highlight for damage, the v4 mockup look, the whole design and plan.

Builder's calls approved with the plan: Tauri shell, React + TypeScript UI, MIT OR Apache-2.0, 1,000-node transient under 1 second, damage computed from simulation results, kids tiles as skins over real parts, mode not stored in the circuit file, installers for Windows/macOS/Linux with no auto-update.

Rulings made during execution (the owner has not reviewed these; each says what it costs if wrong):
1. Task 5 test chains two runs, but `transient()` had no initial-state input. Added `transient_from` and `TransientResult::final_state()`. Cost: small extra API.
2. Open switch stamps a 1e-12 S leak; the floating-node check treats an open switch as no DC path, so a node reachable only through one is an error. Cost: possible false error in odd circuits.
3. Transient starts from rest (capacitors 0 V, inductors 0 A), not from the DC operating point. The plan said DC, but its own charging tests need rest. Cost: pre-charged circuits need an explicit initial state.
4. Newton convergence residual is `1e-9 + 1e-9*|I|`, not absolute 1e-9 A, so a bare LED across an ideal source gives a finite huge current and can raise a damage warning. The diode curve is linear past 40 thermal voltages. Cost: looser convergence at large currents.
5. Non-finite warning peaks are clamped to `f64::MAX` so JSON stays a number.
6. Editor is SVG, not `<canvas>`.
7. Parts are placed by clicking a palette item then the canvas, not by dragging. Wires can be selected and deleted (unasked). A reloaded kids motor becomes separate resistor and inductor parts, since the file has no skin info. Keyboard operation of parts, pins, placement and delete was added.
8. A schematic with no Ground part grounds the first voltage source's negative pin (else the first current source's), in both modes. Without it Kids mode can never simulate. Cost: a forgotten ground in Engineer mode no longer errors when a source exists.
9. Added a properties panel (edit values, open/close switch) on the final reviewer's request; the spec was silent.
10. Tauri scaffold was written by hand; GUI checks were left to the owner.

## Triage 2026-11-01 (uncommitted)

Done: wire routing around parts; `SimError::InvalidInput` for bad values/run settings/initial state (Singular now only means a singular matrix); `transient_from` rejects non-finite state; speed tests lock `set_dense_max`, diode now conducts (asserted), and a release test checks sparse is >3x faster than dense (0.007 s vs 0.178 s).

## Roadmap progress (2026-11-02, uncommitted before this commit)

- BJT (NPN/PNP) done: `PartKind::Bjt` (Ebers-Moll, Is 1e-14 A, reverse beta 1, no Early effect), Newton handles 3-terminal parts, current reported is into the collector (PNP negative), over-current damage on collector current. UI: two palette items, 3-pin symbol (pin 0 collector, 1 base, 2 emitter), beta and max-current editable. Tests: `crates/nodal-core/tests/bjt.rs`, `ui/src/editor/bjt.test.ts`.
- Not done for BJT: Kids tile (falls back to the generic tile), real-window check, speed test with BJTs. Next: MOSFET, op-amps, AC analysis, SPICE import/export.

## Deferred minors (not fixed)

- Core: `KINDS` list in `model.rs` is hand-duplicated from `PartKind` (the round-trip test in `tests/model.rs` fails if one is missing); Rust rejects file versions above 1, a missing version is accepted; `pub` test hooks `set_dense_max` and `dc_op_with_limit` leak into the public API;  Newton rebuilds the sparse ordering every iteration; the oversized-result test only asserts `is_err()`.
- App: commands accept any path (CSP now set, dialog supplies paths); no `saveCircuit` TS test.
- UI: the Kids bulb glows after a run (final-sample power over its watt rating, `panels/glow.ts`); error text is the same in both modes. Undo/redo (Ctrl+Z, Ctrl+Y, header buttons; edits within 500 ms merge) and an unsaved-changes prompt on Open and window close exist; the close prompt (`onCloseRequested` + `ask`) is not checked in the real window.
- Release (fixed, not yet run): macOS builds arm64 and x86_64 targets; `licenseFile` is `src-tauri/LICENSE.txt` (both licenses); actions pinned by SHA in `ci.yml` and `release.yml`. Category "Education" still unchecked against Tauri's list (a `tauri build` would reject a bad value).
- Risk: the repo lives under OneDrive, which will sync `target/` and `node_modules/`.

## Next steps

1. Run the remaining owner checks above.
2. Decide on a git remote and a first `v*` tag to exercise the release workflow.
3. Triage the deferred minors.
4. Later roadmap: transistors, op-amps, AC analysis, SPICE import/export.
