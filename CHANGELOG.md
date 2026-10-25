# Changelog — PrintCAD Lite

All notable changes are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-10-24

First slice.

### Added

- Scene model (format version 1, `.printcad.json`): objects with a
  box, cylinder, sphere or cone shape, centre position in
  millimetres, rotation in degrees, a palette colour, and an
  `is_hole` flag.
- Mesh generation for all four primitives with outward facet
  normals, rotation/translation, scene bounding box, triangle
  counts, and analytic volume estimates per primitive.
- STL export of the combined scene in **ASCII and binary**, with
  facet normals recomputed from the vertices; STL parsers included
  so exports are verified by round-trip tests.
- CLI: `new` (starter scene: box + cylinder), `info`, `export`
  (with `--binary`), `add` (with `key=value` overrides) and `help`.
  Running with no arguments starts the GUI.
- GUI: orthographic 3D viewport with orbit-by-drag, ground grid,
  depth-sorted shaded faces; object list with add / duplicate /
  delete; inspector for name, position, rotation, shape parameters,
  colour and the hole flag; live statistics bar (objects, triangles,
  bounding box, estimated solid volume, hole volume reported
  separately); Save/Open of scene files; ASCII and binary STL
  export buttons. Black/red theme; status is never carried by
  colour alone.
- Documentation: README (TL;DR first), DECISIONS.md (including the
  CSG honesty decision), screenshot capture list, CI workflow.

### Known limitations (honest list)

- **Holes do not cut in v1.** The `is_hole` flag is grouping
  metadata: holes are exported as separate solids. True boolean
  (CSG) subtraction is planned — see DECISIONS.md.
- Overlapping solids are not fused into a single watertight mesh.
- No installer or code signing yet; Windows GUI runtime is
  Unverified until run on SEAN's machine.
