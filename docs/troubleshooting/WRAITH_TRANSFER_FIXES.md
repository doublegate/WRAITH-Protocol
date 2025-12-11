# WRAITH Transfer - Common Issues and Fixes

**Document Version:** 1.0.0
**Last Updated:** 2025-12-11
**Application Version:** 0.1.0+

---

## Overview

This document covers common issues encountered in WRAITH Transfer desktop application and their solutions.

---

## Issue 1: NAT Detection Timeout Warning

### Symptom
When clicking "Start Node" button, terminal displays:
```
Warning: NAT detection failed (STUN server timeout), continuing in local mode
Note: This is not fatal - the node will still function for local connections
```

### Cause
- STUN servers (used for NAT type detection) may be unreachable
- Network firewall blocking UDP port 19302
- DNS resolution issues for Google STUN servers

### Resolution Status: FIXED (v1.5.8+)

**Changes Made:**
1. Updated STUN server IPs from invalid placeholders to Google Public STUN servers:
   - `74.125.250.129:19302` (stun.l.google.com)
   - `74.125.250.130:19302` (stun1.l.google.com)

2. Improved error messaging to clarify that NAT detection failure is non-fatal

3. Node continues to start in "local mode" with NAT type set to `Unknown`

**Code Location:**
- `/crates/wraith-discovery/src/nat/types.rs` (lines 103-123)
- `/crates/wraith-discovery/src/manager.rs` (lines 15-23, 246-259)

### Impact
- Node still starts and functions correctly for local network transfers
- Direct connections and relay-based connections remain available
- Only affects automatic NAT traversal optimization

### Workarounds
If you still experience issues:
1. Ensure UDP port 19302 is not blocked by firewall
2. Configure custom STUN servers via NodeConfig
3. Use relay servers for connections (auto-fallback)

---

## Issue 2: Node ID Not Copyable

### Symptom
Node ID displayed as truncated text (`{first8chars}...`) in header, not clickable

### Cause
Missing click handler and clipboard API integration

### Resolution Status: FIXED (v1.5.8+)

**Changes Made:**
1. Added click-to-copy functionality using `navigator.clipboard.writeText()`
2. Extended visible portion to show more context: `{first12chars}...{last4chars}`
3. Added hover effect to indicate clickability
4. Added tooltip showing full node ID on hover
5. Added visual feedback toast notification after copy ("Copied to clipboard!")

**Code Location:**
- `/clients/wraith-transfer/frontend/src/components/Header.tsx` (lines 3-81)

**How to Use:**
1. Start the node
2. Hover over the truncated node ID in the header
3. Tooltip will show the full 64-character hex node ID
4. Click on the node ID to copy to clipboard
5. Green toast notification confirms successful copy

---

## Issue 3: Browse Button Not Working

### Symptom
Clicking "Browse" button in New Transfer dialog does nothing

### Cause
1. Missing Tauri plugin permissions in `tauri.conf.json`
2. No error handling for file dialog failures

### Resolution Status: FIXED (v1.5.8+)

**Changes Made:**
1. Added `plugins` section to `tauri.conf.json` with proper permissions:
   ```json
   {
     "plugins": {
       "dialog": {
         "all": true,
         "open": true,
         "save": true
       },
       "fs": {
         "scope": ["$HOME/**"]
       }
     }
   }
   ```

2. Added error handling and logging to file selection function
3. Tauri dialog plugin already initialized in `lib.rs` (line 55)

**Code Location:**
- `/clients/wraith-transfer/src-tauri/tauri.conf.json` (lines 43-52)
- `/clients/wraith-transfer/frontend/src/components/NewTransferDialog.tsx` (lines 17-33)
- `/clients/wraith-transfer/src-tauri/src/lib.rs` (line 55)

**How to Use:**
1. Click "New Transfer" button
2. Click "Browse" button in the dialog
3. Native file picker dialog should appear
4. Select a file and click "Open"
5. File path should populate in the dialog

### Debugging
If browse button still doesn't work:
1. Check browser console for errors: `Ctrl+Shift+I` (or `Cmd+Opt+I` on macOS)
2. Look for TypeScript/JavaScript errors
3. Verify file picker has permissions to access $HOME directory
4. Check Tauri dev tools for IPC errors

---

## General Troubleshooting

### Application Won't Start
1. Check that port is not already in use (default: auto-assigned)
2. Verify all dependencies are installed: `cargo build -p wraith-transfer`
3. Check system logs for crash reports

### Wayland/X11 Issues (Linux)
The application automatically handles Wayland Error 71 on KDE Plasma 6 by falling back to X11 backend.

**Affected Systems:**
- KDE Plasma 6 on Wayland

**Automatic Fix:**
- Application detects KDE Plasma + Wayland and sets `GDK_BACKEND=x11`
- Also sets `WEBKIT_DISABLE_COMPOSITING_MODE=1` for GBM compatibility

**Manual Override:**
If you want to force a specific backend:
```bash
export GDK_BACKEND=wayland  # Force Wayland
export GDK_BACKEND=x11      # Force X11
```

**References:**
- Tauri Issue #10702: https://github.com/tauri-apps/tauri/issues/10702
- Tao Issue #977: https://github.com/tauri-apps/tao/issues/977

---

## Development Troubleshooting

### Frontend Build Issues
```bash
cd clients/wraith-transfer/frontend
npm install
npm run build
```

### Backend Build Issues
```bash
cd clients/wraith-transfer/src-tauri
cargo clean
cargo build
```

### Plugin Not Found Errors
Ensure plugins are initialized in `lib.rs`:
```rust
.plugin(tauri_plugin_dialog::init())
.plugin(tauri_plugin_fs::init())
```

---

## Performance Issues

### High CPU Usage
- Check for runaway progress polling intervals
- Verify file I/O is not blocking main thread
- Use browser DevTools Performance profiler

### High Memory Usage
- Large file transfers use memory-mapped I/O
- Check for memory leaks in frontend (React DevTools)
- Monitor transfer manager state cleanup

---

## See Also

- [WRAITH Transfer Architecture](../clients/wraith-transfer/architecture.md)
- [WRAITH Transfer Features](../clients/wraith-transfer/features.md)
- [Tauri Warnings Fix](TAURI_WARNINGS_FIX.md)
- [Tauri Warnings Summary](TAURI_WARNINGS_SUMMARY.md)

---

**Document Maintained By:** WRAITH Protocol Team
**Last Reviewed:** 2025-12-11
