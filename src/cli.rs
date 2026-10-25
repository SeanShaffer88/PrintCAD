//! Command-line interface. Pure dispatch over the library so the
//! binary stays thin: `printcad <command> ...` runs a command and
//! exits; no arguments start the GUI (handled in main.rs).

use crate::io::{load_scene, save_scene, write_atomic};
use crate::mesh::scene_stats;
use crate::model::{default_shape, Object, Scene};
use crate::stl;
use std::path::Path;

pub const USAGE: &str = "\
PrintCAD Lite — all units are millimetres.

Usage:
  printcad new <out.printcad.json>            Create a starter scene (box + cylinder)
  printcad info <scene.printcad.json>         Objects, triangles, bounding box, est. volume
  printcad export <scene> <out.stl> [--binary]  Export the combined scene as STL
  printcad add <scene> <box|cylinder|sphere|cone> [key=value ...]
                                              Add a shape in place. Keys: x, y, z,
                                              size (box, \"x,y,z\"), radius, height,
                                              name, hole=true
  printcad help                               This text
  printcad                                    Start the desktop program (GUI)

Note: objects flagged as holes are grouping metadata in v1 — they are
exported as separate solids. True boolean subtraction is planned.";

pub fn run(args: &[String]) -> i32 {
    match args.first().map(|s| s.as_str()) {
        Some("help") | Some("--help") | Some("-h") => {
            println!("{USAGE}");
            0
        }
        Some("new") => cmd_new(args),
        Some("info") => cmd_info(args),
        Some("export") => cmd_export(args),
        Some("add") => cmd_add(args),
        Some(other) => {
            eprintln!("unknown command: {other}\n\n{USAGE}");
            2
        }
        None => {
            eprintln!("{USAGE}");
            2
        }
    }
}

fn need(args: &[String], i: usize, what: &str) -> Result<String, String> {
    args.get(i)
        .cloned()
        .ok_or_else(|| format!("missing {what}\n\n{USAGE}"))
}

fn cmd_new(args: &[String]) -> i32 {
    let out = match need(args, 1, "output path") {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    match save_scene(&Scene::starter(), Path::new(&out)) {
        Ok(()) => {
            println!("created starter scene: {out}");
            0
        }
        Err(e) => fail(&e),
    }
}

fn cmd_info(args: &[String]) -> i32 {
    let path = match need(args, 1, "scene path") {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    let scene = match load_scene(Path::new(&path)) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let stats = scene_stats(&scene);
    println!("Scene: {} ({path})", scene.name);
    println!("Objects: {}", stats.object_count);
    for o in &scene.objects {
        println!(
            "  #{} {} — {}{} at ({:.1}, {:.1}, {:.1}) mm, {:.1} mm3",
            o.id,
            o.name,
            o.shape.label(),
            if o.is_hole {
                " (hole, metadata only)"
            } else {
                ""
            },
            o.position[0],
            o.position[1],
            o.position[2],
            o.shape.volume_mm3()
        );
    }
    println!("Triangles: {}", stats.triangle_count);
    match stats.bbox {
        Some((min, max)) => println!(
            "Bounding box (mm): X {:.1}..{:.1}  Y {:.1}..{:.1}  Z {:.1}..{:.1}  (size {:.1} x {:.1} x {:.1})",
            min[0],
            max[0],
            min[1],
            max[1],
            min[2],
            max[2],
            max[0] - min[0],
            max[1] - min[1],
            max[2] - min[2]
        ),
        None => println!("Bounding box: (empty scene)"),
    }
    println!(
        "Estimated volume: {:.1} mm3 total ({:.1} solid, {:.1} flagged as holes — holes are NOT subtracted in v1)",
        stats.volume_mm3, stats.solid_volume_mm3, stats.hole_volume_mm3
    );
    0
}

fn cmd_export(args: &[String]) -> i32 {
    let scene_path = match need(args, 1, "scene path") {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    let out = match need(args, 2, "STL output path") {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    let binary = args.iter().any(|a| a == "--binary");
    let scene = match load_scene(Path::new(&scene_path)) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let result = if binary {
        write_atomic(Path::new(&out), &stl::write_binary(&scene))
    } else {
        write_atomic(Path::new(&out), stl::write_ascii(&scene).as_bytes())
    };
    match result {
        Ok(()) => {
            println!(
                "exported {} triangles to {out} ({})",
                scene_stats(&scene).triangle_count,
                if binary { "binary STL" } else { "ASCII STL" }
            );
            0
        }
        Err(e) => fail(&e),
    }
}

fn cmd_add(args: &[String]) -> i32 {
    let path = match need(args, 1, "scene path") {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    let kind = match need(args, 2, "shape kind") {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    let mut shape = match default_shape(&kind) {
        Some(s) => s,
        None => {
            return fail(&format!(
                "unknown shape '{kind}' (box|cylinder|sphere|cone)"
            ))
        }
    };
    let mut scene = match load_scene(Path::new(&path)) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let mut obj = Object::new(scene.next_id(), shape.clone());
    // Apply key=value overrides.
    for pair in &args[3..] {
        let Some((key, value)) = pair.split_once('=') else {
            return fail(&format!("expected key=value, got '{pair}'"));
        };
        let num: Result<f64, _> = value.parse();
        let num_is_err = num.is_err();
        match key {
            "x" => obj.position[0] = num.unwrap_or(obj.position[0]),
            "y" => obj.position[1] = num.unwrap_or(obj.position[1]),
            "z" => obj.position[2] = num.unwrap_or(obj.position[2]),
            "name" => obj.name = value.to_string(),
            "hole" => obj.is_hole = value == "true",
            "size" => {
                let parts: Vec<f64> = value.split(',').filter_map(|p| p.parse().ok()).collect();
                if parts.len() == 3 {
                    if let crate::model::Shape::Box { size } = &mut shape {
                        *size = [parts[0], parts[1], parts[2]];
                    }
                }
            }
            "radius" => match &mut shape {
                crate::model::Shape::Cylinder { radius, .. }
                | crate::model::Shape::Cone { radius, .. }
                | crate::model::Shape::Sphere { radius, .. } => {
                    if let Ok(v) = num {
                        *radius = v;
                    }
                }
                _ => {}
            },
            "height" => match &mut shape {
                crate::model::Shape::Cylinder { height, .. }
                | crate::model::Shape::Cone { height, .. } => {
                    if let Ok(v) = num {
                        *height = v;
                    }
                }
                _ => {}
            },
            other => return fail(&format!("unknown key '{other}'")),
        }
        if matches!(key, "x" | "y" | "z" | "radius" | "height") && num_is_err {
            return fail(&format!("key '{key}' needs a number, got '{value}'"));
        }
    }
    obj.shape = shape;
    let problems = {
        let mut s = scene.clone();
        s.objects.push(obj.clone());
        s.validate()
    };
    if !problems.is_empty() {
        return fail(&problems.join("; "));
    }
    let label = format!("{} #{}", obj.shape.label(), obj.id);
    scene.objects.push(obj);
    match save_scene(&scene, Path::new(&path)) {
        Ok(()) => {
            println!("added {label} to {path}");
            0
        }
        Err(e) => fail(&e),
    }
}

fn fail(msg: &str) -> i32 {
    eprintln!("error: {msg}");
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("printcad-cli-{}-{tag}", std::process::id()))
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn cli_new_info_export_add_flow() {
        let scene = temp("flow.printcad.json");
        let stl_path = temp("flow.stl");
        let scene_s = scene.to_string_lossy().to_string();
        assert_eq!(run(&strings(&["new", &scene_s])), 0);
        assert_eq!(run(&strings(&["info", &scene_s])), 0);
        assert_eq!(
            run(&strings(&["add", &scene_s, "sphere", "x=50", "radius=5"])),
            0
        );
        let loaded = load_scene(&scene).unwrap();
        assert_eq!(loaded.objects.len(), 3);
        assert_eq!(
            run(&strings(&["export", &scene_s, stl_path.to_str().unwrap()])),
            0
        );
        let text = std::fs::read_to_string(&stl_path).unwrap();
        assert_eq!(
            stl::parse_ascii_triangle_count(&text).unwrap(),
            scene_stats(&loaded).triangle_count
        );
        let bin_path = temp("flow-bin.stl");
        assert_eq!(
            run(&strings(&[
                "export",
                &scene_s,
                bin_path.to_str().unwrap(),
                "--binary"
            ])),
            0
        );
        let data = std::fs::read(&bin_path).unwrap();
        assert_eq!(
            stl::parse_binary_triangle_count(&data).unwrap(),
            scene_stats(&loaded).triangle_count
        );
        for p in [scene, stl_path, bin_path] {
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn cli_rejects_unknown_command_and_shape() {
        assert_eq!(run(&strings(&["frobnicate"])), 2);
        let scene = temp("bad.printcad.json");
        assert_eq!(run(&strings(&["new", scene.to_str().unwrap()])), 0);
        assert_eq!(
            run(&strings(&["add", scene.to_str().unwrap(), "pyramid"])),
            1
        );
        let _ = std::fs::remove_file(scene);
    }
}
