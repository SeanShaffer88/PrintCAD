//! Scene data model. All units are millimetres; angles are degrees.
//!
//! A scene is a list of primitive objects. There is deliberately no
//! separate scale field: scaling an object *is* editing its size
//! parameters (box size, radius, height), which keeps the model — and
//! the volume maths — simple. See DECISIONS.md.

use serde::{Deserialize, Serialize};

/// Current scene file format version.
pub const FORMAT_VERSION: u32 = 1;

/// Scene file extension (a JSON file): `name.printcad.json`.
pub const SCENE_EXTENSION: &str = "printcad.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub version: u32,
    pub name: String,
    pub objects: Vec<Object>,
}

impl Default for Scene {
    fn default() -> Self {
        Scene {
            version: FORMAT_VERSION,
            name: "Untitled".to_string(),
            objects: Vec::new(),
        }
    }
}

impl Scene {
    /// A starter scene: one box sitting on the ground plane (z = 0)
    /// and one cylinder standing next to it.
    pub fn starter() -> Scene {
        Scene {
            version: FORMAT_VERSION,
            name: "Starter".to_string(),
            objects: vec![
                Object {
                    id: 1,
                    name: "Box 1".to_string(),
                    shape: Shape::Box {
                        size: [30.0, 20.0, 10.0],
                    },
                    // Centre of the box: z = half its height, so its
                    // base rests on z = 0.
                    position: [0.0, 0.0, 5.0],
                    rotation_deg: [0.0, 0.0, 0.0],
                    colour: 0,
                    is_hole: false,
                },
                Object {
                    id: 2,
                    name: "Cylinder 1".to_string(),
                    shape: Shape::Cylinder {
                        radius: 8.0,
                        height: 20.0,
                        segments: 32,
                    },
                    position: [28.0, 0.0, 10.0],
                    rotation_deg: [0.0, 0.0, 0.0],
                    colour: 1,
                    is_hole: false,
                },
            ],
        }
    }

    /// The next free object id (max existing id + 1, or 1 if empty).
    pub fn next_id(&self) -> u64 {
        self.objects.iter().map(|o| o.id).max().unwrap_or(0) + 1
    }

    pub fn get(&self, id: u64) -> Option<&Object> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Object> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    /// Human-readable problems with the scene, empty when valid.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.version > FORMAT_VERSION {
            problems.push(format!(
                "scene format version {} is newer than this program understands ({})",
                self.version, FORMAT_VERSION
            ));
        }
        let mut ids: Vec<u64> = self.objects.iter().map(|o| o.id).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() != self.objects.len() {
            problems.push("two or more objects share the same id".to_string());
        }
        for o in &self.objects {
            o.shape.validate(&o.name, &mut problems);
            if !o.position.iter().all(|v| v.is_finite()) {
                problems.push(format!("{}: position must be finite", o.name));
            }
            if !o.rotation_deg.iter().all(|v| v.is_finite()) {
                problems.push(format!("{}: rotation must be finite", o.name));
            }
        }
        problems
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Object {
    pub id: u64,
    pub name: String,
    pub shape: Shape,
    /// Centre of the object, millimetres (x right, y back, z up).
    pub position: [f64; 3],
    /// Rotation in degrees, applied around X, then Y, then Z.
    pub rotation_deg: [f64; 3],
    /// Index into [`PALETTE`].
    pub colour: usize,
    /// Grouping metadata only in v1: a hole is stored, listed and
    /// drawn as a hole, but it is exported to STL as a separate solid
    /// like everything else. True boolean subtraction is planned —
    /// see DECISIONS.md. Never describe this flag as cutting material.
    pub is_hole: bool,
}

impl Object {
    pub fn new(id: u64, shape: Shape) -> Object {
        let name = match &shape {
            Shape::Box { .. } => format!("Box {id}"),
            Shape::Cylinder { .. } => format!("Cylinder {id}"),
            Shape::Sphere { .. } => format!("Sphere {id}"),
            Shape::Cone { .. } => format!("Cone {id}"),
        };
        // Default position sits the shape's base on the ground plane.
        let half_height = shape.height_mm() / 2.0;
        Object {
            id,
            name,
            shape,
            position: [0.0, 0.0, half_height],
            rotation_deg: [0.0, 0.0, 0.0],
            colour: (id as usize - 1) % PALETTE.len(),
            is_hole: false,
        }
    }

    /// The palette colour for this object (falls back to the first).
    pub fn rgb(&self) -> [u8; 3] {
        PALETTE
            .get(self.colour)
            .map(|(_, rgb)| *rgb)
            .unwrap_or(PALETTE[0].1)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    Box {
        /// Full extents in millimetres: [x, y, z].
        size: [f64; 3],
    },
    Cylinder {
        radius: f64,
        height: f64,
        segments: u32,
    },
    Sphere {
        radius: f64,
        segments: u32,
        rings: u32,
    },
    Cone {
        radius: f64,
        height: f64,
        segments: u32,
    },
}

impl Shape {
    pub fn label(&self) -> &'static str {
        match self {
            Shape::Box { .. } => "Box",
            Shape::Cylinder { .. } => "Cylinder",
            Shape::Sphere { .. } => "Sphere",
            Shape::Cone { .. } => "Cone",
        }
    }

    /// Overall height along the local Z axis, millimetres.
    pub fn height_mm(&self) -> f64 {
        match *self {
            Shape::Box { size } => size[2],
            Shape::Cylinder { height, .. } => height,
            Shape::Sphere { radius, .. } => radius * 2.0,
            Shape::Cone { height, .. } => height,
        }
    }

    /// Analytic volume of the closed primitive, in cubic millimetres.
    /// Rotation and translation do not change volume, and there is no
    /// separate scale factor, so this is exact for the placed object.
    pub fn volume_mm3(&self) -> f64 {
        match *self {
            Shape::Box { size } => size[0] * size[1] * size[2],
            Shape::Cylinder { radius, height, .. } => {
                std::f64::consts::PI * radius * radius * height
            }
            Shape::Sphere { radius, .. } => {
                4.0 / 3.0 * std::f64::consts::PI * radius * radius * radius
            }
            Shape::Cone { radius, height, .. } => {
                std::f64::consts::PI * radius * radius * height / 3.0
            }
        }
    }

    fn validate(&self, name: &str, problems: &mut Vec<String>) {
        let positive = |label: &str, v: f64, problems: &mut Vec<String>| {
            if !v.is_finite() || v <= 0.0 {
                problems.push(format!("{name}: {label} must be a positive number"));
            }
        };
        match *self {
            Shape::Box { size } => {
                positive("box x size", size[0], problems);
                positive("box y size", size[1], problems);
                positive("box z size", size[2], problems);
            }
            Shape::Cylinder {
                radius,
                height,
                segments,
            } => {
                positive("cylinder radius", radius, problems);
                positive("cylinder height", height, problems);
                if segments < 3 {
                    problems.push(format!("{name}: cylinder needs at least 3 segments"));
                }
            }
            Shape::Sphere {
                radius,
                segments,
                rings,
            } => {
                positive("sphere radius", radius, problems);
                if segments < 3 {
                    problems.push(format!("{name}: sphere needs at least 3 segments"));
                }
                if rings < 2 {
                    problems.push(format!("{name}: sphere needs at least 2 rings"));
                }
            }
            Shape::Cone {
                radius,
                height,
                segments,
            } => {
                positive("cone radius", radius, problems);
                positive("cone height", height, problems);
                if segments < 3 {
                    problems.push(format!("{name}: cone needs at least 3 segments"));
                }
            }
        }
    }
}

/// Default shapes offered by the "add" buttons / CLI.
pub fn default_shape(kind: &str) -> Option<Shape> {
    match kind {
        "box" => Some(Shape::Box {
            size: [20.0, 20.0, 20.0],
        }),
        "cylinder" => Some(Shape::Cylinder {
            radius: 10.0,
            height: 20.0,
            segments: 32,
        }),
        "sphere" => Some(Shape::Sphere {
            radius: 10.0,
            segments: 24,
            rings: 16,
        }),
        "cone" => Some(Shape::Cone {
            radius: 10.0,
            height: 20.0,
            segments: 32,
        }),
        _ => None,
    }
}

/// Display palette: (name, rgb). Status is never carried by colour
/// alone in the UI — holes also get a "(hole)" text label.
pub const PALETTE: &[(&str, [u8; 3])] = &[
    ("Red", [229, 72, 77]),
    ("Grey", [168, 168, 168]),
    ("White", [245, 245, 245]),
    ("Dark red", [107, 7, 0]),
    ("Steel", [110, 130, 150]),
    ("Sand", [200, 170, 110]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_scene_is_valid_and_grounded() {
        let s = Scene::starter();
        assert_eq!(s.objects.len(), 2);
        assert!(s.validate().is_empty(), "{:?}", s.validate());
        for o in &s.objects {
            // Base of each object rests on (or above) the ground.
            assert!(o.position[2] - o.shape.height_mm() / 2.0 >= -1e-9);
        }
    }

    #[test]
    fn next_id_skips_used_ids() {
        let mut s = Scene::default();
        assert_eq!(s.next_id(), 1);
        s.objects
            .push(Object::new(7, default_shape("box").unwrap()));
        assert_eq!(s.next_id(), 8);
    }

    #[test]
    fn validation_catches_bad_values() {
        let mut s = Scene::default();
        s.objects.push(Object {
            id: 1,
            name: "Bad".into(),
            shape: Shape::Cylinder {
                radius: -1.0,
                height: 10.0,
                segments: 2,
            },
            position: [0.0, 0.0, f64::NAN],
            rotation_deg: [0.0; 3],
            colour: 0,
            is_hole: false,
        });
        let problems = s.validate();
        assert!(problems.len() >= 3, "{problems:?}");
    }

    #[test]
    fn duplicate_ids_are_flagged() {
        let mut s = Scene::default();
        s.objects
            .push(Object::new(1, default_shape("box").unwrap()));
        s.objects
            .push(Object::new(1, default_shape("cone").unwrap()));
        assert!(s.validate().iter().any(|p| p.contains("share the same id")));
    }

    #[test]
    fn newer_format_version_is_flagged() {
        let mut s = Scene::starter();
        s.version = FORMAT_VERSION + 1;
        assert!(s.validate().iter().any(|p| p.contains("newer")));
    }

    #[test]
    fn scene_json_round_trip() {
        let s = Scene::starter();
        let text = serde_json::to_string_pretty(&s).unwrap();
        let back: Scene = serde_json::from_str(&text).unwrap();
        assert_eq!(s, back);
        assert!(text.contains("\"kind\": \"cylinder\""));
    }

    #[test]
    fn hole_flag_round_trips() {
        let mut o = Object::new(3, default_shape("sphere").unwrap());
        o.is_hole = true;
        let text = serde_json::to_string(&o).unwrap();
        let back: Object = serde_json::from_str(&text).unwrap();
        assert!(back.is_hole);
        assert_eq!(o, back);
    }
}
