# Tauri GUI Warnings Fix

**Date:** 2025-12-08
**Issue:** libayatana-appindicator and Wayland warnings when running wraith-transfer

## Root Cause Analysis

### Warning 1: libayatana-appindicator Deprecation

```
(wraith-transfer:1299417): libayatana-appindicator-WARNING **: 19:07:48.394:
libayatana-appindicator is deprecated. Please use libayatana-appindicator-glib
in newly written code.
```

**Root Cause:**
- System tray is enabled in `tauri.conf.json` (lines 28-31)
- `tray-icon` feature is enabled in `Cargo.toml` (line 20)
- Tauri initializes the system tray on Linux using libayatana-appindicator
- The library warns about its own deprecation in favor of a newer variant
- **System tray is not actually used** in the application code

**Impact:**
- Warning appears on every application launch
- No functional impact, purely informational
- Clutters CLI output for `--help` and `-V` commands

### Warning 2: Wayland Protocol Error

```
Gdk-Message: 19:08:19.354: Error 71 (Protocol error) dispatching to
Wayland display.
```

**Root Cause:**
- Tauri is a GUI framework that initializes the entire windowing subsystem
- Even for CLI-style invocations (`--help`, `-V`), the GUI subsystem starts
- GTK/GDK attempts to communicate with the Wayland display server
- Error 71 (EPROTO) occurs during display protocol communication
- Likely happens during application shutdown when display resources are cleaned up

**Impact:**
- Warning appears on application exit
- No functional impact on GUI operation
- Indicates improper cleanup sequence for headless CLI invocations

## Solutions

### Solution 1: Disable System Tray (Recommended)

Since the system tray is not actually used in the application, we should remove it entirely.

**Changes Required:**

1. **Remove tray-icon feature from Cargo.toml:**

```diff
--- a/clients/wraith-transfer/src-tauri/Cargo.toml
+++ b/clients/wraith-transfer/src-tauri/Cargo.toml
@@ -17,7 +17,7 @@ tauri-build = { version = "2.5.3", features = [] }

 [dependencies]
 # Tauri framework
-tauri = { version = "2.9.4", features = ["tray-icon"] }
+tauri = { version = "2.9.4", features = [] }
 tauri-plugin-log = "2"
 tauri-plugin-dialog = "2"
 tauri-plugin-fs = "2"
```

2. **Remove trayIcon configuration from tauri.conf.json:**

```diff
--- a/clients/wraith-transfer/src-tauri/tauri.conf.json
+++ b/clients/wraith-transfer/src-tauri/tauri.conf.json
@@ -24,11 +24,6 @@
     ],
     "security": {
       "csp": "default-src 'self'; img-src 'self' data: asset: https://asset.localhost; style-src 'self' 'unsafe-inline'"
-    },
-    "trayIcon": {
-      "iconPath": "icons/32x32.png",
-      "iconAsTemplate": true
     }
   },
   "bundle": {
```

**Benefits:**
- Completely eliminates the libayatana-appindicator warning
- Reduces application startup time (no tray initialization)
- Smaller binary size (no tray-icon dependencies)
- Cleaner dependency tree

**Drawbacks:**
- If system tray functionality is needed in the future, it must be re-enabled

### Solution 2: Conditional System Tray (If Needed in Future)

If system tray functionality is planned for future use, make it optional:

```rust
// In lib.rs, conditionally setup tray icon
#[cfg(feature = "tray-icon")]
fn setup_tray(app: &tauri::AppHandle) {
    use tauri::tray::{TrayIcon, TrayIconBuilder};

    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .build(app)
        .unwrap();
}

pub fn run() {
    let builder = tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![/* ... */]);

    #[cfg(feature = "tray-icon")]
    let builder = builder.setup(|app| {
        setup_tray(app.handle());
        Ok(())
    });

    builder
        .run(tauri::generate_context!())
        .expect("error while running WRAITH Transfer");
}
```

Then use `cargo build --features tray-icon` when needed.

### Solution 3: Suppress Wayland Warning (Environment Variable)

The Wayland protocol error is harder to fix without modifying Tauri internals. For now, users can suppress it:

**For end users (add to shell rc file):**

```bash
# Suppress GTK/GDK Wayland protocol warnings
export GDK_BACKEND=x11  # Force X11 backend (not recommended, may cause other issues)

# OR suppress all GTK warnings (better approach)
export G_MESSAGES_DEBUG=""
export GTK_DEBUG=""
```

**For development (temporary):**

```bash
# Run with warnings suppressed
G_MESSAGES_DEBUG="" wraith-transfer --help
```

### Solution 4: Document Known Issue (If Unfixable)

If the Wayland warning cannot be eliminated without major Tauri changes, document it as a known issue:

**In TROUBLESHOOTING.md:**

```markdown
### Wayland Protocol Warnings

**Symptom:**
```
Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.
```

**Cause:**
Tauri initializes GUI subsystems even for CLI-style invocations. This is a
harmless warning from GTK/GDK during cleanup.

**Impact:**
None. The application functions correctly despite this warning.

**Workaround:**
Suppress GTK messages:
```bash
G_MESSAGES_DEBUG="" wraith-transfer --help
```
```

## Recommended Implementation

**Phase 1 (Immediate - High Priority):**
1. ✅ Remove system tray configuration (Solution 1)
   - Eliminates libayatana-appindicator warning completely
   - No functional loss (tray not used)
   - Clean, permanent fix

**Phase 2 (Future - Low Priority):**
2. ⏸️ Investigate Wayland warning upstream
   - Check Tauri issue tracker for similar reports
   - Consider contributing fix to Tauri if feasible
   - May require changes to Tauri's GTK initialization logic

**Phase 3 (Documentation):**
3. 📝 Document remaining warnings (if any)
   - Add to TROUBLESHOOTING.md
   - Provide user-facing workarounds
   - Include in known issues section

## Implementation Steps

### Step 1: Remove System Tray

```bash
cd /home/parobek/Code/WRAITH-Protocol

# Edit Cargo.toml
# Change line 20: tauri = { version = "2.9.4", features = [] }

# Edit tauri.conf.json
# Remove lines 28-31 (trayIcon block)

# Rebuild
cd clients/wraith-transfer
cargo build

# Test
./target/debug/wraith-transfer --help
./target/debug/wraith-transfer -V
```

### Step 2: Verify Fix

**Expected output after fix:**
- ✅ No libayatana-appindicator warning
- ⚠️ Wayland warning may still appear (acceptable)
- ✅ Application functions normally

### Step 3: Update Documentation

Add to `docs/troubleshooting/KNOWN_ISSUES.md`:

```markdown
## Wayland Protocol Warnings (Non-Critical)

Tauri applications may emit GTK/GDK Wayland protocol warnings during
initialization and cleanup. This is a known limitation of the Tauri
framework and does not affect functionality.

**Workaround:** Set `G_MESSAGES_DEBUG=""` environment variable.
```

## Testing Checklist

After implementing Solution 1:

- [ ] No libayatana-appindicator warning on Linux
- [ ] Application launches normally
- [ ] GUI displays correctly
- [ ] All IPC commands functional
- [ ] CLI help works: `wraith-transfer --help`
- [ ] Version display works: `wraith-transfer -V`
- [ ] Binary size reduced (check with `ls -lh`)
- [ ] Startup time improved (subjective test)

## References

- [Tauri System Tray Documentation](https://v2.tauri.app/reference/javascript/api/namespacewindow/#tray)
- [libayatana-appindicator Project](https://github.com/AyatanaIndicators/libayatana-appindicator)
- [GTK Wayland Backend](https://docs.gtk.org/gdk3/class.Display.html)
- [Tauri Issue #8234: Wayland Protocol Errors](https://github.com/tauri-apps/tauri/issues/8234) (example)

## Conclusion

**Recommended Action:** Implement Solution 1 (Remove System Tray)

- **Effort:** Low (2 file edits)
- **Risk:** None (feature not used)
- **Benefit:** High (cleaner output, faster startup)
- **Status:** Ready to implement

The libayatana-appindicator warning can be completely eliminated by removing unused system tray functionality. The Wayland protocol warning is a Tauri framework limitation that can be suppressed via environment variables if needed, but is generally harmless.
