# WRAITH iOS Client

Native iOS application for WRAITH Protocol secure file transfer.

## Overview

This client provides a native iOS interface to the WRAITH protocol, using SwiftUI for the UI and Rust (via UniFFI) for the protocol implementation.

## Architecture

- **SwiftUI**: Native iOS UI with modern declarative syntax
- **UniFFI**: Automatic Swift bindings from Rust code
- **Async/Await**: Native Swift concurrency for async operations
- **MVVM Pattern**: Clean separation of concerns

## Requirements

- iOS 16.0+
- Xcode 15.0+
- Rust toolchain with iOS targets
- cargo-lipo for universal library building

## Building

### Setup

1. Install Rust iOS targets:
```bash
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim
```

2. Install cargo-lipo:
```bash
cargo install cargo-lipo
```

### Build Framework

Build the Rust library as an XCFramework:

```bash
cd wraith-swift-ffi

# Build for all iOS targets
cargo lipo --release

# Generate Swift bindings
cargo run --bin uniffi-bindgen generate src/wraith.udl --language swift --out-dir ../WraithiOS/Generated
```

### Build iOS App

Open the project in Xcode:
```bash
open WraithiOS.xcodeproj
```

Or build from command line:
```bash
xcodebuild -scheme WraithiOS -configuration Release build
```

## Features

- **P2P File Transfer**: Send and receive files directly between devices
- **Session Management**: Establish secure sessions with peers
- **Background Transfers**: Transfers continue when app is backgrounded
- **Native SwiftUI**: Modern iOS UI with system design language
- **Share Extension**: Share files from other apps (planned)

## Project Structure

```
wraith-ios/
├── wraith-swift-ffi/          # Rust FFI library
│   ├── src/
│   │   ├── lib.rs             # UniFFI bindings
│   │   ├── error.rs           # Error types
│   │   └── wraith.udl         # UniFFI interface definition
│   ├── Cargo.toml             # Rust library config
│   └── build.rs               # UniFFI scaffolding generation
├── WraithiOS/                 # iOS application
│   └── Sources/
│       ├── WraithApp.swift    # App entry point + state
│       └── Views/             # SwiftUI views
│           ├── ContentView.swift
│           ├── HomeView.swift
│           ├── TransfersView.swift
│           ├── SessionsView.swift
│           └── SettingsView.swift
├── Package.swift              # Swift Package Manager config
└── README.md                  # This file
```

## Usage Example

```swift
// Create app state
let appState = AppState()

// Start the node
appState.startNode(listenAddr: "0.0.0.0:0")

// Establish a session
appState.establishSession(peerId: "...")

// Send a file
appState.sendFile(
    peerId: session.peerId,
    filePath: "/path/to/file.pdf"
)

// Shutdown
appState.shutdownNode()
```

## UniFFI Integration

The iOS client uses [UniFFI](https://mozilla.github.io/uniffi-rs/) to automatically generate Swift bindings from Rust code. The interface is defined in `wraith.udl` and automatically generates:

- Swift types matching Rust structs/enums
- Swift protocol for the WraithNode interface
- Error handling with Swift Error protocol
- Async/await support for Rust async functions

## Architecture Details

### State Management

The app uses SwiftUI's `@StateObject` and `@EnvironmentObject` for state management:

- **AppState**: Root state object containing node instance and collections
- **Published Properties**: Automatically update UI when state changes
- **MainActor**: All state updates on main thread for UI safety

### Error Handling

Rust errors are automatically converted to Swift errors via UniFFI:

```swift
do {
    try node.start(listenAddr: "0.0.0.0:0")
} catch WraithError.InitializationFailed(let message) {
    print("Failed to start: \(message)")
}
```

### Threading

- **Rust**: Tokio runtime for async operations
- **Swift**: MainActor for UI updates, background queue for heavy operations
- **UniFFI**: Automatic thread safety between Rust and Swift

## Testing

```bash
# Build and test Rust library
cd wraith-swift-ffi
cargo test

# Run iOS app tests
xcodebuild test -scheme WraithiOS -destination 'platform=iOS Simulator,name=iPhone 15'
```

## Future Enhancements

1. **Share Extension**: Share files from other apps
2. **Background Tasks**: Continue transfers when app is terminated
3. **Push Notifications**: Notify user of incoming transfers
4. **iCloud Sync**: Sync settings across devices
5. **Widget**: Quick status widget for home screen

## License

MIT OR Apache-2.0
