# AGENTS.md — PrintCAD Lite

Notes for agents working on this project. The Universal Windows
Desktop Master Prompt applies in full:
`~/workspace/your_files/windows-desktop-master-prompt.md`

## Project in one line

A Windows desktop program in Rust for 3D-printing hobbyists: place
simple primitives (box, cylinder, sphere, cone) in millimetres,
arrange them, and export an ASCII or binary STL a slicer accepts.
MIT. Offline, no telemetry.

## Hard rules

- **Never claim boolean subtraction works.** v1 grouping is
  mesh-combining; `is_hole` is grouping metadata only and holes are
  exported as separate solids. True CSG is Planned (see
  DECISIONS.md). Only change this wording when CSG is genuinely
  implemented *and* covered by tests.
- **Millimetres everywhere** — model, UI, CLI, docs and STL output.
  Angles are degrees.
- **No separate scale field.** Size parameters are the scale.
- Public documentation (README, CHANGELOG, guides) must not mention
  or compare competitor software.
- **Never claim Windows runtime is verified** until the build has
  actually been run on SEAN's Windows machine. Passing local tests
  or CI is not Windows verification. Label claims Verified /
  Partially Verified / Unverified.

## Architecture

- All geometry/mesh/STL/projection/validation/IO logic lives in the
  library (`src/model.rs`, `mesh.rs`, `stl.rs`, `projection.rs`,
  `io.rs`, `cli.rs`) and is testable off-Windows. The GUI
  (`src/app.rs`) renders state and collects intent only.
- Single crate: `[lib]` + `[[bin]]`. The binary dispatches to the
  CLI when arguments are present, otherwise starts the GUI.
- Scene files are versioned JSON (`.printcad.json`), written
  atomically. Format changes bump the version and update tests in
  the same change.
- Renderer: egui via eframe with the **glow** feature explicitly
  selected, plus `accesskit` and `default_fonts` (x11 on
  non-Windows). Do not change the renderer casually.

## Design tokens (black/red family look)

- Background: `#000000` / `#101010`
- Hairlines / borders: `#242424`
- Primary red: `#6B0700`
- Accent: `#E5484D`
- Text: `#F5F5F5` (main), `#A8A8A8` (muted), `#6E6E6E` (faint)

Status must always be paired with a text cue — never colour alone
(holes are labelled "(hole)" in text, not just drawn darker).

## The gate (run in full before publishing anything)

```sh
cargo fmt --all
cargo clippy --locked --all-targets -- -D warnings
cargo clippy --locked --target x86_64-pc-windows-msvc --all-targets -- -D warnings
cargo test --locked
cargo check --target x86_64-pc-windows-msvc --all-targets
```

Run the **full** test suite, never a filtered subset. CI mirrors
the essentials on `windows-latest` (fmt check, clippy, full tests,
release build).

## Documentation

- `README.md` opens with `## TL;DR` and a screenshots section that
  points at `docs/screenshots/CAPTURE.md`. Screenshots come only
  from real runs on SEAN's machine — never placeholders.
- Record significant decisions in `DECISIONS.md` and user-facing
  changes in `CHANGELOG.md`.

## License

MIT — see `LICENSE`. Copyright 2026 Sean Shaffer.
