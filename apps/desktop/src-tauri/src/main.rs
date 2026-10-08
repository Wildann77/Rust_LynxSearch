// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Safe WebKitGTK rendering on Wayland with GNOME Mutter extensions (Forge, Tiling Assistant)
        // Disabling DMABUF renderer prevents Clutter actor assertion failures
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            unsafe {
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }
    desktop_lib::run();
}
