//! Page zoom of the main window, persisted as the `zoom` setting.
//!
//! The level moves through [`STEPS`] like a browser's zoom does. Tauri has
//! no getter for the zoom, so the setting is the source of truth: every
//! change is applied with `WebviewWindow::set_zoom` and saved in one step.
//! A zoom the host sets stays across navigations on every platform.

use tauri::{Manager, Runtime, WebviewWindow, Wry};

use crate::settings::SettingsStore;

/// The zoom levels, as factors (`1.0` is 100 %). Chromium's steps between
/// 50 % and 200 %.
pub const STEPS: &[f64] = &[0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0];

/// The zoom of a fresh install and of a reset.
pub const DEFAULT: f64 = 1.0;

/// The level `zoom` is shown at: within the range of [`STEPS`].
pub fn clamp(zoom: f64) -> f64 {
    zoom.clamp(STEPS[0], STEPS[STEPS.len() - 1])
}

/// The first step above `zoom`, or the largest step.
pub fn step_in(zoom: f64) -> f64 {
    STEPS
        .iter()
        .copied()
        .find(|&step| step > zoom + f64::EPSILON)
        .unwrap_or(STEPS[STEPS.len() - 1])
}

/// The first step below `zoom`, or the smallest step.
pub fn step_out(zoom: f64) -> f64 {
    STEPS
        .iter()
        .rev()
        .copied()
        .find(|&step| step < zoom - f64::EPSILON)
        .unwrap_or(STEPS[0])
}

/// Applies the saved zoom to `window`. Call it before the window is shown.
pub fn apply_saved<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let zoom = window.state::<SettingsStore<Wry>>().get().zoom;
    window.set_zoom(clamp(zoom))
}

/// Saves the zoom `change` makes from the current level and applies it.
pub fn change<R: Runtime>(window: &WebviewWindow<R>, change: impl FnOnce(f64) -> f64) {
    // Inside the update, so two quick changes (Windows runs actions on a thread
    // pool) can't read the same old level or apply out of order.
    let saved = window.state::<SettingsStore<Wry>>().update(|settings| {
        settings.zoom = clamp(change(clamp(settings.zoom)));
        if let Err(err) = window.set_zoom(settings.zoom) {
            eprintln!("failed to zoom the page: {err}");
        }
    });
    if let Err(err) = saved {
        eprintln!("failed to save the zoom level: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_sorted_and_include_the_default() {
        assert!(STEPS.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(STEPS.contains(&DEFAULT));
    }

    #[test]
    fn steps_move_one_level() {
        assert_eq!(step_in(1.0), 1.1);
        assert_eq!(step_in(0.9), 1.0);
        assert_eq!(step_out(1.0), 0.9);
        assert_eq!(step_out(1.1), 1.0);
    }

    #[test]
    fn steps_stop_at_the_ends() {
        assert_eq!(step_in(2.0), 2.0);
        assert_eq!(step_out(0.5), 0.5);
    }

    #[test]
    fn a_level_between_steps_moves_to_the_next_step() {
        assert_eq!(step_in(1.2), 1.25);
        assert_eq!(step_out(1.2), 1.1);
    }

    #[test]
    fn saved_levels_outside_the_range_are_clamped() {
        assert_eq!(clamp(5.0), 2.0);
        assert_eq!(clamp(0.1), 0.5);
        assert_eq!(clamp(1.25), 1.25);
    }
}
