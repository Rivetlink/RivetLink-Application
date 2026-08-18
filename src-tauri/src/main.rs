// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // RivetLink's system and session services need a durable executable.  This
    // private argument starts only the agent and never opens a webview.  It is
    // intentionally not exposed as a normal application action.
    #[cfg(target_os = "linux")]
    if matches!(
        std::env::args_os().nth(1).as_deref(),
        Some(arg)
            if arg == std::ffi::OsStr::new("--rivetlink-agent")
                // Compatibility for an already installed pre-physical-console
                // service. New units exclusively use --rivetlink-agent.
                || arg == std::ffi::OsStr::new("--rivetlink-headless-agent")
    ) {
        let args = std::env::args_os()
            .enumerate()
            .filter_map(|(index, arg)| (index != 1).then_some(arg));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create RivetLink agent runtime");
        let exit_code = match runtime.block_on(rivetlink_agent::runner::run_from(args)) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("RivetLink agent failed: {error}");
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
