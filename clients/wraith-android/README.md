# WRAITH Android Client

Android application for WRAITH Protocol secure file transfer.

## Overview

This client provides a native Android interface to the WRAITH protocol, using Kotlin with Jetpack Compose for the UI and Rust (via JNI) for the protocol implementation.

## Architecture

- **Kotlin/Jetpack Compose**: Modern Android UI with Material Design 3
- **Rust JNI**: Native protocol implementation via cargo-ndk
- **Coroutines**: Asynchronous operations and lifecycle management

## Requirements

- Android SDK 26+ (Android 8.0 Oreo)
- Rust toolchain with Android targets
- cargo-ndk for cross-compilation

## Building

### Setup

1. Install Android SDK and NDK:
```bash
# Install Android Studio or command-line tools
sdkmanager "platforms;android-34" "build-tools;34.0.0" "ndk;26.1.10909125"
```

2. Install Rust Android targets:
```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

3. Install cargo-ndk:
```bash
cargo install cargo-ndk
```

### Build APK

```bash
# Debug build
./gradlew assembleDebug

# Release build
./gradlew assembleRelease
```

Output APKs are in `app/build/outputs/apk/`.

## Features

- **P2P File Transfer**: Send and receive files directly between devices
- **Session Management**: Establish secure sessions with peers
- **Background Service**: Transfers continue when app is backgrounded
- **Material Design 3**: Modern Android UI
- **Notification Support**: Progress notifications for transfers

## Project Structure

```
wraith-android/
├── app/
│   ├── src/main/
│   │   ├── kotlin/com/wraith/android/
│   │   │   ├── MainActivity.kt          # Main activity
│   │   │   ├── WraithNative.kt          # JNI interface
│   │   │   ├── WraithClient.kt          # High-level Kotlin API
│   │   │   └── WraithService.kt         # Foreground service
│   │   ├── rust/
│   │   │   ├── Cargo.toml               # Rust library config
│   │   │   └── src/
│   │   │       ├── lib.rs               # JNI bindings
│   │   │       ├── error.rs             # Error types
│   │   │       └── types.rs             # Rust types
│   │   └── AndroidManifest.xml          # Manifest
│   └── build.gradle.kts                 # App build config
├── build.gradle.kts                     # Root build config
└── settings.gradle.kts                  # Project settings
```

## Usage Example

```kotlin
val wraith = WraithClient()

// Start the node
wraith.start(listenAddr = "0.0.0.0:0")

// Establish a session
val session = wraith.establishSession(peerId = "...")

// Send a file
val transfer = wraith.sendFile(
    peerId = session.peerId,
    filePath = "/sdcard/Download/file.pdf"
)

// Shutdown
wraith.shutdown()
```

## Testing

```bash
# Run unit tests
./gradlew test

# Run instrumented tests
./gradlew connectedAndroidTest
```

## License

MIT OR Apache-2.0
