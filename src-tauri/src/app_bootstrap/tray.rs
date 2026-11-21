use tauri::Manager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use crate::{lcu_connector, models::AppLocale, storage};

use super::window_factory;

pub const MAIN_TRAY_ID: &str = "main-tray";
const MENU_ID_OPEN: &str = "tray-open";
const MENU_ID_STATUS: &str = "tray-lcu-status";
const MENU_ID_QUIT: &str = "tray-quit";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayLabels {
    pub open: &'static str,
    pub status: &'static str,
    pub quit: &'static str,
    pub tooltip: &'static str,
}

/// Resolves localized system tray menu labels and hover tooltip for the current locale and LCU state.
pub fn tray_labels(locale: AppLocale, lcu_connected: bool) -> TrayLabels {
    match locale {
        AppLocale::PtBr => TrayLabels {
            open: "Abrir schumaz.gg",
            status: if lcu_connected {
                "Cliente do LoL: Conectado"
            } else {
                "Cliente do LoL: Aguardando..."
            },
            quit: "Encerrar",
            tooltip: if lcu_connected {
                "schumaz.gg — Cliente do LoL conectado"
            } else {
                "schumaz.gg — Aguardando cliente do LoL"
            },
        },
        AppLocale::EnUs => TrayLabels {
            open: "Open schumaz.gg",
            status: if lcu_connected {
                "LoL Client: Connected"
            } else {
                "LoL Client: Waiting..."
            },
            quit: "Quit",
            tooltip: if lcu_connected {
                "schumaz.gg — LoL client connected"
            } else {
                "schumaz.gg — Waiting for LoL client"
            },
        },
    }
}

/// Managed handles for live, zero-polling updates to the native system tray menu items.
pub struct TrayMenuState {
    open_item: MenuItem<tauri::Wry>,
    status_item: MenuItem<tauri::Wry>,
    quit_item: MenuItem<tauri::Wry>,
}

fn apply_tray_labels(app_handle: &tauri::AppHandle, locale: AppLocale, lcu_connected: bool) {
    let labels = tray_labels(locale, lcu_connected);
    if let Some(menu_state) = app_handle.try_state::<TrayMenuState>() {
        let _ = menu_state.open_item.set_text(labels.open);
        let _ = menu_state.status_item.set_text(labels.status);
        let _ = menu_state.quit_item.set_text(labels.quit);
    }
    if let Some(tray) = app_handle.tray_by_id(MAIN_TRAY_ID) {
        let _ = tray.set_tooltip(Some(labels.tooltip));
    }
}

/// Synchronizes tray menu strings and tooltip after the user switches the active app locale.
pub fn refresh_tray_locale(app_handle: &tauri::AppHandle, locale: AppLocale) {
    apply_tray_labels(app_handle, locale, lcu_connector::is_lcu_connected());
}

fn spawn_tray_lcu_watcher(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut rx = lcu_connector::subscribe_lcu_connection();
        while rx.changed().await.is_ok() {
            let connected = *rx.borrow();
            let locale = storage::get_global_settings()
                .general
                .language
                .unwrap_or_default();
            apply_tray_labels(&app_handle, locale, connected);
        }
    });
}

/// Builds the native Tauri v2 system tray icon and reactive LCU status menu.
pub fn setup_system_tray(
    app: &tauri::App,
    initial_locale: AppLocale,
) -> Result<(), Box<dyn std::error::Error>> {
    let initial_connected = lcu_connector::is_lcu_connected();
    let labels = tray_labels(initial_locale, initial_connected);

    let open_item = MenuItem::with_id(app, MENU_ID_OPEN, labels.open, true, None::<&str>)?;
    let status_item = MenuItem::with_id(app, MENU_ID_STATUS, labels.status, false, None::<&str>)?;
    let separator_top = PredefinedMenuItem::separator(app)?;
    let separator_bottom = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, MENU_ID_QUIT, labels.quit, true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open_item,
            &separator_top,
            &status_item,
            &separator_bottom,
            &quit_item,
        ],
    )?;

    app.manage(TrayMenuState {
        open_item,
        status_item,
        quit_item,
    });

    let mut builder = TrayIconBuilder::with_id(MAIN_TRAY_ID)
        .menu(&menu)
        .tooltip(labels.tooltip)
        .show_menu_on_left_click(false)
        .on_menu_event(|app_handle, event| match event.id().as_ref() {
            MENU_ID_OPEN => {
                let _ = window_factory::show_or_create_main_window(app_handle);
            }
            MENU_ID_QUIT => {
                window_factory::allow_app_exit(app_handle);
                app_handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                let _ = window_factory::show_or_create_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    spawn_tray_lcu_watcher(app.handle().clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_labels_localized_for_connected_and_waiting_states() {
        let pt_waiting = tray_labels(AppLocale::PtBr, false);
        assert_eq!(pt_waiting.open, "Abrir schumaz.gg");
        assert_eq!(pt_waiting.status, "Cliente do LoL: Aguardando...");
        assert_eq!(pt_waiting.quit, "Encerrar");

        let pt_connected = tray_labels(AppLocale::PtBr, true);
        assert_eq!(pt_connected.status, "Cliente do LoL: Conectado");
        assert!(pt_connected.tooltip.contains("conectado"));

        let en_waiting = tray_labels(AppLocale::EnUs, false);
        assert_eq!(en_waiting.open, "Open schumaz.gg");
        assert_eq!(en_waiting.status, "LoL Client: Waiting...");
        assert_eq!(en_waiting.quit, "Quit");

        let en_connected = tray_labels(AppLocale::EnUs, true);
        assert_eq!(en_connected.status, "LoL Client: Connected");
        assert!(en_connected.tooltip.contains("connected"));
    }
}
