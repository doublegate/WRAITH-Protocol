# WRAITH-Chat

Secure end-to-end encrypted messaging application built on the WRAITH protocol with Signal Protocol's Double Ratchet algorithm.

## Overview

WRAITH-Chat is a desktop messaging application that provides:

- End-to-end encryption with Double Ratchet (Signal Protocol)
- Encrypted local storage with SQLCipher
- Peer-to-peer communication via WRAITH protocol
- Cross-platform support (Windows, macOS, Linux)
- Modern React UI with Tailwind CSS

## Architecture

### Backend (Rust + Tauri 2.0)

- **Double Ratchet Crypto** (`src-tauri/src/crypto.rs`): Signal Protocol implementation
- **SQLCipher Database** (`src-tauri/src/database.rs`): Encrypted message storage
- **IPC Commands** (`src-tauri/src/commands.rs`): Tauri command handlers
- **State Management** (`src-tauri/src/state.rs`): Application state

### Frontend (React + TypeScript)

- **Zustand Stores**: State management for conversations, messages, contacts, and node
- **Tauri Bindings**: Type-safe IPC communication
- **React Components**: ConversationList, ChatView, MessageBubble
- **Tailwind CSS**: Modern, responsive styling

## Features

- 1:1 encrypted messaging
- Conversation management
- Contact management with safety numbers
- Message delivery status
- Read receipts
- Encrypted database (SQLCipher + AES-256)
- Double Ratchet forward secrecy
- Out-of-order message handling

## Building

### Prerequisites

- Rust 1.85+ (2024 Edition)
- Node.js 18+
- Tauri CLI

### Development

```bash
# Install frontend dependencies
cd frontend
npm install

# Run in development mode (starts both frontend and backend)
cd ..
cargo tauri dev
```

### Production Build

```bash
# Build for production
cargo tauri build

# Outputs:
# - Windows: src-tauri/target/release/bundle/msi/
# - macOS: src-tauri/target/release/bundle/macos/
# - Linux: src-tauri/target/release/bundle/appimage/
```

## Project Structure

```
wraith-chat/
├── src-tauri/                     # Rust backend
│   ├── src/
│   │   ├── main.rs                # Entry point
│   │   ├── lib.rs                 # Library setup
│   │   ├── crypto.rs              # Double Ratchet implementation
│   │   ├── database.rs            # SQLCipher database
│   │   ├── commands.rs            # Tauri IPC commands
│   │   └── state.rs               # Application state
│   ├── Cargo.toml                 # Rust dependencies
│   └── tauri.conf.json            # Tauri configuration
├── frontend/                      # React frontend
│   ├── src/
│   │   ├── components/            # React components
│   │   │   ├── ConversationList.tsx
│   │   │   ├── ChatView.tsx
│   │   │   └── MessageBubble.tsx
│   │   ├── stores/                # Zustand stores
│   │   │   ├── conversationStore.ts
│   │   │   ├── messageStore.ts
│   │   │   ├── contactStore.ts
│   │   │   └── nodeStore.ts
│   │   ├── lib/
│   │   │   └── tauri.ts           # Tauri bindings
│   │   ├── types/
│   │   │   └── index.ts           # TypeScript types
│   │   ├── App.tsx                # Main app component
│   │   ├── main.tsx               # Entry point
│   │   └── index.css              # Global styles
│   ├── package.json               # Node dependencies
│   ├── vite.config.ts             # Vite configuration
│   ├── tailwind.config.js         # Tailwind configuration
│   └── index.html                 # HTML entry
└── README.md                      # This file
```

## Security

### Cryptography

- **Key Exchange**: X25519 Diffie-Hellman
- **AEAD**: ChaCha20-Poly1305
- **KDF**: HKDF-SHA256
- **Ratchet**: Double Ratchet (Signal Protocol)

### Database

- **Encryption**: SQLCipher with AES-256
- **KDF**: PBKDF2-HMAC-SHA512 (64,000 iterations)
- **Page Size**: 4096 bytes

### Forward Secrecy

The Double Ratchet algorithm provides:
- **Forward secrecy**: Past messages remain secure even if keys are compromised
- **Post-compromise security**: Future messages are secure after key compromise
- **Out-of-order handling**: Messages can arrive in any order

## Usage

1. **Start the application**
2. **Node initialization**: The WRAITH node starts automatically
3. **Create conversations**: Add contacts and start conversations
4. **Send messages**: Messages are encrypted end-to-end with Double Ratchet
5. **Receive messages**: Incoming messages are decrypted and stored locally

## Testing

```bash
# Run Rust tests
cd src-tauri
cargo test

# Run frontend tests (if configured)
cd frontend
npm test
```

## Future Enhancements

- Group messaging with Sender Keys
- Media attachments (images, videos, files)
- Voice messages
- Voice/video calls (WebRTC over WRAITH)
- Push notifications
- Disappearing messages
- Message search
- Export/import encrypted backups
- Mobile clients (iOS/Android)

## License

MIT OR Apache-2.0

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for contribution guidelines.
