//! PrintCAD Lite library: scene model, mesh generation, STL export,
//! projection maths and file IO. All of it is testable off-Windows;
//! the GUI in `main.rs` / `app.rs` is a thin shell over this.

pub mod cli;
pub mod io;
pub mod mesh;
pub mod model;
pub mod projection;
pub mod stl;

pub use mesh::{scene_stats, scene_triangles, SceneStats};
pub use model::{Object, Scene, Shape};
