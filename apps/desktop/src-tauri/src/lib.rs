use tauri::Manager;

static TARGET_ZOOM: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[tauri::command]
fn open_file_in_editor(path: String) -> Result<(), String> {
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn focus_window(window: tauri::WebviewWindow) -> Result<(), String> {
    let _ = window.set_always_on_top(false);
    let _ = window.unminimize();
    let _ = window.show();
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
            .args(["set", "org.gtk.Settings.FileChooser", "window-size", "(900, 560)"])
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
                let _ = window.set_always_on_top(false);
                let _ = window.unminimize();
                let _ = window.show();
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

