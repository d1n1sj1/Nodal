# Nodal handoff

Date: 2026-11-04. Status: all roadmap items done (BJT, MOSFET, op-amp, AC analysis, SPICE import/export; see Verified and Next steps). v0.1.0 and v0.2.0 tagged and published by the owner. All 14 plan tasks built, reviewed and committed on `main`. A final whole-branch review found one Critical and four Important issues; they were fixed in `10d8f95` and `e7f75ed`, and a scoped re-review passed. The app was then launched in a real window, checked, and further UI bugs were fixed (`6cddc1d`, `1fd52fd`, `4eff4fe`, `41751a3`). Remote: `https://github.com/d1n1sj1/Nodal` (public); `main` and tag `v0.1.0` are pushed.

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
- `v0.1.0` and `v0.2.0` published by the owner.
- Wire routing (`ui/src/editor/route.ts`) avoids parts, and `routeAll` makes later wires avoid running along earlier wires of other nets (2026-11-03). Not checked in the Tauri window.
- Installers are unsigned, so macOS and Windows will warn (SmartScreen: More info, Run anyway).

## Release

- `v0.1.0` tag at `2d62a86`. `release.yml` ran green on all four jobs (Windows, Linux, macOS arm64, macOS x86_64) and made a draft release; CI is green on `main` and the tag. Asset names were not listed (the API hides drafts); check them in the Releases tab.
- Release fixes made before tagging: Intel macOS build, `src-tauri/LICENSE.txt` (MIT + Apache) as `licenseFile`, actions pinned by SHA. The "Education" category passed the build.
- `v0.2.0` tag at `3191a83` (version bumped to 0.2.0 in `tauri.conf.json` and both `Cargo.toml`). The first push at `2db3d4b` failed CI and Release: `tsc -b` (part of `npm run build --prefix ui`) rejects unused variables (TS6133) that `tsc --noEmit` does not flag. Fixed in `route.ts`, tag moved, Release and CI both green. Published by the owner; asset list unchecked (API hides drafts). Always run `npm run build --prefix ui` before pushing.
- 2026-11-03 after v0.2.0: Kids palette now has LED, Transistor (NPN), Amplifier (op-amp) and Wave (signal source, same defaults as the engineer one). Real window (CDP, as above): Kids battery+LED+resistor loop and battery+bulb+NPN+base resistor circuit both simulate over IPC with no errors; the bulb glows. Kids-mode AC: the AC panel now shows in Kids mode as "Pitch sweep" (fixed 10 Hz to 100 kHz, 20 points per decade, source = the Wave tile, else the first voltage source, one "Loudness" % plot, starts on the last node); window-checked on Wave + Motor + Resistor (flat to 1 kHz, 16% at 100 kHz). Kids Capacitor tile added 2026-11-04 (1 uF, yellow; 11 Kids tiles now): window-checked Wave + Resistor + Capacitor "Pitch sweep" is a low-pass (100% at 10 Hz, about 0% by 10 kHz). The motor coil also still gives a pitch-dependent circuit. Amplifier tile window-checked 2026-11-04: Kids battery into the + input, output tied to the - input and to a resistor, gives 5.00 V out (voltage follower), no errors. Not window-checked: Open/Save of the new tiles (OS dialogs), close-prompt Yes/No. `cargo test --workspace`, release speed tests and 115 UI tests pass.
- 2026-11-04: AC sweep no longer capped at 300 unknowns (`068c2e9`): block-real solve through the shared sparse LU (see AC entry above). New tests in `tests/ac.rs`: sparse vs dense agree, 1,000-node ladder runs. CI green on `068c2e9`. Not timed in the real window on a large circuit.

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
- MOSFET (N/P) done: `PartKind::Mosfet` (level-1 square law, bulk at source, no channel-length modulation, symmetric drain/source swap, Newton steps limited to 1 V per junction), drain current reported into the drain, over-current damage, same three-pin UI (0 drain, 1 gate, 2 source), `vth` and `kp` editable. Tests: `crates/nodal-core/tests/mosfet.rs`, `ui/src/editor/mosfet.test.ts`. No gate-capacitance or body diode, so switching circuits look ideal.
- Op-amp done: `PartKind::Opamp` (behavioural, ground referenced, no supply pins: out = vsat*tanh(gain*(plus-minus)/vsat), so the limit is soft, ~1.6 mV low at 10 V of 12 V; inputs draw no current). Output is a branch unknown, current reported into the output pin (negative when sourcing), over-current warning above `max_amps` (a warning, not a clamp). UI: palette item, symbol with inputs left (pin 1 +, pin 2 -) and output right (pin 0), gain and output limit editable. Tests: `crates/nodal-core/tests/opamp.rs`, `ui/src/editor/opamp.test.ts`. Newton for the op-amp is undamped; odd positive-feedback circuits may fail to converge.
- Real-window check of the new parts done 2026-11-02 (Tauri window over WebView2 CDP, no console errors): inverting op-amp 1 V in gives -10.0 V out, 5 V in clips at -12.0 V; NPN common emitter (5 V, 100k, beta 100, 1k load) Ic 4.31 mA; N-MOSFET (Vg 4 V, 1k load) Id 9.74 mA, matches the hand-solved triode value; a 1 mA limit gives the red halo and banner for both; Kids mode with an engineer-placed MOSFET shows the generic tile without errors. Symbols and pin placement look right. Not checked in the window: PNP, P-MOSFET, Open/Save with the new parts (unit-tested only). Seen: the MOSFET label sits close to the source pin.
- Newton and speed follow-up (2026-11-02): op-amp circuits with positive feedback now converge. `newton(.., rails)` first tries every op-amp from deep positive saturation (20 iterations max) on the first solve of a run (DC op, first transient step); a linear start lands on the unstable solution between the rails. Negative feedback does not converge from there and falls back to the normal start. Inside a Schmitt trigger's hysteresis band both rails are stable and the solver picks +. Checked: Schmitt trigger DC states, relaxation oscillator period within 3% of 2RC ln 3 (`tests/opamp.rs`). An op-amp output driving a capacitor node directly is rejected as singular at rest (source across a short), as for any voltage source.
- Speed with active parts (`cargo test --release -p nodal-core --test speed`, 1,000 nodes, 1,000 steps): NPN 0.91 s, N-MOSFET 0.87 s, op-amp 0.93 s, diode 0.90 s. Fixed the same day: the COLAMD column order is now cached (`ColOrder`, `Stamper::factor_cached`) across Newton iterations and transient steps, giving about 0.2 s for each (NPN 0.21, N-MOSFET 0.20-0.26, op-amp 0.21-0.38, diode 0.21-0.26), linear ladder 0.04 s.
- AC analysis done: `nodal_core::ac::ac_sweep(circuit, source_id, f_start, f_stop, points_per_decade)` solves the DC operating point, linearises diodes, LEDs, BJTs, MOSFETs and op-amps around it (`mna::linearize`), then solves a complex system per frequency with the chosen voltage source as a 1 V stimulus (other sources zero, current sources open). Returns magnitude (volts = gain) and phase in degrees per node. No file-format change: the source is chosen per sweep. Complex system solved as a real 2n block system `[Gr -Gi; Gi Gr]` through the shared `Stamper` LU (dense up to 50 block unknowns, faer sparse above, column order cached across frequencies), so no unknown cap (a 1,000-node ladder sweeps in the test suite); 20,000-point cap. Tauri command `run_ac`; UI `panels/AcPanel.tsx` (Engineer mode only: source, from, to, points per decade, node picker) and `panels/Bode.tsx` (gain in dB and phase, log frequency axis). Tests: `crates/nodal-core/tests/ac.rs` (RC corner -3 dB and -45 degrees, RL, inverting op-amp, common-emitter against the hybrid-pi formula, common-source gain gm*RD, diode small-signal resistance, bad input), `ui/src/panels/Bode.test.tsx`, `AcPanel.test.tsx`. Real window: RC low-pass sweep draws 0 dB to -56 dB and 0 to -90 degrees as expected. Not done: no AC amplitude per source (always 1 V), no input/output impedance, no sparse complex solve, no Kids-mode AC.
- SPICE import/export done (2026-11-03): see Next steps item 4 for format, losses and tests. Not checked in the real window; PNP, P-MOSFET and Open/Save of new parts are only unit-tested.
- Real-window checks 2026-11-03 (Tauri window over WebView2 CDP, driven by a small Node CDP script): PNP high-side switch (5 V, 220k base, 1k load) Ic -1.97 mA, base 4.33 V, matches hand value; P-MOSFET (gate grounded, 1k load) Id -4.92 mA vs hand-solved 4.91 mA; no console errors. Save/Load IPC (`save_circuit`/`load_circuit`) round-trips PNP and P-MOSFET circuits as `.json` and `.cir` with identical simulated currents. The close prompt "You have unsaved changes. Discard them?" (Yes/No) shows in a real window on WM_CLOSE with unsaved edits. Not checked: the OS Open/Save dialogs through the UI buttons (the webview `invoke` is frozen, so the dialog cannot be stubbed; owner confirmed Open/Save earlier), the No/Yes outcomes of the close prompt, the new wire routing.
- Signal source done (2026-11-03): `VSource` gained optional `wave: Wave` (`Sine`/`Square`, `amp`, `freq`; `volts` is the offset; square is high for the first `duty` of each period, default 0.5, valid 0 to 1 exclusive). Transient only: DC and AC analysis see `volts`. Old files load unchanged (`wave` defaults to none). Core validates finite values and `freq > 0` (`InvalidInput`). UI: palette item `Signal source` (`volts` offset, amplitude, frequency, Sine/Square toggle in the properties panel); `buildCircuit` folds `amp`/`freq`/`square` into `wave`, `schematicFromCircuit` reads them back. SPICE: exports `SIN(offset amp freq)` and `PULSE(lo hi 0 0 0 half period)`; imports `SIN` and `PULSE`, where PULSE becomes an ideal square with duty = width / period (delay and rise/fall dropped). Tests: `tests/transient_linear.rs` (sine, square, bad freq), `tests/spice.rs`, `ui/src/editor/signal.test.ts`. Real window checked: 1 kHz sine into RC (1k, 159 nF) reads -500 mV at 5 ms with a +-0.71 V scope range (-3 dB, -45 degrees); square wave peaks +-0.92 V (tanh(pi/2)). The canvas was squeezed to about 275 px by the panels below; it is now `flex: 1 0 60svh` and the page scrolls. No Kids-mode signal source; no triangle/sawtooth or phase; duty is unit-tested, not window-checked.
- Kids tiles for BJT/MOSFET (Transistor) and op-amp (Amplifier) added for `tileFor`; they are not in the Kids palette, only shown when an engineer-placed part is viewed in Kids mode.

## Deferred minors (not fixed)

- Core: `KINDS` list in `model.rs` is hand-duplicated from `PartKind` (the round-trip test in `tests/model.rs` fails if one is missing); Rust rejects file versions above 1, a missing version is accepted; `pub` test hooks `set_dense_max` and `dc_op_with_limit` leak into the public API;  the oversized-result test only asserts `is_err()`.
- App: commands accept any path (CSP now set, dialog supplies paths); `saveCircuit` TS test added.
- UI: the Kids bulb glows after a run (final-sample power over its watt rating, `panels/glow.ts`); error text is the same in both modes. Undo/redo (Ctrl+Z, Ctrl+Y, header buttons; edits within 500 ms merge) and an unsaved-changes prompt on Open and window close exist; the close prompt (`onCloseRequested` + `ask`) is not checked in the real window.
- Release (fixed, not yet run): macOS builds arm64 and x86_64 targets; `licenseFile` is `src-tauri/LICENSE.txt` (both licenses); actions pinned by SHA in `ci.yml` and `release.yml`. Category "Education" still unchecked against Tauri's list (a `tauri build` would reject a bad value).
- Risk: the repo lives under OneDrive, which will sync `target/` and `node_modules/`.

## Next steps

1. Run the remaining owner checks above, plus a real-window Open/Save of a `.cir` file.
2. Done: `v0.2.0` published. Remote and release workflow are done.
3. Triage the deferred minors.
4. Roadmap done. SPICE import/export: `crates/nodal-core/src/spice.rs` (`to_spice`, `from_spice`), chosen by a `.cir`/`.sp` extension in `save_circuit`/`load_circuit`. Lossy by design: switches export as 1 mOhm / 1 GOhm resistors, op-amps as a VCVS `E` (vsat resets to 12 V on import), power and current ratings reset to defaults, part ids gain a type letter unless they already start with it (`p1` -> `Rp1`, `V1` stays `V1`; a name clash falls back to prefixing). Reads suffixes (k, meg, u...), `+` continuation, named nodes, `.model` NPN/PNP/NMOS/PMOS/D. Not read: subcircuits, `.param`, AC/SIN/PULSE sources, switches, `;` inline comments. Tests: `tests/spice.rs`, one in `src-tauri/tests/commands.rs`. Not window-checked.
