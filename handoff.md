# Nodal handoff

Date: 2026-10-31. Status: design and build plan approved by the owner. No product code written yet.

## What Nodal is

An open-source desktop circuit simulator for students and engineers, for planning projects and testing circuits before building them. It aims for LTspice-class speed with a modern UI that is friendly to learners (the owner likes LTspice's speed, dislikes its dated UI, and finds Multisim friendly but slow).

One app, two looks selected by a mode switch: Engineer mode (dark, schematic symbols, scope, readouts) and Kids mode (bright white and sky blue, colorful picture tiles, friendly wording). Both show the Nodal logo and name, and every part shows an icon and a name. A part that would be damaged gets a warning and a red highlight.

## Documents

- Design: `docs/superpowers/specs/2026-10-31-nodal-design.md`
- Build plan: `docs/superpowers/plans/2026-10-31-nodal-v1.md` (14 tasks, test-first)
- Visual reference mockups: `.superpowers/brainstorm/1776-1793472491/content/two-modes-v4.html` (the owner approved this look; earlier versions v1 to v3 are in the same folder)

## Decisions

Confirmed by the owner:
- One app with a mode switch, not two apps.
- Rust for the solver core.
- First version leaves transistors and op-amps for later.
- Damage handling: warning plus red highlight.
- Look: the v4 mockups.
- The owner approved the whole design and plan.

My calls, approved along with the document but not individually discussed: Tauri shell, React + TypeScript UI, MIT OR Apache-2.0 licence, 1,000-node transient under 1 second, damage computed from simulation results, kids tiles as skins over real parts, mode not saved in the circuit file, Windows/macOS/Linux installers with no auto-update.

## Next step

Execute the plan. The plan recommends subagent-driven development (superpowers:subagent-driven-development) because the tasks share interfaces. The owner has not yet chosen an execution method, so ask first: subagent-driven, or native (one session implements everything, then one reviewer checks the whole branch).

## Housekeeping

- `C:\Users\ernie\OneDrive\Desktop\Nodal` is not a git repo yet. Task 1 of the plan runs `git init`. Add `.superpowers/` to `.gitignore`.
- The browser mockup server has been stopped. The mockup HTML files are still on disk and open directly in a browser.
- The kids-mode wording and tile artwork are first drafts; the owner should review them during the manual checks in Tasks 10 and 13.
- The speed target is unmeasured. Task 6 reports the real time; do not weaken the assertion if it is missed.
