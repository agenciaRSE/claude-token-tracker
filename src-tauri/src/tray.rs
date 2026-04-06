use tauri::{
    AppHandle, Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
    image::Image,
};

use crate::state::PeakColor;

/// Generate a 32x32 colored circle PNG icon as raw RGBA pixels
fn generate_icon_rgba(color: PeakColor) -> Vec<u8> {
    let size: u32 = 32;
    let center = (size / 2) as f64;
    let radius = (size / 2 - 2) as f64; // 2px padding
    let [r, g, b, _a] = color.rgba();

    let mut pixels = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let offset = ((y * size + x) * 4) as usize;

            if dist <= radius - 1.0 {
                // Inner: solid color with slight radial gradient
                let brightness = 1.0 - (dist / radius) * 0.15;
                pixels[offset] = (r as f64 * brightness).min(255.0) as u8;
                pixels[offset + 1] = (g as f64 * brightness).min(255.0) as u8;
                pixels[offset + 2] = (b as f64 * brightness).min(255.0) as u8;
                pixels[offset + 3] = 255;
            } else if dist <= radius {
                // Anti-aliased edge
                let alpha = ((radius - dist + 1.0) * 255.0) as u8;
                pixels[offset] = r;
                pixels[offset + 1] = g;
                pixels[offset + 2] = b;
                pixels[offset + 3] = alpha;
            }
            // else: transparent (already 0)
        }
    }

    pixels
}

/// Create a Tauri Image from a PeakColor
fn color_to_image(color: PeakColor) -> Image<'static> {
    let rgba = generate_icon_rgba(color);
    Image::new_owned(rgba, 32, 32)
}

/// Build and register the system tray icon with menu
pub fn setup_tray(app: &AppHandle) -> tauri::Result<TrayIcon> {
    // Right-click context menu
    let dashboard_item = MenuItem::with_id(app, "dashboard", "Open Dashboard", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let refresh_item = MenuItem::with_id(app, "refresh", "Refresh Now", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[
        &dashboard_item,
        &settings_item,
        &tauri::menu::PredefinedMenuItem::separator(app)?,
        &refresh_item,
        &tauri::menu::PredefinedMenuItem::separator(app)?,
        &quit_item,
    ])?;

    let tray = TrayIconBuilder::new()
        .icon(color_to_image(PeakColor::Green))
        .tooltip("Claude Peak Monitor - Loading...")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    // Toggle popup window visibility
                    if let Some(window) = tray.app_handle().get_webview_window("popup") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.center();
                        }
                    }
                }
                _ => {}
            }
        })
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "dashboard" => {
                    if let Some(window) = app.get_webview_window("dashboard") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.center();
                    }
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("dashboard") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.center();
                        // Emit event to switch to settings tab
                        let _ = app.emit("navigate-settings", ());
                    }
                }
                "refresh" => {
                    let _ = app.emit("force-refresh", ());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(tray)
}

/// Update the tray icon color and tooltip text
pub fn update_tray(tray: &TrayIcon, color: PeakColor, score: u8) {
    let _ = tray.set_icon(Some(color_to_image(color)));
    let tooltip = format!(
        "Claude Peak Monitor - {} ({}/100)",
        color.label(),
        score
    );
    let _ = tray.set_tooltip(Some(&tooltip));
}
