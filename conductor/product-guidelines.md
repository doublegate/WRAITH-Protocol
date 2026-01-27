# Product Guidelines

## Documentation Style

- **Tone:** The documentation prose must be highly technical, rigorous, and academic. No emojis in any documentation or code comments.
- **Structure:** Prioritize formal specifications, mathematical proofs, and precise technical definitions over simplified metaphors. Use tables for structured data, code blocks for protocol formats, and ASCII diagrams for architecture.
- **Detail:** Explanations should assume a high level of domain knowledge in cryptography, systems programming, and network protocols, focusing on the correctness and security properties of the protocol.
- **Verification:** Implementation details must reference their theoretical basis where applicable:
  - **RFCs:** RFC 8445 (ICE), RFC 7748 (X25519), RFC 8439 (ChaCha20-Poly1305)
  - **Protocol Specifications:** Noise Protocol Framework (Noise_XX pattern), Kademlia DHT
  - **Algorithms:** BBR congestion control (Google), Elligator2 (Bernstein et al.), BLAKE3 (Aumasson et al.)
  - **Security Standards:** MITRE ATT&CK framework (for RedOps/Recon clients)
- **Code Documentation:** All public Rust APIs require `///` doc comments with `# Examples`, `# Errors`, `# Panics`, and `# Safety` sections as applicable. Module-level documentation uses `//!` comments with architecture diagrams.

## Visual Identity

- **Theme:** The visual language evokes a "Cyberpunk" or "Terminal-Inspired" aesthetic, reinforcing the protocol's focus on invisibility and security. All Tauri 2.0 desktop clients share this visual language.
- **Palette:** High-contrast color schemes with predominantly dark mode backgrounds and neon accents (terminal green, amber, cyan). Tailwind CSS provides the styling foundation for React frontends.
- **Typography:** Heavy reliance on monospace fonts for data display, hex dumps, connection IDs, public keys, and code. Proportional fonts reserved for prose and navigation.
- **Atmosphere:** The interface must feel like a professional, precision instrument -- dense, responsive, and stripped of unnecessary "consumer" fluff. Zustand state management ensures immediate UI feedback.
- **Client UI Consistency:** All Tauri desktop clients (Transfer, Chat, Sync, Share, Stream, Mesh, Publish, Vault, Recon, RedOps Operator) follow the same React + TypeScript + Tailwind CSS + Vite architecture with consistent component patterns.

## Brand Messaging

- **Core Pillars:** Security, Performance, Invisibility.
- **Voice:** Authoritative, uncompromising, and precise. Avoid marketing buzzwords; speak in measurable metrics (latency in microseconds, throughput in GiB/s, encryption strength in bits, test counts, vulnerability counts).
- **Mission:** To provide the most secure and performant decentralized file transfer fabric in existence.
- **Differentiators:** Six-layer protocol architecture, kernel-bypass acceleration, traffic analysis resistance, and a comprehensive client ecosystem spanning desktop, mobile, and server environments.
