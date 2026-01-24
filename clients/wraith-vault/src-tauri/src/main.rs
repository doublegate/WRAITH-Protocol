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
    // Workaround for Wayland Error 71 (Protocol error) on KDE Plasma 6
    // See: https://github.com/tauri-apps/tauri/issues/10702
    //      https://github.com/tauri-apps/tao/issues/977
    //
    // WebKitGTK has compatibility issues with Wayland on KDE Plasma 6, causing
    // "Error 71 (Protocol error) dispatching to Wayland display" crashes.
    // This is an upstream issue blocked by tao/webkit2gtk compatibility.
    //
    // Solution: Automatically fallback to X11 via XWayland if:
    // 1. We're on Linux
    // 2. GDK_BACKEND is not already set (respect user preference)
    // 3. We're in a Wayland session
    // 4. We're on KDE Plasma 6 (or any Wayland compositor with issues)
    #[cfg(target_os = "linux")]
    {
        use std::env;

        // Only set GDK_BACKEND if not already configured by user
        if env::var("GDK_BACKEND").is_err() {
            // Check if we're in a Wayland session
            if let Ok(session_type) = env::var("XDG_SESSION_TYPE") {
                if session_type == "wayland" {
                    // Check for KDE Plasma (common source of Error 71)
                    let is_kde = env::var("KDE_SESSION_VERSION").is_ok()
                        || env::var("KDE_FULL_SESSION").is_ok()
                        || env::var("DESKTOP_SESSION")
                            .map(|s| s.contains("plasma") || s.contains("kde"))
                            .unwrap_or(false);

                    if is_kde {
                        eprintln!(
                            "Detected KDE Plasma on Wayland - forcing X11 backend to avoid Error 71"
                        );
                        eprintln!("See: https://github.com/tauri-apps/tauri/issues/10702");
                        // SAFETY: We're in main() before any threads are spawned,
                        // so there's no risk of data races with other threads reading env vars
                        unsafe {
                            env::set_var("GDK_BACKEND", "x11");
                        }
                    } else {
                        // For other Wayland compositors, prefer Wayland but fallback to X11
                        // This allows GDK to try Wayland first, then X11 if issues occur
                        // SAFETY: We're in main() before any threads are spawned,
                        // so there's no risk of data races with other threads reading env vars
                        unsafe {
                            env::set_var("GDK_BACKEND", "wayland,x11");
                        }
                    }
                }
            }
        }

        // Workaround for GBM (Generic Buffer Management) errors
        // See: https://github.com/tauri-apps/tauri/issues/13493
        //      https://github.com/winfunc/opcode/issues/26
        //
        // WebKitGTK's hardware-accelerated compositing can fail with:
        // "Failed to create GBM buffer of size WxH: Invalid argument"
        //
        // This occurs due to incompatibility between WebKitGTK, Mesa, and GPU drivers
        // (especially NVIDIA). Disabling compositing mode forces WebKit to use a
        // simpler, more compatible rendering path.
        if env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
            // SAFETY: We're in main() before any threads are spawned
            unsafe {
                env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            }
        }
    }

    wraith_vault_lib::run();
}
