//! WRAITH Vault - Distributed Secret Storage
//!
//! Entry point for the WRAITH Vault desktop application.
//! This application provides secure distributed secret storage using
//! Shamir's Secret Sharing with a guardian network.

// Prevents additional console window on Windows in release
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    // Wayland compatibility workaround for Tauri
    // https://github.com/nickvidal/wraith-protocol/blob/main/docs/troubleshooting/TAURI_WARNINGS.md
    #[cfg(target_os = "linux")]
    {
        // Prefer X11 backend on Linux for better WebView compatibility
        if std::env::var("GDK_BACKEND").is_err() {
            // SAFETY: Called at program startup before any threads are spawned
            unsafe { std::env::set_var("GDK_BACKEND", "x11") };
        }
        // Disable WebKitGTK compositing mode (fixes transparency issues)
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
            // SAFETY: Called at program startup before any threads are spawned
            unsafe { std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1") };
        }
    }

    wraith_vault_lib::run();
}
