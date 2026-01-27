# Spectre Implant (WRAITH-RedOps)

**Spectre** is the advanced, lightweight agent for the WRAITH-RedOps platform. It is designed for maximum stealth, evasion, and resilience in contested environments.

## 🛠️ Technical Overview

*   **Language:** Rust (`no_std` environment)
*   **Target:** Windows (x64), Linux (x64/aarch64)
*   **Size:** < 200KB (Stripped & Optimized)
*   **Dependencies:** Minimal. Uses `wraith-crypto` for secure comms.

## 👻 Key Capabilities

*   **Native WRAITH Protocol:** Uses the WRAITH wire protocol (Noise_XX + XChaCha20-Poly1305) for all communications.
*   **Sleep Masking:** Obfuscates memory and call stack during sleep cycles to evade memory scanners.
*   **Heap Encryption:** Encrypts heap allocations when not in use.
*   **Indirect Syscalls:** Bypasses user-mode hooks (EDR/AV) by making direct system calls.
*   **API Hashing:** Resolves Windows APIs dynamically using compilation-time hashes, avoiding static import table (IAT) detection.
*   **Panic Safety:** Configured to `abort` silently on panic to prevent crash dumps or error dialogs.

## 🏗️ Build Instructions

### Prerequisites
*   **Rust Nightly:** Required for certain `no_std` and inline assembly features.
*   **Cross:** Recommended for cross-compilation.

### Building
The implant relies on release profiles for optimization.

```bash
# Debug Build (Larger, contains symbols)
cargo build

# Release Build (Stripped, Optimized for Size)
cargo build --release
```

### Configuration
Implant configuration (C2 URL, Campaign ID, Keys) is typically stamped into the binary during generation by the Team Server (`builder` module). For local testing, check `src/config.rs` (if applicable) or the `resources/` folder.

## 🧩 Project Structure

*   `src/`: Core logic.
    *   `c2/`: Command and Control transport logic.
    *   `modules/`: Capability implementations (File ops, Process injection, etc.).
    *   `utils/`: Helper functions (Hashing, Memory).
*   `resources/`: Static assets or templates.

## ⚠️ Evasion Note
While Spectre implements modern evasion techniques, it is an **educational tool** and should be tested against defenses in a controlled lab environment.
