//! Orthographic (isometric-style) projection for the GUI viewport.
//! Pure maths, following the approach of the RoomForge 3D view: orbit
//! by azimuth/elevation, project to 2D, depth-sort painter's order.
//! Units are millimetres; `scale` is pixels per millimetre.

use crate::mesh::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Orbit angle around the vertical (Z) axis, degrees.
    pub azimuth_deg: f64,
    /// Tilt above the ground plane, degrees (0 = side view,
    /// 90 = straight down).
    pub elevation_deg: f64,
    /// Pixels per millimetre.
    pub scale: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            azimuth_deg: 45.0,
            elevation_deg: 35.0,
            scale: 2.0,
        }
    }
}

/// Project a world point to screen space relative to `origin` (the
/// screen point for the world origin). Returns (sx, sy, depth):
/// larger depth is *nearer* the viewer, so the painter draws in
/// ascending depth order (far first).
pub fn project(p: Vec3, cam: &Camera, origin: (f64, f64)) -> (f64, f64, f64) {
    let az = cam.azimuth_deg.to_radians();
    let el = cam.elevation_deg.to_radians();
    // Azimuth: rotate around Z.
    let (rx, ry) = (
        p[0] * az.cos() - p[1] * az.sin(),
        p[0] * az.sin() + p[1] * az.cos(),
    );
    // Elevation: the vertical screen axis mixes height and depth.
    let vert = p[2] * el.cos() + ry * el.sin();
    let depth = ry * el.cos() - p[2] * el.sin();
    (
        origin.0 + rx * cam.scale,
        origin.1 - vert * cam.scale,
        depth,
    )
}

/// Shade factor 0.35..=1.0 from a world normal and a fixed light
/// direction (from the upper front-left), so faces read in 3D without
/// any lighting code in the GUI beyond calling this.
pub fn shade_factor(normal: Vec3) -> f64 {
    let light = crate::mesh::normalize([-0.4, -0.55, 0.73]);
    let d = crate::mesh::dot(normal, light).abs();
    0.35 + 0.65 * d
}

/// Apply a shade factor to an RGB colour.
pub fn shade(rgb: [u8; 3], factor: f64) -> [u8; 3] {
    rgb.map(|c| ((c as f64 * factor).clamp(0.0, 255.0)) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_projects_to_origin() {
        let cam = Camera::default();
        let (sx, sy, _) = project([0.0, 0.0, 0.0], &cam, (400.0, 300.0));
        assert_eq!((sx, sy), (400.0, 300.0));
    }

    #[test]
    fn higher_points_project_higher_on_screen() {
        let cam = Camera::default();
        let (_, low, _) = project([0.0, 0.0, 0.0], &cam, (400.0, 300.0));
        let (_, high, _) = project([0.0, 0.0, 10.0], &cam, (400.0, 300.0));
        assert!(high < low);
    }

    #[test]
    fn azimuth_90_swaps_x_into_depth() {
        let cam = Camera {
            azimuth_deg: 90.0,
            elevation_deg: 0.0,
            scale: 1.0,
        };
        // At elevation 0 the screen shows x' horizontal and z up;
        // after a 90° azimuth the world +x axis points away, so a
        // point on +x stays horizontally centred but gains depth.
        let (sx, _sy, depth) = project([10.0, 0.0, 0.0], &cam, (0.0, 0.0));
        assert!(sx.abs() < 1e-9, "sx={sx}");
        assert!((depth - 10.0).abs() < 1e-9);
    }

    #[test]
    fn shade_factor_range_and_top_face_brightest() {
        let top = shade_factor([0.0, 0.0, 1.0]);
        let side = shade_factor([1.0, 0.0, 0.0]);
        assert!((0.35..=1.0).contains(&top));
        assert!(top > side, "top {top} should beat side {side}");
        assert_eq!(shade([100, 200, 50], 0.5), [50, 100, 25]);
    }
}
