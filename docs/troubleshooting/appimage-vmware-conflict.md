# AppImage Build Failure with VMware Workstation

## Issue

When building AppImage bundles on systems with VMware Workstation installed, the build fails with errors like:

```
failed to run linuxdeploy
```

The underlying error involves `libffi.so.7` not being found:

```
/usr/lib/vmware/lib/libglib-2.0.so.0/libglib-2.0.so.0: error while loading shared libraries: libffi.so.7: cannot open shared object file: No such file or directory
```

## Root Cause

VMware Workstation installs its own bundled GTK/GLib libraries in `/usr/lib/vmware/lib/`. These libraries were compiled against older system dependencies, specifically `libffi.so.7`.

Modern Linux distributions (Fedora 39+, Ubuntu 22.04+, Arch Linux, etc.) ship with `libffi.so.8`, making VMware's bundled libraries incompatible.

During AppImage creation, the `linuxdeploy` tool with the GTK plugin recursively searches `/usr/lib` for GTK-related libraries. It discovers VMware's incompatible libraries and attempts to bundle them, causing the build to fail when these libraries cannot load their dependencies.

### Affected Libraries

VMware typically bundles these libraries in `/usr/lib/vmware/lib/`:

- `libgdk_pixbuf-2.0.so.0`
- `libglib-2.0.so.0`
- `libgio-2.0.so.0`
- `libgobject-2.0.so.0`
- `libgmodule-2.0.so.0`
- And various GTK-related libraries

## Workaround

Temporarily move VMware's library directory out of `/usr/lib` during AppImage builds:

```bash
# Before building AppImage
sudo mv /usr/lib/vmware/lib /opt/vmware-lib-backup

# Build the AppImage
cargo tauri build --target x86_64-unknown-linux-gnu

# Restore VMware libraries after build
sudo mv /opt/vmware-lib-backup /usr/lib/vmware/lib
```

### Alternative: Rename Instead of Move

```bash
# Before building
sudo mv /usr/lib/vmware/lib /usr/lib/vmware/lib.bak

# Build AppImage
cargo tauri build

# After building
sudo mv /usr/lib/vmware/lib.bak /usr/lib/vmware/lib
```

**Note:** Moving the directory completely out of `/usr/lib` (not just renaming) is more reliable, as some tools may still find `.bak` directories during recursive searches.

## Permanent Solutions

### Option 1: Build Script Automation

Create a build script that handles the workaround automatically:

```bash
#!/bin/bash
# scripts/build-appimage.sh

VMWARE_LIB="/usr/lib/vmware/lib"
BACKUP_LOCATION="/opt/vmware-lib-backup"

# Check if VMware libs exist and move them
if [ -d "$VMWARE_LIB" ]; then
    echo "Moving VMware libraries temporarily..."
    sudo mv "$VMWARE_LIB" "$BACKUP_LOCATION"
    RESTORE_VMWARE=true
fi

# Build AppImage
cargo tauri build --target x86_64-unknown-linux-gnu

BUILD_RESULT=$?

# Restore VMware libraries
if [ "$RESTORE_VMWARE" = true ]; then
    echo "Restoring VMware libraries..."
    sudo mv "$BACKUP_LOCATION" "$VMWARE_LIB"
fi

exit $BUILD_RESULT
```

### Option 2: CI/CD Environment

Build AppImages in a clean CI/CD environment (GitHub Actions, GitLab CI) where VMware Workstation is not installed. This is the recommended approach for release builds.

### Option 3: Container-Based Builds

Use a Docker container for AppImage builds:

```bash
docker run --rm -v $(pwd):/app -w /app rust:latest cargo tauri build
```

## Verification

After a successful build, the AppImage will be located at:

```
target/release/bundle/appimage/WRAITH Chat_<version>_amd64.AppImage
```

You can verify it works:

```bash
chmod +x "WRAITH Chat_1.7.0_amd64.AppImage"
./WRAITH\ Chat_1.7.0_amd64.AppImage
```

## Affected Systems

This issue affects:

- Any Linux system with VMware Workstation installed
- Systems running libffi 3.4+ (libffi.so.8)
- Tauri applications using GTK (all Linux desktop apps)

## Related Issues

- VMware Workstation library compatibility with modern Linux
- linuxdeploy GTK plugin recursive library discovery
- libffi ABI versioning (libffi.so.7 vs libffi.so.8)

## References

- [Tauri AppImage bundling documentation](https://tauri.app/distribute/appimage/)
- [linuxdeploy GTK plugin](https://github.com/linuxdeploy/linuxdeploy-plugin-gtk)
- VMware library bundling in `/usr/lib/vmware/lib/`
