// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Disable WebKitGTK's DMABUF renderer — a common cause of a blank/white
    // window on Linux with little downside. Must be set before the webview
    // starts. (Heavier fallbacks like WEBKIT_DISABLE_COMPOSITING_MODE or
    // LIBGL_ALWAYS_SOFTWARE can be set in the environment if ever needed.)
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    rivetlink_app_lib::run()
}
