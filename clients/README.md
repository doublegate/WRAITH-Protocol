# WRAITH Client Applications

This directory contains reference implementations and production clients for the WRAITH Protocol.

## Directory Structure

```
clients/
├── README.md                 # This file
└── (Future client projects will be added here)
```

## Planned Clients

### Reference Client Foundation

#### wraith-ffi (In Progress - Sprint 15.1)
**Status:** Foundation Complete, Compilation Fixes Required
**Location:** `crates/wraith-ffi/`
**Purpose:** C-compatible FFI bindings for wraith-core Node API

**Completed:**
- Project structure created
- Cargo.toml with cdylib/staticlib/rlib configuration
- Core modules implemented:
  - `lib.rs` - Main FFI entry point with initialization
  - `types.rs` - FFI-safe type definitions (IDs, stats, progress, enums)
  - `error.rs` - Error handling with WraithErrorCode enum
  - `config.rs` - Configuration FFI (padding, timing, mimicry modes)
  - `node.rs` - Node API FFI (create, start, stop, save identity)
  - `session.rs` - Session API FFI (establish, close, stats)
  - `transfer.rs` - Transfer API FFI (send, wait, progress)
- cbindgen integration for C header generation
- Rust 2024 safety attributes (`#[unsafe(no_mangle)]`)

**Remaining Work:**
- Fix API mismatches with wraith-core Node API
- Fix NodeError enum pattern matching
- Add unsafe blocks for Rust 2024 compliance
- Complete test coverage
- Generate TypeScript bindings from C headers

#### wraith-transfer (Planned - Sprint 15.2-15.4)
**Technology:** Tauri 2.0 + React 18 + TypeScript 5+
**Purpose:** Cross-platform desktop file transfer client

**Planned Features:**
- File selection and drag-and-drop
- Transfer progress with speed/ETA display
- Peer connection management
- Settings panel for obfuscation configuration
- System tray integration
- Multi-platform: Windows 10+, macOS 11+, Linux (GTK3+)

## Development Roadmap

See [ROADMAP-clients.md](../to-dos/ROADMAP-clients.md) for the complete client ecosystem roadmap.

### Phase 15: Reference Client Foundation (Current)
- **Sprint 15.1:** Core Library Bindings (FFI) - **IN PROGRESS**
- **Sprint 15.2:** Tauri Desktop Shell - Planned
- **Sprint 15.3:** React UI Foundation - Planned
- **Sprint 15.4:** Transfer UI - Planned

### Future Phases
- **Phase 16:** Mobile Clients (wraith-mobile-flutter)
- **Phase 17:** CLI Enhancements (wraith-tui)
- **Phase 18:** Browser Extension (wraith-extension)
- **Phase 19-22:** Ecosystem Expansion (Server, Sync, Enterprise, Integration)

## Building

### wraith-ffi
```bash
# Build the FFI library
cargo build -p wraith-ffi

# Generate C headers (output to target/include/)
cargo build -p wraith-ffi --release

# Run tests
cargo test -p wraith-ffi
```

## Integration Examples

### Using wraith-ffi from C
```c
#include "wraith-ffi.h"

int main() {
    // Initialize library
    wraith_init();

    // Create node with default config
    WraithNode* node = wraith_node_new(NULL, NULL);

    // Start node
    wraith_node_start(node, NULL);

    // ... use node ...

    // Cleanup
    wraith_node_stop(node, NULL);
    wraith_node_free(node);

    return 0;
}
```

### Using wraith-ffi from Tauri (TypeScript)
```typescript
// TypeScript bindings will be generated from C headers
import { invoke } from '@tauri-apps/api/core';

async function initNode(): Promise<void> {
  await invoke('wraith_node_init');
  await invoke('wraith_node_start');
}
```

## Documentation

- [Reference Client Design](../docs/clients/REFERENCE_CLIENT.md) - UI/UX specification
- [Integration Guide](../docs/INTEGRATION_GUIDE.md) - wraith-core integration patterns
- [Sprint Planning](../to-dos/clients/wraith-transfer-sprints.md) - Detailed sprint breakdown

## License

MIT/Apache-2.0 (matching WRAITH Protocol)
