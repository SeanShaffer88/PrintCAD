# PrintCAD Lite

**Arrange simple 3D shapes on your Windows PC and export a file your 3D printer's slicer can open.** Place boxes, cylinders, spheres and cones, size and move them in millimetres, look at the result in 3D, then export an STL. Fully offline, no account, no telemetry.

## TL;DR

PrintCAD Lite is the simplest useful kind of 3D design program. You put basic shapes — a box, a cylinder, a sphere, a cone — next to and on top of each other until they look like the bracket, spacer, knob or enclosure you want to print. Everything is measured in millimetres. When it looks right, you export an **STL file** (ASCII or binary) and open that file in your usual slicer to print it. That is the whole job, done well: no accounts, no cloud, no internet needed. One honest limit, stated plainly: shapes you mark as "holes" are only grouped and labelled as holes in this version — they are exported as separate solids, and true cut-out (boolean) shapes are planned for a later version.

## Contents

- [Key features](#key-features)
- [Screenshots](#screenshots)
- [System requirements](#system-requirements)
- [How to use](#how-to-use)
- [Command line](#command-line)
- [File formats](#file-formats)
- [Building from source](#building-from-source)
- [Tests](#tests)
- [Privacy and your data](#privacy-and-your-data)
- [Limitations and known issues](#limitations-and-known-issues)
- [Roadmap](#roadmap)
- [License](#license)

## Key features

- **Four primitives:** box, cylinder, sphere and cone, each with real millimetre dimensions (box size; radius/height; sphere radius with segments and rings).
- **Arrange freely:** position (the object's centre, in mm) and rotation in degrees around X, Y and Z for every object; duplicate and delete.
- **3D viewport:** orbit by dragging (azimuth/elevation), with a ground grid at z = 0 mm, shaded faces and depth-sorted drawing; holes are drawn darker and are always labelled "(hole)" in text as well.
- **Live statistics:** object count, triangle count, bounding box size and an estimated volume for each object and the scene. Volumes are the exact analytic volumes of the primitives.
- **STL export, both kinds:** ASCII and binary STL of the whole scene combined, with facet normals recomputed from the geometry, ready to open in a slicer.
- **Scene files:** versioned, human-readable JSON (`.printcad.json`), written atomically (a temporary file is swapped into place, so an interrupted save should not corrupt an existing scene).
- **Command line included:** create a starter scene, inspect a scene, add shapes and export STL without opening the window (see below).
- **Colour palette** per object for telling parts apart on screen. Colours are for the screen only — STL carries no colour.

## Screenshots

Screenshots come only from real runs on SEAN's Windows machine — never placeholders. The capture list is in [docs/screenshots/CAPTURE.md](docs/screenshots/CAPTURE.md), and captures will be wired in here as soon as they are supplied.

## System requirements

| | |
| --- | --- |
| Operating system | Windows 10 or Windows 11 (64-bit) — designed and CI-tested for Windows; also compiles on Linux for development |
| Display | 1280 × 820 window by default (minimum 900 × 600); the 3D view uses OpenGL through your normal graphics driver |
| Internet | **Not required** |
| To build | Rust 1.80 or newer |

## How to use

1. **Start.** PrintCAD Lite opens with a starter scene: a box and a cylinder sitting on the ground plane.
2. **Add shapes** with the Box / Cylinder / Sphere / Cone buttons on the left.
3. **Select** an object in the list, then use the **Inspector** on the right: name, position in mm, rotation in degrees, size/radius/height, colour, and the hole flag.
4. **Orbit** by dragging in the 3D view, or type azimuth/elevation/zoom values above it. Watch the bottom bar for triangle count, bounding box and estimated volume.
5. **Save** by typing a scene path (ending in `.printcad.json`) at the top and clicking **Save**; **Open** loads it back.
6. **Export** with **Export STL (ASCII)** or **Export STL (binary)** — the STL is written next to the scene path. Open it in your slicer and print.

## Command line

```sh
printcad new starter.printcad.json
printcad info starter.printcad.json
printcad add starter.printcad.json sphere x=50 radius=5
printcad export starter.printcad.json starter.stl
printcad export starter.printcad.json starter.stl --binary
printcad help
```

`add` accepts `x=`, `y=`, `z=`, `size=x,y,z` (box), `radius=`, `height=`, `name=` and `hole=true`.

## File formats

| Format | What it is |
| --- | --- |
| `.printcad.json` | A scene: versioned JSON listing each object's shape, position, rotation, colour and hole flag. All units are millimetres. |
| `.stl` (ASCII) | Text STL of the combined scene — readable, larger. |
| `.stl` (binary) | Binary STL of the combined scene — compact, the usual slicer input. |

## Building from source

```sh
cargo build --release
cargo run --release            # GUI (no arguments)
cargo test --locked            # full test suite
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --all --check
```

## Project structure

| Path | What lives there |
| --- | --- |
| `src/model.rs` | Scene model: objects, shapes, palette, validation, analytic volumes |
| `src/mesh.rs` | Triangle generation per primitive, rotation/translation, bounding box, scene statistics |
| `src/stl.rs` | ASCII + binary STL writers and verifying parsers |
| `src/projection.rs` | Orthographic orbit projection and face shading maths |
| `src/io.rs` | Atomic save / load of scene JSON with a format-version check |
| `src/cli.rs` | The `new` / `info` / `export` / `add` commands |
| `src/app.rs` | The egui window: object list, inspector, 3D viewport |
| `src/main.rs` | Entry point: CLI dispatch when arguments are present, GUI otherwise |

## Tests

The test suite covers the library: triangle counts and outward normals for every primitive, rotation and translation on known vertices, bounding boxes of known scenes, the analytic volume formulas, ASCII and binary STL round-trips (including the binary header/count field), scene JSON round-trips, file IO, and an end-to-end CLI flow (`new` → `add` → `export` in both STL formats). Run it with `cargo test --locked`.

## Privacy and your data

PrintCAD Lite works **fully offline**: no accounts, no sign-in, no telemetry, no analytics and no network calls. Your scenes and STL files are files on your own computer, stored where you save them. The only file the program writes on its own is an error log at `%LOCALAPPDATA%\PrintCADLite\printcad.log`.

## Limitations and known issues

- **No true boolean operations yet.** Grouping in v1 is mesh-combining: every object is exported as its own solid in one STL. An object flagged as a hole does **not** cut material out of other objects — it is exported as a separate solid exactly like the rest. If you need a real hole in v1, design around the space or cut it in your slicer/other tooling. True CSG subtraction is planned (see Roadmap).
- Overlapping solids are exported as-is; most slicers handle intersecting solids, but PrintCAD Lite does not fuse them into one watertight mesh.
- No text shapes, no rounded/filleted edges, no image tracing, no parametric history, and no direct printing — a slicer is still the last step, as usual.
- Colours, names and the hole flag live in the scene file only; STL does not store them.
- The 3D viewport is a shaded preview for arranging shapes, not a photorealistic render.
- No installer or code signing yet in this slice.

## Roadmap

- **True CSG booleans** (union/subtraction/intersection), so holes really cut.
- Fused, watertight export for overlapping solids.
- More primitives (wedge, torus, tube), rounded edges, and text.
- Dragging objects directly in the 3D viewport, snapping and a measurement tool.
- Importing an STL as a starting object.

## License

MIT — see [LICENSE](LICENSE). Copyright 2026 Sean Shaffer. Built with Rust and the [egui](https://github.com/emilk/egui) immediate-mode GUI library (MIT/Apache-2.0).
