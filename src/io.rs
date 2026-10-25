//! Scene file IO: versioned JSON, written atomically (write a
//! temporary file in the same folder, then replace).

use crate::model::{Scene, FORMAT_VERSION};
use std::path::{Path, PathBuf};

pub fn load_scene(path: &Path) -> Result<Scene, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let scene: Scene = serde_json::from_str(&text)
        .map_err(|e| format!("{} is not a valid scene file: {e}", path.display()))?;
    if scene.version > FORMAT_VERSION {
        return Err(format!(
            "{} was saved by a newer version of PrintCAD Lite (format {} > {})",
            path.display(),
            scene.version,
            FORMAT_VERSION
        ));
    }
    let problems = scene.validate();
    if !problems.is_empty() {
        return Err(format!("invalid scene: {}", problems.join("; ")));
    }
    Ok(scene)
}

pub fn save_scene(scene: &Scene, path: &Path) -> Result<(), String> {
    let text =
        serde_json::to_string_pretty(scene).map_err(|e| format!("cannot serialise scene: {e}"))?;
    write_atomic(path, text.as_bytes())
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp: PathBuf = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    // On Windows, rename over an existing file fails; remove first.
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("cannot replace {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Scene;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "printcad-test-{}-{}.printcad.json",
            std::process::id(),
            tag
        ))
    }

    #[test]
    fn save_load_round_trip_via_disk() {
        let path = temp_path("roundtrip");
        let scene = Scene::starter();
        save_scene(&scene, &path).unwrap();
        let back = load_scene(&path).unwrap();
        assert_eq!(scene, back);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_rejects_newer_version_and_garbage() {
        let path = temp_path("garbage");
        std::fs::write(&path, "not json").unwrap();
        assert!(load_scene(&path).is_err());
        let mut scene = Scene::starter();
        scene.version = FORMAT_VERSION + 9;
        std::fs::write(&path, serde_json::to_string(&scene).unwrap()).unwrap();
        let err = load_scene(&path).unwrap_err();
        assert!(err.contains("newer version"), "{err}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let path = temp_path("atomic");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert!(!path.with_extension("tmp").exists());
        let _ = std::fs::remove_file(&path);
    }
}
