# Nodal: design

## Intent and why

Nodal is an open-source desktop circuit simulator for students and engineers. The owner wants to plan electronics projects and test a circuit before building it, so a wrong design shows up on screen instead of burning a real part.

Existing tools split the field: LTspice is fast but its UI looks dated and clunky; Multisim is friendly to learners but slow. Nodal aims for LTspice-class speed with a modern UI and an approachable learning experience.

## Goals

- Fast, accurate simulation. Speed is a headline feature.
- A modern, clean UI (rounded panels, sans-serif type, one accent color, soft shadows). Nothing that looks like a dated engineering tool.
- One app with a mode switch. The same circuit and the same save file render in two looks:
  - **Engineer mode:** dark theme, standard schematic symbols, parts list with symbol icon and name, scope panel and numeric readouts (voltage, current, time).
  - **Kids mode:** bright white and sky blue with coral and mint accents (no yellow backgrounds), colorful picture tiles per part (battery, bulb, switch, buzzer, motor, resistor), friendly hint messages.
- Both modes show the Nodal logo (three linked nodes in a rounded indigo-to-blue square) and the name "Nodal". Kids mode adds a small "Kids" tag. Every part, in both modes, shows an icon and a name.
- Damage warnings: when a part would be damaged (for example an LED with no series resistor, or a resistor over its power rating), show a warning and highlight the part in red. In kids mode the wording is friendly.
- Open source.

## Non-goals

- No cloud, accounts or collaboration.
- No PCB layout.
- No mobile app.
- No full SPICE model-library parity in the first version.

## Anti-goals

- A slow solver or UI.
- An engineer mode that looks robotic or dated.
- A kids mode that is a separate app or a toy with a fake engine. It must use the same engine as engineer mode.

## Constraints

- Desktop app first.
- Both modes read and write the same file format.

## Decisions made by the owner

- One app with a mode switch, not two apps.
- Warn and highlight the part in red for damage, in both modes.
- Rust for the solver core: confirmed.
- First version leaves transistors, op-amps and the rest for later: confirmed.
- Visual direction: the v4 mockups (kept in `.superpowers/brainstorm/1776-1793472491/content/two-modes-v4.html`).

## Builder's calls (made by me, not yet confirmed by the owner)

- **Stack:** Rust solver core, Tauri desktop shell, React + TypeScript UI with a canvas schematic editor. Chosen for speed and small installers; the Rust core can later compile to WASM for a web version.
- **Licence:** MIT or Apache-2.0.
- **Kids mode** uses a smaller, simpler parts set at first (no transistors or op-amps).
- **First version scope:** DC operating point and transient analysis; resistor, capacitor, inductor, voltage/current sources, ground, diode, switch; schematic editor; scope plot; both modes; damage warnings. Later: BJT/MOSFET, op-amp, AC analysis, SPICE netlist import/export, bigger part library.

## More builder's calls (after review, mine, not yet confirmed by the owner)

- **Performance target:** a circuit of about 1,000 nodes runs a typical transient in under 1 second on a normal laptop. Diodes and LEDs need Newton iteration with convergence handling, so v1 includes nonlinear solving. Use a sparse solver once circuits grow past small sizes.
- **Damage detection:** computed from simulation results (power or current above a part's rating), shown after each run. The LED is a v1 part (a diode with a forward-current limit), so "LED with no resistor" is caught by the current check, not by a topology rule.
- **Kids tiles are skins over real v1 models:** battery = voltage source, bulb = resistor with a power rating (glows with power), switch = switch, buzzer = resistive load, motor = resistor and inductor in series, resistor = resistor.
- **Mockups:** `two-modes-v4.html` is a visual reference for look, layout and part palette, not a pixel spec.
- **Mode is a per-user view setting**, not stored in the circuit file. A part with no kids tile shows a generic tile in kids mode.
- **Platforms:** Windows, macOS and Linux, with standard installers. No auto-update in v1.

## Left to the builder

- Solver details (modified nodal analysis, integration method, convergence handling).
- File format specifics, project layout, testing approach.
- Exact component ratings used for damage detection, and the default values.
- Exact icon artwork and the final color values.
