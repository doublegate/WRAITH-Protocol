# WRAITH-RedOps Protocol Definitions

This directory contains the **Protocol Buffers (`.proto`)** that define the API contract for the WRAITH-RedOps ecosystem. These definitions are the single source of truth for communication between the Team Server, Operator Client, and Implant.

## 📄 `redops.proto`

The primary definition file containing services and message types.

### Services

#### 1. `OperatorService`
Used by the **Operator Client** to manage the operation.
*   **Authentication:** `Authenticate`, `RefreshToken`
*   **Campaigns:** `CreateCampaign`, `ListCampaigns`, `GetCampaign`
*   **Implants:** `ListImplants`, `GetImplant`, `KillImplant`, `GenerateImplant`
*   **Commanding:** `SendCommand`, `ListCommands`, `GetCommandResult`, `CancelCommand`
*   **Events:** `StreamEvents` (Server-side streaming of real-time updates)
*   **Listeners:** `CreateListener`, `ListListeners`, `StartListener`, `StopListener`
*   **Logic:** `CreateAttackChain`, `InstantiatePlaybook`

#### 2. `ImplantService`
Used by **Spectre** (or redirectors) to communicate with the Team Server.
*   **Session:** `Register`, `CheckIn`
*   **Tasking:** `GetPendingCommands`
*   **Exfiltration:** `SubmitResult`, `UploadArtifact`
*   **Payloads:** `DownloadPayload`

### Data Models
Key data structures defined include:
*   `Campaign`: High-level operation grouping.
*   `Implant`: Represents a compromised host.
*   `Command`: A unit of work for an implant.
*   `RulesOfEngagement`: Scoping constraints (signed).
*   `AttackChain`: A sequence of automated steps.

## 🔨 Code Generation

These protos are compiled into native code for each component using `tonic-build` (Rust).

### Rust (Team Server & Operator Backend)
The `build.rs` script in both `team-server` and `operator-client` compiles these protos automatically.

```rust
// Example build.rs usage
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../proto/redops.proto")?;
    Ok(())
}
```

### TypeScript (Frontend - Optional)
If direct gRPC-web usage is implemented in the future, these would be compiled using `protoc-gen-ts`. Currently, the Tauri backend handles gRPC, so the frontend uses JSON-serialized Rust structs.
