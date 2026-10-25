//! Mesh generation: turn each primitive into triangles with outward
//! normals, then apply the object's rotation (X, then Y, then Z, in
//! degrees) and translation. Pure functions, tested off-Windows.
//!
//! Winding is counter-clockwise seen from outside each face, and the
//! stored normal is recomputed from the wound vertices, so orientation
//! and normal always agree.

use crate::model::{Object, Scene, Shape};

pub type Vec3 = [f64; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub verts: [Vec3; 3],
    pub normal: Vec3,
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn normalize(v: Vec3) -> Vec3 {
    let len = dot(v, v).sqrt();
    if len < 1e-12 {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

/// A triangle with its normal recomputed from its winding.
pub fn tri(a: Vec3, b: Vec3, c: Vec3) -> Triangle {
    let normal = normalize(cross(sub(b, a), sub(c, a)));
    Triangle {
        verts: [a, b, c],
        normal,
    }
}

/// Rotate a point: around X by `rot[0]`, then Y, then Z (degrees).
pub fn rotate(p: Vec3, rot_deg: [f64; 3]) -> Vec3 {
    let (rx, ry, rz) = (
        rot_deg[0].to_radians(),
        rot_deg[1].to_radians(),
        rot_deg[2].to_radians(),
    );
    // X axis
    let (s, c) = rx.sin_cos();
    let p = [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c];
    // Y axis
    let (s, c) = ry.sin_cos();
    let p = [p[0] * c + p[2] * s, p[1], -p[0] * s + p[2] * c];
    // Z axis
    let (s, c) = rz.sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]]
}

pub fn transform(p: Vec3, obj: &Object) -> Vec3 {
    let r = rotate(p, obj.rotation_deg);
    [
        r[0] + obj.position[0],
        r[1] + obj.position[1],
        r[2] + obj.position[2],
    ]
}

fn push_quad(out: &mut Vec<Triangle>, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
    out.push(tri(a, b, c));
    out.push(tri(a, c, d));
}

/// Local (untransformed) triangles for a shape, centred on the origin
/// with its base at z = -height/2.
pub fn local_triangles(shape: &Shape) -> Vec<Triangle> {
    match *shape {
        Shape::Box { size } => box_triangles(size),
        Shape::Cylinder {
            radius,
            height,
            segments,
        } => cylinder_triangles(radius, height, segments.max(3)),
        Shape::Sphere {
            radius,
            segments,
            rings,
        } => sphere_triangles(radius, segments.max(3), rings.max(2)),
        Shape::Cone {
            radius,
            height,
            segments,
        } => cone_triangles(radius, height, segments.max(3)),
    }
}

fn box_triangles(size: Vec3) -> Vec<Triangle> {
    let (hx, hy, hz) = (size[0] / 2.0, size[1] / 2.0, size[2] / 2.0);
    // Corners: bottom (z=-hz) 0..3, top 4..7, CCW seen from below/above.
    let v: [Vec3; 8] = [
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];
    let mut out = Vec::with_capacity(12);
    // Bottom (outward = -z): wind clockwise seen from above.
    out.push(tri(v[0], v[2], v[1]));
    out.push(tri(v[0], v[3], v[2]));
    // Top (+z)
    out.push(tri(v[4], v[5], v[6]));
    out.push(tri(v[4], v[6], v[7]));
    // -y face
    push_quad(&mut out, v[0], v[1], v[5], v[4]);
    // +x face
    push_quad(&mut out, v[1], v[2], v[6], v[5]);
    // +y face
    push_quad(&mut out, v[2], v[3], v[7], v[6]);
    // -x face
    push_quad(&mut out, v[3], v[0], v[4], v[7]);
    out
}

fn ring_point(radius: f64, z: f64, i: u32, segments: u32) -> Vec3 {
    let a = (i % segments) as f64 / segments as f64 * std::f64::consts::TAU;
    let (s, c) = a.sin_cos();
    [radius * c, radius * s, z]
}

fn cylinder_triangles(radius: f64, height: f64, segments: u32) -> Vec<Triangle> {
    let hz = height / 2.0;
    let mut out = Vec::with_capacity((segments * 4) as usize);
    for i in 0..segments {
        let b0 = ring_point(radius, -hz, i, segments);
        let b1 = ring_point(radius, -hz, i + 1, segments);
        let t0 = ring_point(radius, hz, i, segments);
        let t1 = ring_point(radius, hz, i + 1, segments);
        // Side, outward.
        push_quad(&mut out, b0, b1, t1, t0);
        // Top cap fan (+z).
        out.push(tri([0.0, 0.0, hz], t0, t1));
        // Bottom cap fan (-z).
        out.push(tri([0.0, 0.0, -hz], b1, b0));
    }
    out
}

fn sphere_triangles(radius: f64, segments: u32, rings: u32) -> Vec<Triangle> {
    // Rows of vertices from the top pole (row 0) to the bottom pole
    // (row = rings). Interior rows are rings at polar angle
    // pi * row / rings.
    let point = |row: u32, i: u32| -> Vec3 {
        let theta = std::f64::consts::PI * row as f64 / rings as f64;
        let phi = (i % segments) as f64 / segments as f64 * std::f64::consts::TAU;
        let (st, ct) = theta.sin_cos();
        let (sp, cp) = phi.sin_cos();
        [radius * st * cp, radius * st * sp, radius * ct]
    };
    let mut out = Vec::new();
    for i in 0..segments {
        // Top fan (rows run top to bottom, so the winding that is
        // counter-clockwise seen from outside is i -> i+1 here).
        out.push(tri(point(0, 0), point(1, i), point(1, i + 1)));
        // Bottom fan.
        out.push(tri(
            point(rings, 0),
            point(rings - 1, i + 1),
            point(rings - 1, i),
        ));
    }
    for row in 1..rings - 1 {
        for i in 0..segments {
            let a = point(row, i);
            let b = point(row, i + 1);
            let c = point(row + 1, i + 1);
            let d = point(row + 1, i);
            // Rows descend, so traverse a -> d -> c -> b for an
            // outward normal (verified by the normals test).
            push_quad(&mut out, a, d, c, b);
        }
    }
    out
}

fn cone_triangles(radius: f64, height: f64, segments: u32) -> Vec<Triangle> {
    let hz = height / 2.0;
    let apex: Vec3 = [0.0, 0.0, hz];
    let mut out = Vec::with_capacity((segments * 2) as usize);
    for i in 0..segments {
        let b0 = ring_point(radius, -hz, i, segments);
        let b1 = ring_point(radius, -hz, i + 1, segments);
        // Side, outward.
        out.push(tri(b0, b1, apex));
        // Base fan (-z).
        out.push(tri([0.0, 0.0, -hz], b1, b0));
    }
    out
}

/// World-space triangles for one object (rotation + translation).
pub fn object_triangles(obj: &Object) -> Vec<Triangle> {
    local_triangles(&obj.shape)
        .into_iter()
        .map(|t| {
            let verts = t.verts.map(|p| transform(p, obj));
            tri(verts[0], verts[1], verts[2])
        })
        .collect()
}

/// World-space triangles for the whole scene, in object order.
/// Holes are included as separate solids — v1 has no CSG; see
/// DECISIONS.md.
pub fn scene_triangles(scene: &Scene) -> Vec<Triangle> {
    scene.objects.iter().flat_map(object_triangles).collect()
}

/// Axis-aligned bounding box: (min, max), or None for an empty scene.
pub fn bounding_box(tris: &[Triangle]) -> Option<(Vec3, Vec3)> {
    let mut iter = tris.iter().flat_map(|t| t.verts.iter());
    let first = *iter.next()?;
    let (mut min, mut max) = (first, first);
    for p in iter {
        for axis in 0..3 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    Some((min, max))
}

/// Scene statistics for the stats panel and the `info` CLI command.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneStats {
    pub object_count: usize,
    pub triangle_count: usize,
    pub bbox: Option<(Vec3, Vec3)>,
    /// Sum of analytic volumes of all objects, mm³.
    pub volume_mm3: f64,
    /// Volume of objects not flagged as holes, mm³.
    pub solid_volume_mm3: f64,
    /// Volume of objects flagged as holes, mm³. In v1 a hole is
    /// grouping metadata only, so this material is NOT subtracted
    /// from `volume_mm3` — it is reported separately for clarity.
    pub hole_volume_mm3: f64,
}

pub fn scene_stats(scene: &Scene) -> SceneStats {
    let tris = scene_triangles(scene);
    let mut solid = 0.0;
    let mut hole = 0.0;
    for o in &scene.objects {
        if o.is_hole {
            hole += o.shape.volume_mm3();
        } else {
            solid += o.shape.volume_mm3();
        }
    }
    SceneStats {
        object_count: scene.objects.len(),
        triangle_count: tris.len(),
        bbox: bounding_box(&tris),
        volume_mm3: solid + hole,
        solid_volume_mm3: solid,
        hole_volume_mm3: hole,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{default_shape, Object, Scene, Shape};

    fn obj(shape: Shape) -> Object {
        Object::new(1, shape)
    }

    fn centroid(t: &Triangle) -> Vec3 {
        [
            (t.verts[0][0] + t.verts[1][0] + t.verts[2][0]) / 3.0,
            (t.verts[0][1] + t.verts[1][1] + t.verts[2][1]) / 3.0,
            (t.verts[0][2] + t.verts[1][2] + t.verts[2][2]) / 3.0,
        ]
    }

    /// Every face normal must point away from the shape's centre.
    fn assert_normals_outward(tris: &[Triangle]) {
        for t in tris {
            let c = centroid(t);
            assert!(
                dot(t.normal, c) > 0.0,
                "normal {:?} should point outward from centre via centroid {c:?}",
                t.normal
            );
            let len = dot(t.normal, t.normal).sqrt();
            assert!((len - 1.0).abs() < 1e-9, "normal not unit: {len}");
        }
    }

    #[test]
    fn box_has_12_triangles_with_outward_normals() {
        let tris = local_triangles(&Shape::Box {
            size: [10.0, 20.0, 30.0],
        });
        assert_eq!(tris.len(), 12);
        assert_normals_outward(&tris);
    }

    #[test]
    fn cylinder_triangle_count_and_normals() {
        let tris = local_triangles(&Shape::Cylinder {
            radius: 5.0,
            height: 10.0,
            segments: 8,
        });
        // sides 2/seg + top fan 1/seg + bottom fan 1/seg
        assert_eq!(tris.len(), 8 * 4);
        assert_normals_outward(&tris);
    }

    #[test]
    fn sphere_triangle_count_and_normals() {
        let (seg, rings) = (8, 4);
        let tris = local_triangles(&Shape::Sphere {
            radius: 5.0,
            segments: seg,
            rings,
        });
        // top fan + bottom fan + (rings-2) quad rows * 2
        assert_eq!(tris.len(), (2 * seg * (rings - 1)) as usize);
        assert_normals_outward(&tris);
        // All vertices lie on the sphere.
        for t in &tris {
            for v in t.verts {
                assert!((dot(v, v).sqrt() - 5.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn cone_triangle_count_and_normals() {
        let tris = local_triangles(&Shape::Cone {
            radius: 5.0,
            height: 10.0,
            segments: 12,
        });
        assert_eq!(tris.len(), 12 * 2);
        assert_normals_outward(&tris);
    }

    #[test]
    fn rotation_z_90_maps_x_to_y() {
        let mut o = obj(default_shape("box").unwrap());
        o.position = [0.0, 0.0, 0.0];
        o.rotation_deg = [0.0, 0.0, 90.0];
        let p = transform([1.0, 0.0, 0.0], &o);
        assert!(p[0].abs() < 1e-9 && (p[1] - 1.0).abs() < 1e-9 && p[2].abs() < 1e-9);
    }

    #[test]
    fn rotation_x_90_maps_y_to_z() {
        let p = rotate([0.0, 1.0, 0.0], [90.0, 0.0, 0.0]);
        assert!(p[0].abs() < 1e-9 && p[1].abs() < 1e-9 && (p[2] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn translation_moves_a_known_vertex() {
        let mut o = obj(Shape::Box {
            size: [10.0, 10.0, 10.0],
        });
        o.position = [100.0, 50.0, 5.0];
        o.rotation_deg = [0.0; 3];
        let tris = object_triangles(&o);
        let (min, max) = bounding_box(&tris).unwrap();
        assert_eq!(min, [95.0, 45.0, 0.0]);
        assert_eq!(max, [105.0, 55.0, 10.0]);
    }

    #[test]
    fn bbox_of_known_scene() {
        let scene = Scene::starter();
        let stats = scene_stats(&scene);
        let (min, max) = stats.bbox.unwrap();
        // Box: x -15..15, cyl at x=28 r=8 -> 20..36 (segment polygon
        // reaches the full radius at angle 0).
        assert!((min[0] - -15.0).abs() < 1e-9);
        assert!((max[0] - 36.0).abs() < 1e-6);
        assert!((min[2] - 0.0).abs() < 1e-9);
        assert!((max[2] - 20.0).abs() < 1e-9);
        assert_eq!(stats.object_count, 2);
        assert_eq!(stats.triangle_count, 12 + 32 * 4);
    }

    #[test]
    fn volume_formulas() {
        assert_eq!(
            Shape::Box {
                size: [2.0, 3.0, 4.0]
            }
            .volume_mm3(),
            24.0
        );
        let cyl = Shape::Cylinder {
            radius: 2.0,
            height: 5.0,
            segments: 8,
        }
        .volume_mm3();
        assert!((cyl - std::f64::consts::PI * 20.0).abs() < 1e-9);
        let sph = Shape::Sphere {
            radius: 3.0,
            segments: 8,
            rings: 4,
        }
        .volume_mm3();
        assert!((sph - 36.0 * std::f64::consts::PI).abs() < 1e-9);
        let cone = Shape::Cone {
            radius: 3.0,
            height: 6.0,
            segments: 8,
        }
        .volume_mm3();
        assert!((cone - 18.0 * std::f64::consts::PI).abs() < 1e-9);
    }

    #[test]
    fn stats_separate_hole_volume_without_subtracting() {
        let mut scene = Scene::default();
        let mut hole = obj(Shape::Box {
            size: [10.0, 10.0, 10.0],
        });
        hole.is_hole = true;
        scene.objects.push(hole);
        let stats = scene_stats(&scene);
        assert_eq!(stats.solid_volume_mm3, 0.0);
        assert_eq!(stats.hole_volume_mm3, 1000.0);
        // v1 honesty: the hole is still exported, so total includes it.
        assert_eq!(stats.volume_mm3, 1000.0);
    }

    #[test]
    fn empty_scene_has_no_bbox() {
        let stats = scene_stats(&Scene::default());
        assert_eq!(stats.triangle_count, 0);
        assert!(stats.bbox.is_none());
    }
}
