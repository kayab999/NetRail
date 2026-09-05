use crate::server;
use tauri::{
    menu::{MenuBuilder, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Keep the tray icon alive for the app lifetime (dropping it removes the icon on Linux).
/// The field is intentionally unread — `app.manage` holds the icon so it is not dropped.
#[allow(dead_code)]
struct TrayState(TrayIcon);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    crate::logging::init("netrail=info,tower_http=warn");

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
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // S1 attach-explicit: never silently hijack a foreign :7421.
            match crate::instance::probe_existing() {
                crate::instance::ProbeOutcome::Empty => {
                    tauri::async_runtime::spawn(async move {
                        if let Err(err) = server::start().await {
                            // Bind race (another instance won between probe
                            // and bind): re-probe once before giving up.
                            match crate::instance::probe_existing() {
                                crate::instance::ProbeOutcome::NetRail(_) => {
                                    tracing::warn!(
                                        "bind failed ({err}) but a NetRail API is already up — attached, no second server"
                                    );
                                }
                                _ => {
                                    tracing::error!("API server failed: {err}");
                                    eprintln!("NetRail API server failed: {err}");
                                    std::process::exit(1);
                                }
                            }
                        }
                    });
                }
                crate::instance::ProbeOutcome::NetRail(fp) => {
                    tracing::warn!(
                        body = %fp.body_snippet,
                        "NetRail API already running on :7421 — attached to the existing instance, no second server started"
                    );
                    let handle = app.app_handle().clone();
                    tauri::async_runtime::spawn(async move {
                        mark_attached_banner(handle).await;
                    });
                }
                crate::instance::ProbeOutcome::Foreign(reason) => {
                    let msg = format!(
                        "Port 7421 is occupied by a non-NetRail process ({reason}). Refusing to hijack it — stop that process or free the port, then relaunch."
                    );
                    tracing::error!("{msg}");
                    eprintln!("{msg}");
                    std::process::exit(1);
                }
            }

            let help_menu = SubmenuBuilder::new(app, "Help")
                .text("doc-manual", "User Manual")
                .text("doc-about", "About NetRail")
                .build()?;

            let app_menu = MenuBuilder::new(app)
                .items(&[&help_menu])
                .text("donate", "Donate…")
                .build()?;

            app.set_menu(app_menu)?;

            app.on_menu_event(|app, event| {
                tracing::debug!(menu_id = %event.id().0, "menu event");
                match event.id().0.as_str() {
                    "show" => focus_main_window(app),
                    "quit" => quit_app(app),
                    "doc-manual" => trigger_doc_view(app, "manual"),
                    "doc-about" => trigger_doc_view(app, "about"),
                    "donate" => trigger_donate(app),
                    _ => {}
                }
            });

            let show = tauri::menu::MenuItem::with_id(app, "show", "Show NetRail", true, None::<&str>)?;
            let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = tauri::menu::Menu::with_items(app, &[&show, &quit])?;

            let mut tray_builder = TrayIconBuilder::with_id("main");
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }

            let tray = tray_builder
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .tooltip("NetRail — search first, browse second")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => focus_main_window(app),
                    "quit" => quit_app(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        focus_main_window(tray.app_handle());
                    }
                })
                .build(app)?;
            app.manage(TrayState(tray));

            // Optional global shortcut: a compositor without the portal must
            // never make the whole app unusable (S2). Log and continue.
            let shortcut =
                Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);
            if let Err(err) = app.global_shortcut().register(shortcut) {
                tracing::warn!(error = %err, "global shortcut unavailable — continuing without Ctrl+Shift+S");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Confirm-always (S2): closing the X must never silently
                // orphan a live API process. Ask every time: tray or quit.
                api.prevent_close();
                ask_close_action(window.clone());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {});
}

fn focus_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::warn!("main window not found for focus");
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    // Wayland compositors sometimes ignore set_focus; briefly pin on top.
    let _ = window.set_always_on_top(true);
    let _ = window.set_always_on_top(false);

    // Spotlight UX: put the caret in the query box after the OS settles focus.
    // The webview is bridged via eval only (withGlobalTauri: false), matching
    // the docs/donate bridge. Tauri event emit is dead without the Tauri API
    // in the page, so it is intentionally not used here (A7).
    let _ = window.eval(
        "window.setTimeout(function(){if(window.netrailFocusSearch)window.netrailFocusSearch();},50)",
    );
}

/// Explicit-attach banner: the webview is now served by a *pre-existing*
/// NetRail API, not by this process. Retry briefly — the window may not
/// exist yet during `setup`.
async fn mark_attached_banner<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    use tauri::Manager;
    for _ in 0..20 {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.eval(
                r#"try{if(!document.getElementById('netrail-attached-banner')){var d=document.createElement('div');d.id='netrail-attached-banner';d.textContent='Attached to the already-running NetRail API (:7421) — this window did not start a second server.';d.style.cssText='position:fixed;top:0;left:0;right:0;z-index:9999;background:#3a2f00;color:#ffe9a8;font:12px system-ui;padding:6px 12px;text-align:center';document.body.prepend(d);}}catch(e){}"#,
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    tracing::warn!("attached to existing API but main window never appeared for banner");
}

/// Native confirm-on-close (S2). Custom button labels keep the choice
/// explicit: hiding is a deliberate "keep running", never an accident.
fn ask_close_action<R: tauri::Runtime>(window: tauri::Window<R>) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    let app = window.app_handle().clone();
    window
        .dialog()
        .message("NetRail keeps its local API running in the background. Minimize to the tray, or quit fully?")
        .title("Close NetRail")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Keep running in tray".into(),
            "Quit NetRail".into(),
        ))
        .show(move |keep_running| {
            match keep_running {
                // Ok == first button == keep running.
                true => {
                    tracing::info!("close confirmed: minimizing to tray");
                    let _ = window.hide();
                }
                // Cancel/Esc/dialog-close == quit. WAL checkpoint first so
                // -wal/-shm do not linger (S2), then exit.
                false => {
                    tracing::info!("close confirmed: quitting");
                    graceful_quit(&app);
                }
            }
        });
}

/// Quit path shared by the close dialog and the tray/menu items: checkpoint
/// the history WAL (best-effort) before exiting, so a GUI quit does not
/// leave -wal/-shm behind. In-flight Axum requests are aborted on desktop
/// quit — acceptable for a single-user local app; the headless
/// `netrail-api` keeps its SIGTERM drain (server::shutdown_signal).
fn graceful_quit<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    crate::history::wal_checkpoint();
    app.exit(0);
}

fn quit_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    tracing::info!("NetRail quit requested from menu");
    graceful_quit(app);
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