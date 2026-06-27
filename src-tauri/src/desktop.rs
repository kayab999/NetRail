use crate::history;
use crate::server;
use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, RunEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "netrail=info,tower_http=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_main_window(app);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let search_shortcut =
                        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);
                    if shortcut == &search_shortcut {
                        focus_main_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let settings = crate::config::load_settings();
            history::init_history_on_startup(&settings);
            if history::encryption_degraded() {
                let _ = app.handle().emit(
                    "security:encryption-degraded",
                    history::encryption_degraded_message(),
                );
            }

            tauri::async_runtime::spawn(async move {
                if let Err(err) = server::start().await {
                    tracing::error!("API server failed: {err}");
                }
            });

            let help_menu = SubmenuBuilder::new(app, "Help")
                .text("doc-manual", "User Manual")
                .text("doc-about", "About NetRail")
                .build()?;

            let app_menu = MenuBuilder::new(app)
                .items(&[&help_menu])
                .text("donate", "Donate…")
                .build()?;

            app.set_menu(app_menu)?;

            app.on_menu_event(|app, event| match event.id().0.as_str() {
                "doc-manual" => trigger_doc_view(app, "manual"),
                "doc-about" => trigger_doc_view(app, "about"),
                "donate" => trigger_donate(app),
                _ => {}
            });

            let show = tauri::menu::MenuItem::with_id(app, "show", "Show NetRail", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;

            let mut tray_builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }

            let _tray = tray_builder
                .menu(&tray_menu)
                .tooltip("NetRail — search first, browse second")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => focus_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        focus_main_window(app);
                    }
                })
                .build(app)?;

            let shortcut =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);
            app.global_shortcut().register(shortcut)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
            let _ = app;
        });
}

fn focus_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn trigger_doc_view<R: tauri::Runtime>(app: &tauri::AppHandle<R>, slug: &str) {
    focus_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        let script = format!("window.netrailOpenDoc('{slug}')");
        let _ = window.eval(&script);
    }
}

fn trigger_donate<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    focus_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval("window.netrailDonate()");
    }
}