// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{WebviewUrl, WebviewWindowBuilder};

const START_URL: &str = "https://lichess.org";

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let url = START_URL.parse().expect("START_URL is a valid URL");
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Lichess Desktop")
                .inner_size(1280.0, 800.0)
                .min_inner_size(480.0, 360.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Lichess Desktop");
}
