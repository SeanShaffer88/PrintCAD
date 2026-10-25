# Decisions — PrintCAD Lite

## 2026-10-24 — v1 grouping is mesh-combining; true CSG is planned

**Decision.** Objects flagged `is_hole` are stored, listed and drawn
as holes, but v1 exports every object — holes included — as a
separate solid in one combined STL. No boolean subtraction,
union or intersection is performed.

**Why.** Correct, robust CSG on arbitrary triangle meshes is the
hardest problem in this product class (degenerate intersections,
non-manifold results, coplanar faces). Shipping a subtraction that
silently produces broken meshes would be worse than shipping none:
a slicer would reject or misprint the part while the user trusted
the preview.

**Consequences.** The README, the CLI help, the info output, the GUI
inspector and the stats bar all say, in text, that holes are NOT
subtracted in v1. The flag exists now so scenes designed today keep
their intent when CSG lands, and so grouping (solid parts vs hole
parts) is visible in the object list. Claiming boolean behaviour
before it is implemented *and tested* is explicitly forbidden for
this project.

## 2026-10-24 — No separate scale field; size parameters are the scale

**Decision.** An object has shape parameters (box size, radius,
height), position and rotation — and nothing else. There is no
independent scale vector.

**Why.** It keeps the model, the inspector and the volume maths
one-to-one: what you type is the printed size, and the analytic
volume of the primitive is the volume of the placed object
(rotation and translation never change volume).

## 2026-10-24 — Millimetres everywhere

All stored, displayed and exported values are millimetres; angles
are degrees. STL is unitless by specification, and PrintCAD Lite
writes millimetre coordinates, which is what slicers assume by
default. This is stated in the UI, the CLI help and the docs.

## 2026-10-24 — Position is the object's centre

`position` is the centre of the primitive (for a cylinder, the
middle of its height). New objects are placed with their base on
the ground plane (z = centre z − height/2 = 0), so the common case
"standing on the print bed" is the default without making the
stored value a special case.

## 2026-10-24 — Renderer: eframe with glow, explicitly

The GUI uses eframe 0.33 with the **glow** renderer feature pinned
(not wgpu), following the house pattern from sibling projects: a
wgpu build with no backend feature panicked on real Windows
machines. AccessKit and default fonts are enabled; x11 is enabled
on non-Windows hosts for development builds.

## 2026-10-24 — Library-first architecture

All geometry, mesh generation, STL writing/parsing, projection
maths, validation and file IO live in the library and are tested
off-Windows. The GUI renders state and collects input only. The
binary dispatches to the CLI when arguments are present, and to
the GUI otherwise.

## 2026-10-24 — Format version 1, atomic saves

Scene files (`.printcad.json`) carry `version: 1`. Files from a
newer format are refused with a clear message rather than misread.
Saves write a temporary file in the same folder and replace the
target, so an interrupted save should not corrupt an existing
scene.
