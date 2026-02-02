# WRAITH Transfer - Troubleshooting Guide

Quick reference for common issues and solutions.

## Linux: Application Crashes on Startup

### Error 71: Protocol error dispatching to Wayland display

**Symptoms:**
- Window briefly appears then closes (0.5-1 second)
- Terminal shows: `Gdk-Message: Error 71 (Protocol error) dispatching to Wayland display.`

**Solution:**
Application **automatically fixes this** on KDE Plasma 6. If you still see the error:

```bash
# Force X11 backend manually
GDK_BACKEND=x11 wraith-transfer
```

**More info:** See [WAYLAND.md](./WAYLAND.md)

### Failed to create GBM buffer

**Symptoms:**
- Error: `Failed to create GBM buffer of size WxH: Invalid argument`
- Blank/white window
- Common with NVIDIA GPUs

**Solution:**
Application **automatically fixes this** by disabling WebKit compositing. If issues persist:

```bash
# Manually disable compositing
WEBKIT_DISABLE_COMPOSITING_MODE=1 wraith-transfer

# Combine with X11 backend
GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 wraith-transfer
```

**More info:** See [WAYLAND.md](./WAYLAND.md)

## Application Won't Start in Development Mode

**Symptoms:**
- `cargo tauri dev` fails with GTK initialization errors
- "Failed to initialize gtk backend!" errors

**Solution:**
```bash
# Set X11 backend for development
GDK_BACKEND=x11 cargo tauri dev

# Or add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export GDK_BACKEND=x11
```

## Window is Blank or Shows White Screen

**Possible Causes:**
1. Graphics driver issues (especially NVIDIA)
2. WebKitGTK hardware acceleration problems
3. Older GPU lacking required features

**Solutions:**

### Quick Fix
```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 wraith-transfer
```

### Check Graphics Setup
```bash
# Check GPU
lspci | grep VGA

# Check WebKitGTK version
# Arch/CachyOS:
pacman -Q webkit2gtk

# Ubuntu/Debian:
dpkg -l | grep webkit2gtk

# Fedora:
rpm -qa | grep webkit2gtk
```

### Try Different Graphics Output
If using NVIDIA, try switching HDMI/DisplayPort to integrated Intel/AMD graphics.

### Update Graphics Drivers
```bash
# Arch/CachyOS
sudo pacman -Syu

# Ubuntu/Debian
sudo apt update && sudo apt upgrade

# Fedora
sudo dnf upgrade
```

## Node/Core Errors

### Failed to start WRAITH node

**Symptoms:**
- UI shows "Failed to start node" error
- Backend logs show port/binding errors

**Solution:**
Check if another instance is running:
```bash
# Check for running instances
ps aux | grep wraith-transfer
killall wraith-transfer

# Check if port is in use (if applicable)
sudo netstat -tulpn | grep :PORT
```

### Permission Errors

**Symptoms:**
- "Permission denied" errors
- Network interface binding failures

**Solution:**
```bash
# Add user to required groups
sudo usermod -aG video,render $USER

# Logout and login for changes to take effect
```

## Frontend/UI Issues

### UI Doesn't Load or Shows React Errors

**Symptoms:**
- Blank window with console errors
- React component failures

**Solution:**
```bash
# Rebuild frontend
cd clients/wraith-transfer/frontend
npm install
npm run build

# Clear cache
rm -rf node_modules .next dist
npm install
npm run build
```

### Styles/CSS Not Loading

**Symptoms:**
- Unstyled or broken layout
- Missing Tailwind CSS classes

**Solution:**
```bash
# Rebuild with cleared cache
cd clients/wraith-transfer/frontend
rm -rf dist
npm run build
```

## IPC (Backend ↔ Frontend) Errors

### Commands Not Found

**Symptoms:**
- Frontend shows "Command not found" errors
- IPC invoke failures

**Solution:**
Check backend is properly exposing commands in `src-tauri/src/lib.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    commands::get_node_status,
    commands::start_node,
    // ... ensure all commands are listed
])
```

## Build Issues

### Compilation Errors

**Symptoms:**
- `cargo build` fails
- Dependency resolution errors

**Solution:**
```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build --release
```

### Missing Dependencies

**Arch/CachyOS:**
```bash
sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator
```

**Ubuntu/Debian:**
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev
```

**Fedora:**
```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel
```

## Diagnostic Information

When reporting issues, include this diagnostic output:

```bash
# System info
uname -a
echo "Desktop: $DESKTOP_SESSION"
echo "Session Type: $XDG_SESSION_TYPE"
echo "KDE Version: $KDE_SESSION_VERSION"

# Package versions
# Arch/CachyOS:
pacman -Q webkit2gtk gtk3 | grep -E "webkit2gtk|gtk3"

# Application info
wraith-transfer --version
cargo --version
rustc --version

# Tauri dependencies
cd clients/wraith-transfer/src-tauri
cargo tree -p tauri
cargo tree -p tao
cargo tree -p webkit2gtk
```

## Getting Help

1. **Check existing documentation:**
   - [WAYLAND.md](./WAYLAND.md) - Wayland compatibility
   - [WAYLAND_FIX_SUMMARY.md](./WAYLAND_FIX_SUMMARY.md) - Wayland fix summary

2. **Search known issues:**
   - [WRAITH Protocol Issues](https://github.com/doublegate/WRAITH-Protocol/issues)
   - [Tauri Issues](https://github.com/tauri-apps/tauri/issues)

3. **Report a new issue:**
   - Include diagnostic information (above)
   - Describe your environment (OS, DE, GPU)
   - Provide error messages and logs
   - Open issue at: https://github.com/doublegate/WRAITH-Protocol/issues

## Quick Reference

| Issue | Solution | Reference |
|-------|----------|-----------|
| Error 71 Wayland crash | `GDK_BACKEND=x11` (auto-fixed) | [WAYLAND.md](./WAYLAND.md) |
| GBM buffer error | `WEBKIT_DISABLE_COMPOSITING_MODE=1` (auto-fixed) | [WAYLAND.md](./WAYLAND.md) |
| Blank window | Disable compositing | Above |
| Build errors | Update deps, clean build | Above |
| IPC failures | Check command registration | Above |
| Dev mode fails | Set `GDK_BACKEND=x11` | Above |

## Environment Variables Summary

Useful environment variables for debugging:

```bash
# Backend selection
GDK_BACKEND=x11                          # Force X11
GDK_BACKEND=wayland                      # Force Wayland
GDK_BACKEND=wayland,x11                  # Wayland with X11 fallback

# WebKit rendering
WEBKIT_DISABLE_COMPOSITING_MODE=1        # Disable hardware compositing
WEBKIT_FORCE_COMPLEX_TEXT=1              # Force complex text rendering

# GTK debugging
GTK_DEBUG=all                            # Enable all GTK debug output
GDK_DEBUG=all                            # Enable all GDK debug output
G_MESSAGES_DEBUG=all                     # Enable all GLib messages

# Tauri debugging
RUST_LOG=debug                           # Enable debug logging
RUST_BACKTRACE=1                         # Show backtraces on panic
```

Use these in combination:
```bash
GDK_BACKEND=x11 WEBKIT_DISABLE_COMPOSITING_MODE=1 RUST_LOG=debug wraith-transfer
```
