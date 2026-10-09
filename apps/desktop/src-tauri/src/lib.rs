use tauri::Manager;

static TARGET_ZOOM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[tauri::command]
fn open_file_in_editor(path: String, editor_cmd: Option<String>) -> Result<(), String> {
    let resolved_cmd = editor_cmd
        .filter(|c| !c.trim().is_empty())
        .or_else(|| {
            std::env::var("VISUAL")
                .ok()
                .filter(|c| !c.trim().is_empty())
        })
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|c| !c.trim().is_empty())
        });

    if let Some(cmd) = resolved_cmd {
        match cmd.as_str() {
            "default" => {
                return tauri_plugin_opener::open_path(&path, None::<&str>)
                    .map_err(|e| e.to_string());
            }
            "antigravity-ide" => {
                if std::process::Command::new("antigravity-ide")
                    .args(["-r", &path])
                    .spawn()
                    .is_ok()
                    || std::process::Command::new("antigravity-ide")
                        .arg(&path)
                        .spawn()
                        .is_ok()
                {
                    return Ok(());
                }
            }
            "code" => {
                if std::process::Command::new("code")
                    .args(["-r", &path])
                    .spawn()
                    .is_ok()
                    || std::process::Command::new("code")
                        .arg(&path)
                        .spawn()
                        .is_ok()
                    || std::process::Command::new("antigravity-ide")
                        .args(["-r", &path])
                        .spawn()
                        .is_ok()
                {
                    return Ok(());
                }
            }
            "gnome-text-editor" => {
                if std::process::Command::new("gnome-text-editor")
                    .arg(&path)
                    .spawn()
                    .is_ok()
                {
                    return Ok(());
                }
            }
            custom => {
                let parts: Vec<&str> = custom.split_whitespace().collect();
                if let Some((bin, args)) = parts.split_first()
                    && std::process::Command::new(bin)
                        .args(args)
                        .arg(&path)
                        .spawn()
                        .is_ok()
                {
                    return Ok(());
                }
            }
        }
    }

    // Default cascading fallbacks
    if std::process::Command::new("antigravity-ide")
        .args(["-r", &path])
        .spawn()
        .is_ok()
        || std::process::Command::new("antigravity-ide")
            .arg(&path)
            .spawn()
            .is_ok()
    {
        return Ok(());
    }
    if std::process::Command::new("code")
        .args(["-r", &path])
        .spawn()
        .is_ok()
        || std::process::Command::new("code")
            .arg(&path)
            .spawn()
            .is_ok()
    {
        return Ok(());
    }
    if std::process::Command::new("gnome-text-editor")
        .arg(&path)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn reveal_file_in_folder(path: String) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_window(window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.set_always_on_top(false);
    let _ = window.unminimize();
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_desktop_zoom(window: tauri::WebviewWindow, scale: f64) -> Result<(), String> {
    TARGET_ZOOM.store(scale.to_bits(), std::sync::atomic::Ordering::SeqCst);
    window.set_zoom(scale).map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    #[cfg(target_os = "linux")]
    {
        // Enforce sensible window size for GTK File Chooser so it fits laptop screens (768p/1080p)
        let _ = std::process::Command::new("gsettings")
            .args([
                "set",
                "org.gtk.Settings.FileChooser",
                "window-size",
                "(900, 560)",
            ])
            .output();
    }

    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = tokio::sync::oneshot::channel();

    let mut builder = app.dialog().file();
    if let Some(window) = app.get_webview_window("main") {
        builder = builder.set_parent(&window);
    }

    builder.pick_folder(move |folder| {
        let path = folder
            .and_then(|f| f.into_path().ok())
            .map(|p| p.to_string_lossy().to_string());
        let _ = tx.send(path);
    });

    rx.await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    TARGET_ZOOM.store(1.0f64.to_bits(), std::sync::atomic::Ordering::SeqCst);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            open_file_in_editor,
            reveal_file_in_folder,
            pick_folder,
            focus_window,
            set_desktop_zoom
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(false);
                #[cfg(target_os = "linux")]
                {
                    use gtk::prelude::WidgetExt;
                    use webkit2gtk::WebViewExt;
                    let _ = window.with_webview(|platform_webview| {
                        let webview = platform_webview.inner();

                        // 1. Intercept and discard GDK TouchpadPinch events before WebKit receives them
                        webview.connect_event(|_, event| {
                            if event.event_type() == gdk::EventType::TouchpadPinch {
                                glib::Propagation::Stop
                            } else {
                                glib::Propagation::Proceed
                            }
                        });

                        // 2. Lock zoom level against any gesture drift
                        webview.connect_zoom_level_notify(move |wv| {
                            let target_bits = TARGET_ZOOM.load(std::sync::atomic::Ordering::SeqCst);
                            let target_zoom = f64::from_bits(target_bits);
                            if target_zoom > 0.0 {
                                let current = wv.zoom_level();
                                if (current - target_zoom).abs() > 0.005 {
                                    wv.set_zoom_level(target_zoom);
                                }
                            }
                        });
                    });
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LynxSearch desktop application");
}
