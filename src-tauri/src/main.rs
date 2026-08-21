// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    if matches!(
        std::env::args_os().nth(1).as_deref(),
        Some(arg) if arg == std::ffi::OsStr::new("--rivetlink-console-service")
    ) {
        let exit_code =
            match rivetlink_app_lib::run_console_service_action(std::env::args_os().skip(2)) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("RivetLink console service action failed: {error}");
                    1
                }
            };
        std::process::exit(exit_code);
    }

    // A single, intentionally narrow PolicyKit entry point for physical-console
    // installation. It accepts only validated setup fields, never opens a
    // webview or shell, and waits only for the public relay device id.
    #[cfg(target_os = "linux")]
    if matches!(
        std::env::args_os().nth(1).as_deref(),
        Some(arg) if arg == std::ffi::OsStr::new("--rivetlink-console-install")
    ) {
        let exit_code = match rivetlink_app_lib::run_console_installer(std::env::args_os().skip(2))
        {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("RivetLink console installer failed: {error}");
                1
            }
        };
        std::process::exit(exit_code);
    }

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
