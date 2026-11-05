//! The tray icon, which is always shown: on Linux and Windows there is no
//! menu bar, so it is the only mouse path to app actions.
//!
//! A left click toggles the main window and a right click opens the menu. On
//! Linux the icon is a StatusNotifierItem from tray-icon's ksni backend
//! (ADR 0014). Panels report a left click as `Activate`, which arrives here as a click.
//! The menu repeats the toggle as "Show/Hide" for panels that only show menus.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WindowEvent};

use crate::actions::Action;
use crate::i18n::{self, Key};
use crate::settings::SettingsStore;
use crate::{settings_window, update};

const TRAY_ID: &str = "main";
const TOGGLE: &str = "toggle";
const PALETTE: &str = "palette";
const SETTINGS: &str = "settings";
const SHORTCUTS: &str = "shortcuts";
const QUIT: &str = "quit";
const UPDATE: &str = "update";

/// The badge: `danger` from DESIGN.md.
const BADGE_COLOR: [u8; 3] = [0xc0, 0x1c, 0x28];
/// The badge's center and radius as fractions of the icon's size. It sits in a
/// corner, clear of the pawn, and is still about 8 px wide on a 22 px panel icon.
const BADGE_CENTER: (f32, f32) = (0.81, 0.19);
const BADGE_RADIUS: f32 = 0.18;
/// The radius of the transparent ring that separates the badge from the pawn.
const BADGE_GAP: f32 = 0.23;

/// How long the focus may be gone before the main window hides: while it moves
/// to the palette, no window of the app has it.
const FOCUS_SETTLE: Duration = Duration::from_millis(150);
/// A tray click this soon after the focus went is the click that took it.
const TRAY_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// When the focus last left a window that hides on focus loss. In managed state.
#[derive(Default)]
struct LastFocusLoss(Mutex<Option<Instant>>);

/// Creates the tray icon. The app keeps it alive until exit.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.manage(LastFocusLoss::default());
    let menu = menu(app)?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Lichess Desktop")
        .menu(&menu)
        // Left click toggles the window; the menu stays on right click.
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event);
    // On Linux the title names the item in the panel and heads its tooltip.
    // On macOS it would be drawn as text next to the icon.
    #[cfg(target_os = "linux")]
    {
        tray = tray.title("Lichess Desktop");
    }
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// The tray menu in the current UI language. A known update comes first.
fn menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let strings = app.state::<i18n::Current>().get();
    let item =
        |id: &str, key: Key| MenuItem::with_id(app, id, strings.get(key), true, None::<&str>);
    let menu = Menu::with_items(
        app,
        &[
            &item(TOGGLE, Key::ShowHide)?,
            &item(PALETTE, Action::CommandPalette.label())?,
            &item(SETTINGS, Action::OpenSettings.label())?,
            &item(SHORTCUTS, Action::ShowShortcuts.label())?,
            &PredefinedMenuItem::separator(app)?,
            &item(QUIT, Action::Quit.label())?,
        ],
    )?;
    if let Some(update) = update::available(app) {
        let text = update.text(&strings);
        menu.prepend_items(&[
            &MenuItem::with_id(app, UPDATE, text, true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
        ])?;
    }
    Ok(menu)
}

/// Rebuilds the menu after a language change or a new update.
pub fn refresh_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    match app.tray_by_id(TRAY_ID) {
        Some(tray) => tray.set_menu(Some(menu(app)?)),
        None => Ok(()),
    }
}

/// Shows or removes the badge on the icon that says it's the user's turn.
pub fn set_turn_badge<R: Runtime>(app: &AppHandle<R>, show: bool) -> tauri::Result<()> {
    let (Some(tray), Some(icon)) = (app.tray_by_id(TRAY_ID), app.default_window_icon()) else {
        return Ok(());
    };
    tray.set_icon(Some(if show { with_badge(icon) } else { icon.clone() }))
}

/// `icon` with the badge painted on. Done when needed rather than kept, so an
/// idle app doesn't hold a second copy of the icon.
fn with_badge(icon: &Image<'_>) -> Image<'static> {
    let (width, height) = (icon.width(), icon.height());
    let size = width.min(height) as f32;
    let (center_x, center_y) = (
        BADGE_CENTER.0 * width as f32,
        BADGE_CENTER.1 * height as f32,
    );
    let coverage = |radius: f32, distance: f32| (radius * size - distance + 0.5).clamp(0.0, 1.0);
    let mut rgba = icon.rgba().to_vec();
    for (index, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let x = (index as u32 % width) as f32 + 0.5;
        let y = (index as u32 / width) as f32 + 0.5;
        let distance = (x - center_x).hypot(y - center_y);
        let badge = coverage(BADGE_RADIUS, distance);
        if badge > 0.0 {
            pixel[..3].copy_from_slice(&BADGE_COLOR);
            pixel[3] = (badge * 255.0) as u8;
        } else {
            pixel[3] = (f32::from(pixel[3]) * (1.0 - coverage(BADGE_GAP, distance))) as u8;
        }
    }
    Image::new_owned(rgba, width, height)
}

/// Whether the tray icon exists. Without it a hidden window could only come
/// back through a second launch.
pub fn is_shown<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.tray_by_id(TRAY_ID).is_some()
}

/// Hides the main window when `window` loses focus to something outside the
/// app, if the setting is on. The setting is read on every loss, so a change
/// applies at once.
pub fn hide_on_focus_loss<R: Runtime>(window: &WebviewWindow<R>) {
    let app = window.app_handle().clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event
            && is_shown(&app)
            && app.state::<SettingsStore<R>>().get().hide_on_focus_loss
            && let Some(main) = app.get_webview_window(crate::MAIN_WINDOW)
            // Hiding takes the focus too, and that loss shouldn't swallow a tray click.
            && main.is_visible().unwrap_or(false)
        {
            *lock(&app) = Some(Instant::now());
            let app = app.clone();
            thread::spawn(move || {
                thread::sleep(FOCUS_SETTLE);
                if should_hide(&app) {
                    crate::hide_main_window(&app);
                }
            });
        }
    });
}

/// Whether the focus really left. The open settings window keeps the main window
/// up: it is a child of it, and hiding its parent would leave it stranded.
fn should_hide<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.get_webview_window(settings_window::LABEL).is_none()
        && !app
            .webview_windows()
            .values()
            .any(|window| window.is_focused().unwrap_or(false))
}

fn lock<R: Runtime>(app: &AppHandle<R>) -> std::sync::MutexGuard<'_, Option<Instant>> {
    app.state::<LastFocusLoss>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

fn is_tray_click(lost: Option<Instant>, now: Instant) -> bool {
    lost.is_some_and(|lost| now.duration_since(lost) < TRAY_CLICK_WINDOW)
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id.as_ref() {
        TOGGLE => toggle_main_window(app),
        PALETTE => Action::CommandPalette.run(app),
        SETTINGS => Action::OpenSettings.run(app),
        SHORTCUTS => Action::ShowShortcuts.run(app),
        QUIT => Action::Quit.run(app),
        UPDATE => update::open_release_page(app),
        _ => {}
    }
}

fn on_tray_icon_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        toggle_main_window(tray.app_handle());
    }
}

/// Hides the main window if it is in use, otherwise brings it to the front.
fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(crate::MAIN_WINDOW) else {
        return;
    };
    // Wayland doesn't tell a window that it's minimized, so `is_minimized` stays
    // false there. Only focus tells the cases apart on Linux. Elsewhere a click on
    // the tray can take the focus away first, so focus says nothing there.
    let in_use = if cfg!(target_os = "linux") {
        window.is_focused().unwrap_or(false)
    } else {
        !window.is_minimized().unwrap_or(false)
    };
    // If the click took the focus and so hid the window, it must not bring it back.
    let took_focus = is_tray_click(*lock(app), Instant::now());
    if took_focus || (window.is_visible().unwrap_or(false) && in_use) {
        crate::hide_main_window(app);
    } else {
        crate::raise_main_window(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::WebviewWindowBuilder;
    use tauri::test::mock_builder;

    const SIZE: u32 = 64;
    const PIXEL: [u8; 4] = [10, 20, 30, 255];

    fn pixel(image: &Image<'_>, x: u32, y: u32) -> [u8; 4] {
        let start = ((y * image.width() + x) * 4) as usize;
        image.rgba()[start..start + 4].try_into().unwrap()
    }

    #[test]
    fn only_a_click_right_after_the_focus_went_took_it() {
        let lost = Instant::now();
        assert!(!is_tray_click(None, lost));
        assert!(is_tray_click(Some(lost), lost + Duration::from_millis(100)));
        assert!(!is_tray_click(Some(lost), lost + TRAY_CLICK_WINDOW));
    }

    #[test]
    fn the_open_settings_window_keeps_the_main_window_up() {
        let app = mock_builder().build(crate::context()).unwrap();
        // The mock runtime reports every window as unfocused.
        WebviewWindowBuilder::new(&app, crate::MAIN_WINDOW, Default::default())
            .build()
            .unwrap();
        assert!(should_hide(app.handle()));

        WebviewWindowBuilder::new(&app, settings_window::LABEL, Default::default())
            .build()
            .unwrap();
        assert!(!should_hide(app.handle()));
    }

    #[test]
    fn the_badge_is_painted_in_its_corner_with_a_gap_around_it() {
        let icon = Image::new_owned(PIXEL.repeat((SIZE * SIZE) as usize), SIZE, SIZE);
        let badged = with_badge(&icon);
        assert_eq!((badged.width(), badged.height()), (SIZE, SIZE));

        let center_x = (BADGE_CENTER.0 * SIZE as f32) as u32;
        let center_y = (BADGE_CENTER.1 * SIZE as f32) as u32;
        let [r, g, b] = BADGE_COLOR;
        assert_eq!(pixel(&badged, center_x, center_y), [r, g, b, 255]);

        // On the ring between the badge and the rest of the icon.
        let ring = ((BADGE_RADIUS + BADGE_GAP) / 2.0 * SIZE as f32) as u32;
        assert_eq!(pixel(&badged, center_x - ring, center_y)[3], 0);

        // The pawn's corner of the icon is left alone.
        assert_eq!(pixel(&badged, 0, SIZE - 1), PIXEL);
    }

    #[test]
    fn the_badge_keeps_a_non_square_icon_intact() {
        let icon = Image::new_owned(PIXEL.repeat(32 * 64), 32, 64);
        let badged = with_badge(&icon);
        assert_eq!(badged.rgba().len(), icon.rgba().len());
        assert_eq!(pixel(&badged, 0, 63), PIXEL);
    }
}
