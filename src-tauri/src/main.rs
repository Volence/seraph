// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK's GPU compositing path crashes on some pure-Wayland setups
    // ("Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display").
    // Disable compositing unless the user has explicitly set the var themselves.
    // Must happen before any GTK/WebKit initialization.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_COMPOSITING_MODE").is_none() {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    seraph_lib::run()
}
