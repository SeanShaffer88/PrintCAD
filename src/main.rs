// Release builds are a desktop app: no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;
use std::io::Write;

fn log_path() -> std::path::PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.is_empty() {
            return std::path::PathBuf::from(local)
                .join("PrintCADLite")
                .join("printcad.log");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join(".printcadlite")
            .join("printcad.log");
    }
    std::path::PathBuf::from("printcad.log")
}

fn diag_log(line: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{line}");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // CLI when arguments are present, GUI otherwise.
    if !args.is_empty() {
        std::process::exit(printcad::cli::run(&args));
    }

    std::panic::set_hook(Box::new(|info| {
        diag_log(&format!("PANIC: {info}"));
    }));
    diag_log(&format!(
        "launch: PrintCAD Lite v{}",
        env!("CARGO_PKG_VERSION")
    ));

    let options = eframe::NativeOptions {
        // Glow (OpenGL), pinned explicitly — see DECISIONS.md.
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("PrintCAD Lite")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            "PrintCAD Lite",
            options,
            Box::new(|_cc| Ok(Box::new(app::App::default()) as Box<dyn eframe::App>)),
        )
    }));
    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            diag_log(&format!("fatal: {e}"));
            std::process::exit(1);
        }
        Err(_) => {
            diag_log("fatal: panic during startup; see the PANIC line above");
            std::process::exit(1);
        }
    }
}
