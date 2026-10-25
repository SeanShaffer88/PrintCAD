//! STL export (ASCII and binary) plus small parsers used to verify
//! exports in tests. Facet normals are always recomputed from the
//! vertices, never copied from the mesh, so the file is consistent
//! even if a mesh normal were wrong.

use crate::mesh::{cross, normalize, scene_triangles, sub, Triangle};
use crate::model::Scene;

fn facet_normal(t: &Triangle) -> [f64; 3] {
    normalize(cross(
        sub(t.verts[1], t.verts[0]),
        sub(t.verts[2], t.verts[0]),
    ))
}

fn fmt(v: f64) -> String {
    // Avoid "-0.000000" noise and keep round-trippable precision.
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.6}")
}

/// ASCII STL for the combined scene (all objects, transformed;
/// holes included as separate solids — no CSG in v1).
pub fn write_ascii(scene: &Scene) -> String {
    let tris = scene_triangles(scene);
    write_ascii_tris(&scene.name, &tris)
}

pub fn write_ascii_tris(name: &str, tris: &[Triangle]) -> String {
    let mut out = String::with_capacity(tris.len() * 220);
    out.push_str(&format!("solid {}\n", name.replace(['\n', '\r'], " ")));
    for t in tris {
        let n = facet_normal(t);
        out.push_str(&format!(
            "  facet normal {} {} {}\n    outer loop\n",
            fmt(n[0]),
            fmt(n[1]),
            fmt(n[2])
        ));
        for v in t.verts {
            out.push_str(&format!(
                "      vertex {} {} {}\n",
                fmt(v[0]),
                fmt(v[1]),
                fmt(v[2])
            ));
        }
        out.push_str("    endloop\n  endfacet\n");
    }
    out.push_str(&format!("endsolid {}\n", name.replace(['\n', '\r'], " ")));
    out
}

/// Binary STL for the combined scene: 80-byte header, little-endian
/// triangle count, then 50 bytes per facet.
pub fn write_binary(scene: &Scene) -> Vec<u8> {
    let tris = scene_triangles(scene);
    write_binary_tris(&tris)
}

pub fn write_binary_tris(tris: &[Triangle]) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + tris.len() * 50);
    let mut header = [0u8; 80];
    let label = b"PrintCAD Lite binary STL (units: millimetres)";
    header[..label.len()].copy_from_slice(label);
    out.extend_from_slice(&header);
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    for t in tris {
        let n = facet_normal(t);
        for v in n.iter().chain(t.verts.iter().flatten()) {
            out.extend_from_slice(&(*v as f32).to_le_bytes());
        }
        out.extend_from_slice(&0u16.to_le_bytes()); // attribute byte count
    }
    out
}

/// Count facets in an ASCII STL document.
pub fn parse_ascii_triangle_count(text: &str) -> Result<usize, String> {
    if !text.trim_start().starts_with("solid") {
        return Err("missing 'solid' header".to_string());
    }
    let facets = text.matches("facet normal").count();
    let ends = text.matches("endfacet").count();
    if facets != ends {
        return Err(format!("facet count {facets} != endfacet count {ends}"));
    }
    if !text.trim_end().ends_with("endsolid") && !text.contains("\nendsolid") {
        return Err("missing 'endsolid'".to_string());
    }
    Ok(facets)
}

/// Read and fully validate the header/count of a binary STL blob,
/// returning its triangle count.
pub fn parse_binary_triangle_count(data: &[u8]) -> Result<usize, String> {
    if data.len() < 84 {
        return Err(format!("binary STL too short: {} bytes", data.len()));
    }
    let count = u32::from_le_bytes(
        data[80..84]
            .try_into()
            .map_err(|_| "count field unreadable".to_string())?,
    ) as usize;
    let expected = 84 + count * 50;
    if data.len() != expected {
        return Err(format!(
            "binary STL length {} does not match count field {count} (expected {expected})",
            data.len()
        ));
    }
    Ok(count)
}

/// Parse the first facet's normal from a binary STL (test/diagnostic aid).
pub fn parse_binary_first_normal(data: &[u8]) -> Result<[f32; 3], String> {
    if parse_binary_triangle_count(data)? == 0 {
        return Err("no facets".to_string());
    }
    let mut n = [0f32; 3];
    for (i, slot) in n.iter_mut().enumerate() {
        let start = 84 + i * 4;
        *slot = f32::from_le_bytes(
            data[start..start + 4]
                .try_into()
                .map_err(|_| "normal unreadable".to_string())?,
        );
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Object, Scene, Shape};

    fn one_box_scene() -> Scene {
        let mut s = Scene {
            name: "TestBox".into(),
            ..Default::default()
        };
        s.objects.push(Object::new(
            1,
            Shape::Box {
                size: [10.0, 10.0, 10.0],
            },
        ));
        s
    }

    #[test]
    fn ascii_stl_round_trip_count() {
        let scene = Scene::starter();
        let text = write_ascii(&scene);
        let expected = scene_triangles(&scene).len();
        assert_eq!(parse_ascii_triangle_count(&text).unwrap(), expected);
        assert!(text.starts_with("solid Starter"));
        assert!(text.contains("endsolid Starter"));
    }

    #[test]
    fn binary_stl_round_trip_count_and_size() {
        let scene = Scene::starter();
        let data = write_binary(&scene);
        let expected = scene_triangles(&scene).len();
        assert_eq!(parse_binary_triangle_count(&data).unwrap(), expected);
        assert_eq!(data.len(), 84 + expected * 50);
    }

    #[test]
    fn binary_first_normal_is_unit_and_matches_a_box_face() {
        let data = write_binary(&one_box_scene());
        let n = parse_binary_first_normal(&data).unwrap();
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 1e-6);
        // First box face is the bottom: normal (0,0,-1).
        assert_eq!(n, [0.0, 0.0, -1.0]);
    }

    #[test]
    fn ascii_normals_are_unit_length() {
        let text = write_ascii(&one_box_scene());
        for line in text.lines().filter(|l| l.contains("facet normal")) {
            let nums: Vec<f64> = line
                .split_whitespace()
                .skip(2)
                .map(|p| p.parse().unwrap())
                .collect();
            assert_eq!(nums.len(), 3);
            let len = (nums[0] * nums[0] + nums[1] * nums[1] + nums[2] * nums[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal {nums:?} not unit");
        }
    }

    #[test]
    fn empty_scene_exports_zero_facets_both_formats() {
        let scene = Scene::default();
        assert_eq!(parse_ascii_triangle_count(&write_ascii(&scene)).unwrap(), 0);
        assert_eq!(
            parse_binary_triangle_count(&write_binary(&scene)).unwrap(),
            0
        );
    }

    #[test]
    fn parsers_reject_garbage() {
        assert!(parse_ascii_triangle_count("hello").is_err());
        assert!(parse_binary_triangle_count(&[0u8; 10]).is_err());
        let mut data = write_binary(&one_box_scene());
        data.truncate(100);
        assert!(parse_binary_triangle_count(&data).is_err());
    }

    #[test]
    fn hole_object_is_exported_as_separate_solid_triangles() {
        // Honesty check for v1: a hole contributes its triangles to
        // the STL exactly like a solid does (no subtraction).
        let mut s = one_box_scene();
        let mut hole = Object::new(
            2,
            Shape::Cylinder {
                radius: 2.0,
                height: 12.0,
                segments: 8,
            },
        );
        hole.is_hole = true;
        s.objects.push(hole);
        let expected = scene_triangles(&s).len();
        assert_eq!(expected, 12 + 32);
        assert_eq!(
            parse_ascii_triangle_count(&write_ascii(&s)).unwrap(),
            expected
        );
    }
}
