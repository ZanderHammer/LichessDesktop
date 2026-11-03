//! Page zoom of the main window, persisted as the `zoom` setting.
//!
//! The level moves through [`STEPS`] like a browser's zoom does. Tauri has
//! no getter for the zoom, so the setting is the source of truth, applied
//! with `WebviewWindow::set_zoom`. A zoom the host sets stays across
//! navigations on every platform.

use tauri::{Manager, Runtime, WebviewWindow, Wry};

use crate::settings::SettingsStore;

/// The zoom levels, as factors (`1.0` is 100 %). Chromium's steps between
/// 50 % and 200 %.
pub const STEPS: &[f64] = &[0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// The level `zoom` is shown at: within the range of [`STEPS`].
pub fn clamp(zoom: f64) -> f64 {
    zoom.clamp(STEPS[0], STEPS[STEPS.len() - 1])
}

/// Applies the saved zoom to `window`. Call it before the window is shown.
pub fn apply_saved<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let zoom = window.state::<SettingsStore<Wry>>().get().zoom;
    window.set_zoom(clamp(zoom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_sorted() {
        assert!(STEPS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn saved_levels_outside_the_range_are_clamped() {
        assert_eq!(clamp(5.0), 2.0);
        assert_eq!(clamp(0.1), 0.5);
        assert_eq!(clamp(1.25), 1.25);
    }
}
