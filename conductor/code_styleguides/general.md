# General Code Style Principles

This document outlines general coding principles that apply across all languages and frameworks used in the WRAITH Protocol project: Rust (protocol crates, Tauri backends, FFI), TypeScript (Tauri frontends), Kotlin (Android), and Swift (iOS).

For language-specific rules, see the dedicated style guides:
- [Rust](./rust.md)
- [TypeScript](./typescript.md)
- [JavaScript](./javascript.md)
- [HTML/CSS](./html-css.md)

## Readability

- Code should be easy to read and understand by humans. Cryptographic and networking code demands particular clarity due to the security implications of misunderstanding.
- Avoid overly clever or obscure constructs. Prefer explicit logic over implicit behavior, especially in protocol state machines and frame processing.
- Use descriptive names that convey intent. A variable named `ratchet_chain_key` is preferable to `rck`.

## Consistency

- Follow existing patterns in the codebase. When in doubt, look at analogous modules within the same crate.
- Maintain consistent formatting, naming, and structure across all crates in the workspace.
- Use the same error handling patterns within each crate (per-crate error enums via `thiserror`).

## Simplicity

- Prefer simple solutions over complex ones. Complexity in a security-critical protocol is a liability.
- Break down complex problems into smaller, manageable parts. Each module should have a single, well-defined responsibility.
- Minimize state and mutation. Prefer immutable data and transformations.

## Maintainability

- Write code that is easy to modify and extend. The protocol must evolve to address new threats and requirements.
- Minimize dependencies and coupling. Each crate should expose a clean public API and hide implementation details.
- All public APIs must have documentation. Internal functions should be documented when the logic is non-obvious.

## Documentation

- Document *why* something is done, not just *what*. The "what" is visible in the code; the "why" is often invisible.
- Keep documentation up-to-date with code changes. Stale documentation is worse than no documentation.
- Reference specifications and RFCs where applicable (e.g., "per RFC 8445 Section 5.1.2").
- No emojis in documentation, comments, or commit messages.

## Security Principles

- **Defense in depth:** Multiple layers of validation, encryption, and authentication.
- **Fail closed:** When uncertain, deny access or reject data.
- **Least privilege:** Minimize the attack surface of each component.
- **Constant-time operations:** All comparisons involving secret data must be constant-time.
- **Zeroize secrets:** Key material must be zeroed on drop.
- **No panics in library code:** Library crates must never panic; return `Result` instead.

## Performance Principles

- **Measure before optimizing:** Use benchmarks (`criterion`) to validate performance assumptions.
- **Zero-copy where possible:** Parse by reference into existing buffers; avoid unnecessary allocations.
- **Cache-friendly data structures:** Prefer contiguous memory layouts for frequently accessed data.
- **No allocations in hot paths:** Frame processing and encryption paths must be allocation-free.
