//! PrintCAD Lite GUI: a thin egui shell over the library.
//! Left: object list + add buttons. Right: inspector. Centre: an
//! orthographic 3D viewport painted with egui's painter (triangles
//! depth-sorted, ground grid). All geometry comes from the library.

use eframe::egui;
use printcad::io::{load_scene, save_scene, write_atomic};
use printcad::mesh::{object_triangles, scene_stats};
use printcad::model::{default_shape, Object, Shape, PALETTE};
use printcad::projection::{project, shade, shade_factor, Camera};
use printcad::{stl, Scene};
use std::path::PathBuf;

pub const BG: egui::Color32 = egui::Color32::from_rgb(0, 0, 0);
pub const PANEL: egui::Color32 = egui::Color32::from_rgb(16, 16, 16);
pub const HAIRLINE: egui::Color32 = egui::Color32::from_rgb(36, 36, 36);
pub const RED_DARK: egui::Color32 = egui::Color32::from_rgb(107, 7, 0);
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(229, 72, 77);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(245, 245, 245);
pub const MUTED: egui::Color32 = egui::Color32::from_rgb(168, 168, 168);
pub const FAINT: egui::Color32 = egui::Color32::from_rgb(110, 110, 110);

pub struct App {
    scene: Scene,
    selected: Option<u64>,
    camera: Camera,
    path_text: String,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        App {
            scene: Scene::starter(),
            selected: Some(1),
            camera: Camera::default(),
            path_text: "untitled.printcad.json".to_string(),
            status: "Starter scene loaded. All units are millimetres (mm).".to_string(),
        }
    }
}

impl App {
    pub fn open_path(&mut self, path: PathBuf) {
        match load_scene(&path) {
            Ok(scene) => {
                self.selected = scene.objects.first().map(|o| o.id);
                self.scene = scene;
                self.path_text = path.to_string_lossy().to_string();
                self.status = format!("Opened {}", path.display());
            }
            Err(e) => self.status = format!("Open failed: {e}"),
        }
    }

    fn add_shape(&mut self, kind: &str) {
        if let Some(shape) = default_shape(kind) {
            let id = self.scene.next_id();
            // Offset each new object a little so it isn't hidden
            // inside the previous one.
            let mut obj = Object::new(id, shape);
            let n = self.scene.objects.len() as f64;
            obj.position[0] += n * 6.0;
            obj.position[1] += n * 3.0;
            self.scene.objects.push(obj);
            self.selected = Some(id);
            self.status = format!("Added {} #{id}", kind);
        }
    }

    fn delete_selected(&mut self) {
        if let Some(id) = self.selected {
            self.scene.objects.retain(|o| o.id != id);
            self.selected = self.scene.objects.first().map(|o| o.id);
            self.status = format!("Deleted object #{id}");
        }
    }

    fn duplicate_selected(&mut self) {
        if let Some(id) = self.selected {
            if let Some(src) = self.scene.get(id).cloned() {
                let new_id = self.scene.next_id();
                let mut copy = src;
                copy.id = new_id;
                copy.name = format!("{} copy", copy.name);
                copy.position[0] += 12.0;
                self.scene.objects.push(copy);
                self.selected = Some(new_id);
                self.status = format!("Duplicated as #{new_id}");
            }
        }
    }

    fn save(&mut self) {
        let path = PathBuf::from(self.path_text.trim());
        match save_scene(&self.scene, &path) {
            Ok(()) => self.status = format!("Saved {}", path.display()),
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }

    fn open(&mut self) {
        let path = PathBuf::from(self.path_text.trim());
        self.open_path(path);
    }

    fn export_stl(&mut self, binary: bool) {
        let scene_path = PathBuf::from(self.path_text.trim());
        let stl_path = scene_path.with_extension(if binary { "bin.stl" } else { "stl" });
        let result = if binary {
            write_atomic(&stl_path, &stl::write_binary(&self.scene))
        } else {
            write_atomic(&stl_path, stl::write_ascii(&self.scene).as_bytes())
        };
        self.status = match result {
            Ok(()) => format!(
                "Exported {} triangles to {} ({})",
                scene_stats(&self.scene).triangle_count,
                stl_path.display(),
                if binary { "binary STL" } else { "ASCII STL" }
            ),
            Err(e) => format!("Export failed: {e}"),
        };
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals {
            panel_fill: PANEL,
            window_fill: PANEL,
            extreme_bg_color: BG,
            faint_bg_color: egui::Color32::from_rgb(24, 24, 24),
            override_text_color: Some(TEXT),
            selection: egui::style::Selection {
                bg_fill: RED_DARK,
                stroke: egui::Stroke::new(1.0_f32, TEXT),
            },
            widgets: egui::style::Widgets {
                hovered: egui::style::WidgetVisuals {
                    bg_fill: egui::Color32::from_rgb(40, 40, 40),
                    weak_bg_fill: egui::Color32::from_rgb(32, 32, 32),
                    bg_stroke: egui::Stroke::new(1.0_f32, ACCENT),
                    fg_stroke: egui::Stroke::new(1.5_f32, TEXT),
                    corner_radius: egui::CornerRadius::same(4),
                    expansion: 0.0,
                },
                ..egui::style::Widgets::dark()
            },
            ..egui::Visuals::dark()
        });

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("PrintCAD Lite");
                ui.label(egui::RichText::new("units: millimetres (mm)").color(MUTED));
                ui.separator();
                ui.label("Scene file:");
                ui.text_edit_singleline(&mut self.path_text)
                    .on_hover_text("Path of the .printcad.json scene file");
                if ui.button("Save").clicked() {
                    self.save();
                }
                if ui.button("Open").clicked() {
                    self.open();
                }
                if ui.button("New starter scene").clicked() {
                    self.scene = Scene::starter();
                    self.selected = Some(1);
                    self.status = "New starter scene.".to_string();
                }
                ui.separator();
                if ui.button("Export STL (ASCII)").clicked() {
                    self.export_stl(false);
                }
                if ui.button("Export STL (binary)").clicked() {
                    self.export_stl(true);
                }
            });
        });

        egui::TopBottomPanel::bottom("stats").show(ctx, |ui| {
            let stats = scene_stats(&self.scene);
            let bbox = match stats.bbox {
                Some((min, max)) => format!(
                    "Bounding box: {:.1} x {:.1} x {:.1} mm",
                    max[0] - min[0],
                    max[1] - min[1],
                    max[2] - min[2]
                ),
                None => "Bounding box: —".to_string(),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Objects: {}", stats.object_count));
                ui.label(format!("Triangles: {}", stats.triangle_count));
                ui.label(bbox);
                ui.label(format!(
                    "Est. volume (solid objects): {:.0} mm3",
                    stats.solid_volume_mm3
                ));
                if stats.hole_volume_mm3 > 0.0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "({:.0} mm3 flagged as holes — NOT subtracted in v1)",
                            stats.hole_volume_mm3
                        ))
                        .color(ACCENT),
                    );
                }
            });
            ui.label(egui::RichText::new(&self.status).color(MUTED));
        });

        egui::SidePanel::left("objects")
            .default_width(210.0)
            .show(ctx, |ui| {
                ui.heading("Add shape");
                ui.horizontal_wrapped(|ui| {
                    for kind in ["box", "cylinder", "sphere", "cone"] {
                        if ui.button(kind).clicked() {
                            self.add_shape(kind);
                        }
                    }
                });
                ui.separator();
                ui.heading("Objects");
                let mut clicked = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for o in &self.scene.objects {
                        let label = format!(
                            "#{} {} — {}{}",
                            o.id,
                            o.name,
                            o.shape.label(),
                            if o.is_hole { " (hole)" } else { "" }
                        );
                        let sel = self.selected == Some(o.id);
                        if ui.selectable_label(sel, label).clicked() {
                            clicked = Some(o.id);
                        }
                    }
                });
                if let Some(id) = clicked {
                    self.selected = Some(id);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Duplicate").clicked() {
                        self.duplicate_selected();
                    }
                    if ui.button("Delete").clicked() {
                        self.delete_selected();
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "Tip: drag in the 3D view to orbit. Holes are grouping \
                         metadata only in v1 — they export as separate solids.",
                    )
                    .color(FAINT),
                );
            });

        egui::SidePanel::right("inspector")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                let Some(id) = self.selected else {
                    ui.label(egui::RichText::new("Select an object to edit it.").color(MUTED));
                    return;
                };
                let Some(obj) = self.scene.get_mut(id) else {
                    return;
                };
                ui.label(format!("Object #{id} — {}", obj.shape.label()));
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut obj.name);
                });
                ui.separator();
                ui.label("Position (mm) — centre of the object");
                for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        ui.add(egui::DragValue::new(&mut obj.position[axis]).speed(0.5));
                    });
                }
                ui.label("Rotation (degrees)");
                for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(*label);
                        ui.add(
                            egui::DragValue::new(&mut obj.rotation_deg[axis])
                                .speed(1.0)
                                .range(-360.0..=360.0),
                        );
                    });
                }
                ui.separator();
                match &mut obj.shape {
                    Shape::Box { size } => {
                        ui.label("Box size (mm)");
                        for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(*label);
                                ui.add(
                                    egui::DragValue::new(&mut size[axis])
                                        .speed(0.5)
                                        .range(0.1..=1000.0),
                                );
                            });
                        }
                    }
                    Shape::Cylinder {
                        radius,
                        height,
                        segments,
                    } => {
                        drag(ui, "Radius (mm)", radius);
                        drag(ui, "Height (mm)", height);
                        ui.horizontal(|ui| {
                            ui.label("Segments");
                            let mut s = *segments as i64;
                            if ui
                                .add(egui::DragValue::new(&mut s).range(3..=256))
                                .changed()
                            {
                                *segments = s as u32;
                            }
                        });
                    }
                    Shape::Sphere {
                        radius,
                        segments,
                        rings,
                    } => {
                        drag(ui, "Radius (mm)", radius);
                        ui.horizontal(|ui| {
                            ui.label("Segments");
                            let mut s = *segments as i64;
                            if ui
                                .add(egui::DragValue::new(&mut s).range(3..=256))
                                .changed()
                            {
                                *segments = s as u32;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Rings");
                            let mut s = *rings as i64;
                            if ui
                                .add(egui::DragValue::new(&mut s).range(2..=256))
                                .changed()
                            {
                                *rings = s as u32;
                            }
                        });
                    }
                    Shape::Cone {
                        radius,
                        height,
                        segments,
                    } => {
                        drag(ui, "Radius (mm)", radius);
                        drag(ui, "Height (mm)", height);
                        ui.horizontal(|ui| {
                            ui.label("Segments");
                            let mut s = *segments as i64;
                            if ui
                                .add(egui::DragValue::new(&mut s).range(3..=256))
                                .changed()
                            {
                                *segments = s as u32;
                            }
                        });
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Colour");
                    egui::ComboBox::from_id_salt("colour")
                        .selected_text(PALETTE.get(obj.colour).map(|(n, _)| *n).unwrap_or("Red"))
                        .show_ui(ui, |ui| {
                            for (i, (name, _)) in PALETTE.iter().enumerate() {
                                ui.selectable_value(&mut obj.colour, i, *name);
                            }
                        });
                });
                ui.checkbox(&mut obj.is_hole, "Hole (grouping metadata only in v1)")
                    .on_hover_text(
                        "Stored and shown as a hole, but exported as a separate solid. \
                         True boolean subtraction is planned.",
                    );
                let problems = self.scene.validate();
                if !problems.is_empty() {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "Warning (text, not colour alone): {}",
                            problems.join("; ")
                        ))
                        .color(ACCENT),
                    );
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Orbit:");
                ui.label("Azimuth");
                ui.add(
                    egui::DragValue::new(&mut self.camera.azimuth_deg)
                        .speed(1.0)
                        .range(-180.0..=180.0),
                );
                ui.label("Elevation");
                ui.add(
                    egui::DragValue::new(&mut self.camera.elevation_deg)
                        .speed(1.0)
                        .range(5.0..=85.0),
                );
                ui.label("Zoom (px per mm)");
                ui.add(
                    egui::DragValue::new(&mut self.camera.scale)
                        .speed(0.05)
                        .range(0.1..=20.0),
                );
                if ui.button("Reset view").clicked() {
                    self.camera = Camera::default();
                }
            });
            ui.separator();
            self.viewport(ui);
        });
    }
}

fn drag(ui: &mut egui::Ui, label: &str, value: &mut f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(value).speed(0.5).range(0.1..=1000.0));
    });
}

impl App {
    fn viewport(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::drag());
        if response.dragged() {
            let d = response.drag_delta();
            self.camera.azimuth_deg += d.x as f64 * 0.5;
            self.camera.elevation_deg =
                (self.camera.elevation_deg + d.y as f64 * 0.5).clamp(5.0, 85.0);
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(16, 16, 16));
        let origin = (rect.center().x as f64, rect.center().y as f64 + 40.0);
        let to_pos = |sx: f64, sy: f64| egui::pos2(sx as f32, sy as f32);

        // Ground grid: lines every 10 mm across a 120 mm square at z=0.
        for step in -6i32..=6 {
            let v = step as f64 * 10.0;
            for (a, b) in [
                ([v, -60.0, 0.0], [v, 60.0, 0.0]),
                ([-60.0, v, 0.0], [60.0, v, 0.0]),
            ] {
                let (x1, y1, _) = project(a, &self.camera, origin);
                let (x2, y2, _) = project(b, &self.camera, origin);
                let colour = if step == 0 {
                    HAIRLINE
                } else {
                    egui::Color32::from_rgb(26, 26, 26)
                };
                painter.line_segment(
                    [to_pos(x1, y1), to_pos(x2, y2)],
                    egui::Stroke::new(1.0_f32, colour),
                );
            }
        }

        // Collect faces with depth, painter's order (far first).
        struct Face {
            depth: f64,
            points: [egui::Pos2; 3],
            fill: egui::Color32,
        }
        let mut faces: Vec<Face> = Vec::new();
        for obj in &self.scene.objects {
            let base = obj.rgb();
            for t in object_triangles(obj) {
                let mut points = [egui::pos2(0.0, 0.0); 3];
                let mut depth = 0.0;
                for (i, v) in t.verts.iter().enumerate() {
                    let (sx, sy, d) = project(*v, &self.camera, origin);
                    points[i] = to_pos(sx, sy);
                    depth += d;
                }
                let mut rgb = shade(base, shade_factor(t.normal));
                if obj.is_hole {
                    // Holes read differently by pattern AND label (in
                    // the object list), never by colour alone.
                    rgb = shade(rgb, 0.45);
                }
                faces.push(Face {
                    depth: depth / 3.0,
                    points,
                    fill: egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                });
            }
        }
        // project() returns larger depth = nearer, so draw ascending.
        faces.sort_by(|a, b| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let selected_tris: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        let _ = selected_tris; // selection is shown in the list/inspector in v1
        for f in &faces {
            painter.add(egui::Shape::convex_polygon(
                f.points.to_vec(),
                f.fill,
                egui::Stroke::new(0.5_f32, HAIRLINE),
            ));
        }
        painter.text(
            rect.left_top() + egui::vec2(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            "Ground plane z = 0 mm — drag to orbit",
            egui::FontId::proportional(12.0),
            FAINT,
        );
    }
}
