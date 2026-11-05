//! The tray icon, which is always shown: on Linux and Windows there is no
//! menu bar, so it is the only mouse path to app actions.
//!
//! A left click toggles the main window and a right click opens the menu. On
//! Linux the icon is a StatusNotifierItem from tray-icon's ksni backend
//! (ADR 0014). Panels report a left click as `Activate`, which arrives here as a click.
//! The menu repeats the toggle as "Show/Hide" for panels that only show menus.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
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
/// How long the badge stays on and off: one flash per second.
const FLASH_INTERVAL: Duration = Duration::from_millis(500);

/// How long the focus may be gone before the main window hides: while it moves
/// to the palette, no window of the app has it.
const FOCUS_SETTLE: Duration = Duration::from_millis(150);
/// A tray click this soon after the focus went is the click that took it.
const TRAY_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// When the focus last left a window that hides on focus loss. In managed state.
#[derive(Default)]
struct LastFocusLoss(Mutex<Option<Instant>>);

/// Whether the tray menu is open. Only known on Linux, see `watch_menu`. In
/// managed state.
#[derive(Default)]
struct MenuOpen(AtomicBool);

/// Keeps the badge flashing while it holds a sender. In managed state.
#[derive(Default)]
struct Flashing(Mutex<Option<Sender<()>>>);

/// Creates the tray icon. The app keeps it alive until exit.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.manage(LastFocusLoss::default());
    app.manage(MenuOpen::default());
    app.manage(Flashing::default());
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
    #[cfg(target_os = "linux")]
    watch_menu(app);
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

/// Starts or stops flashing the badge that says it's the user's turn.
pub fn set_turn_badge<R: Runtime>(app: &AppHandle<R>, show: bool) {
    let stop = show.then(|| start_flashing(app));
    // Dropping the previous sender stops its thread.
    *app.state::<Flashing>()
        .0
        .lock()
        .unwrap_or_else(|err| err.into_inner()) = stop;
}

/// Flashes on a thread of its own, as `set_icon` waits for the main thread and
/// the caller may be on it. Returns the sender that keeps the flashing going.
fn start_flashing<R: Runtime>(app: &AppHandle<R>) -> Sender<()> {
    let (stop, stopped) = mpsc::channel();
    let app = app.clone();
    thread::spawn(move || {
        let (Some(tray), Some(icon)) = (app.tray_by_id(TRAY_ID), app.default_window_icon()) else {
            return;
        };
        let badged = with_badge(icon);
        flash(&stopped, |lit| {
            let frame = if lit { &badged } else { icon };
            if let Err(err) = tray.set_icon(Some(frame.clone())) {
                eprintln!("failed to update the tray icon: {err}");
            }
        });
    });
    stop
}

/// Alternates the badge until `stop` hangs up, and leaves it off. The hang-up
/// ends the wait, so the badge goes as soon as the window gets focus.
fn flash(stop: &Receiver<()>, mut set: impl FnMut(bool)) {
    for lit in [true, false].into_iter().cycle() {
        set(lit);
        if stop.recv_timeout(FLASH_INTERVAL) != Err(RecvTimeoutError::Timeout) {
            break;
        }
    }
    set(false);
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
            && hides_on_focus_loss(&app)
            && let Some(main) = app.get_webview_window(crate::MAIN_WINDOW)
            // Hiding takes the focus too, and that loss shouldn't swallow a tray click.
            && main.is_visible().unwrap_or(false)
        {
            *lock(&app) = Some(Instant::now());
            hide_soon(&app);
        }
    });
}

fn hides_on_focus_loss<R: Runtime>(app: &AppHandle<R>) -> bool {
    is_shown(app) && app.state::<SettingsStore<R>>().get().hide_on_focus_loss
}

fn hide_soon<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(FOCUS_SETTLE);
        if should_hide(&app) {
            crate::hide_main_window(&app);
        }
    });
}

/// Whether the focus really left. The open settings window keeps the main window
/// up: it is a child of it, and hiding its parent would leave it stranded. The
/// tray menu takes the focus while open, and hiding the window would close it.
/// A window the app keeps on top is meant to stay in view. The window manager's
/// own "keep above" can't be seen, on Wayland or X11 (ADR 0031).
fn should_hide<R: Runtime>(app: &AppHandle<R>) -> bool {
    !app.state::<MenuOpen>().0.load(Ordering::Relaxed)
        && app.get_webview_window(settings_window::LABEL).is_none()
        && !app
            .get_webview_window(crate::MAIN_WINDOW)
            .is_some_and(|main| main.is_always_on_top().unwrap_or(false))
        && !app
            .webview_windows()
            .values()
            .any(|window| window.is_focused().unwrap_or(false))
}

/// Follows the panel's calls on our tray menu: opening it takes the focus, and
/// tray-icon doesn't report that (ADR 0030). A panel that sends no such calls
/// hides the window as for any other focus loss.
#[cfg(target_os = "linux")]
fn watch_menu<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    thread::spawn(move || {
        if let Err(err) = monitor_menu(&app) {
            eprintln!("failed to watch the tray menu: {err}");
        }
    });
}

/// Uses a connection of its own, as a D-Bus monitor can't do anything else.
#[cfg(target_os = "linux")]
fn monitor_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(), gtk::glib::Error> {
    use gtk::gio::{self, BusType, Cancellable, DBusCallFlags, DBusConnection};
    use gtk::gio::{DBusConnectionFlags, DBusMessageType};
    use gtk::glib::prelude::ToVariant;

    const MENU_EVENTS: &str =
        "type='method_call',interface='com.canonical.dbusmenu',member='Event'";

    let address = gio::dbus_address_get_for_bus_sync(BusType::Session, Cancellable::NONE)?;
    let connection = DBusConnection::for_address_sync(
        &address,
        DBusConnectionFlags::AUTHENTICATION_CLIENT | DBusConnectionFlags::MESSAGE_BUS_CONNECTION,
        None,
        Cancellable::NONE,
    )?;
    let own_name = format!("org.kde.StatusNotifierItem-{}-", std::process::id());
    let filter_app = app.clone();
    connection.add_filter(move |_, message, incoming| {
        if !incoming || message.message_type() != DBusMessageType::MethodCall {
            return Some(message.clone());
        }
        let open = message
            .destination()
            .filter(|destination| destination.starts_with(&own_name))
            .and_then(|_| message.body())
            .and_then(|body| menu_opened(&body));
        if let Some(open) = open {
            menu_changed(&filter_app, open);
        }
        // Dropped, as GDBus would reply to the call and a monitor may not send.
        None
    });
    connection.call_sync(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus.Monitoring",
        "BecomeMonitor",
        Some(&(vec![MENU_EVENTS], 0_u32).to_variant()),
        None,
        DBusCallFlags::NONE,
        -1,
        Cancellable::NONE,
    )?;
    // Kept until exit: dropping it would end the monitor.
    app.manage(connection);
    Ok(())
}

/// Reads a dbusmenu `Event` call (`id, event, data, timestamp`): whether the root
/// menu, which has id 0, opened or closed.
#[cfg(target_os = "linux")]
fn menu_opened(body: &gtk::glib::Variant) -> Option<bool> {
    match body.get::<(i32, String, gtk::glib::Variant, u32)>()? {
        (0, event, ..) if event == "opened" => Some(true),
        (0, event, ..) if event == "closed" => Some(false),
        _ => None,
    }
}

// shortcut: a panel that dies with the menu open leaves this set until the next
// menu, so the window stays up until then. Rare enough not to handle.
#[cfg(target_os = "linux")]
fn menu_changed<R: Runtime>(app: &AppHandle<R>, open: bool) {
    app.state::<MenuOpen>().0.store(open, Ordering::Relaxed);
    // A click in another app closes the menu too, and only then does the focus count.
    if !open && hides_on_focus_loss(app) {
        hide_soon(app);
    }
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
        app.manage(MenuOpen::default());
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
    fn the_open_tray_menu_keeps_the_main_window_up() {
        let app = mock_builder().build(crate::context()).unwrap();
        app.manage(MenuOpen(AtomicBool::new(true)));
        assert!(!should_hide(app.handle()));

        app.state::<MenuOpen>().0.store(false, Ordering::Relaxed);
        assert!(should_hide(app.handle()));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn only_the_root_menu_opening_or_closing_counts() {
        use gtk::glib::Variant;
        use gtk::glib::prelude::ToVariant;

        let event = |id: i32, name: &str| (id, name, "".to_variant(), 0_u32).to_variant();
        assert_eq!(menu_opened(&event(0, "opened")), Some(true));
        assert_eq!(menu_opened(&event(0, "closed")), Some(false));
        assert_eq!(menu_opened(&event(3, "opened")), None);
        assert_eq!(menu_opened(&event(0, "clicked")), None);
        assert_eq!(menu_opened(&Variant::from("opened")), None);
    }

    #[test]
    fn the_badge_alternates_until_stopped_and_ends_off() {
        let (stop, stopped) = mpsc::channel::<()>();
        let (frame, frames) = mpsc::channel();
        thread::spawn(move || flash(&stopped, |lit| frame.send(lit).unwrap()));
        assert_eq!(frames.recv(), Ok(true));
        assert_eq!(frames.recv(), Ok(false));
        assert_eq!(frames.recv(), Ok(true));

        drop(stop);
        assert_eq!(frames.iter().last(), Some(false));
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
