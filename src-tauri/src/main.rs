// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Disable WebKitGTK's DMABUF renderer — a common cause of a blank/white
    // window on Linux with little downside. Must be set before the webview
    // starts.
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

        // An AppImage bundles its own GL/EGL stack, which on many hosts can't
        // create an EGL display ("EGL_BAD_PARAMETER. Aborting"), leaving a white
        // window. Disable accelerated compositing there so WebKit renders in
        // software (no EGL needed). The .deb uses the system WebKit + the host's
        // drivers, so it keeps GPU compositing.
        if std::env::var_os("APPIMAGE").is_some() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    rivetlink_app_lib::run()
}
