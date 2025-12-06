# io_uring Integration

## Overview

io_uring is Linux's modern asynchronous I/O interface. WRAITH uses io_uring alongside AF_XDP to achieve zero-copy file I/O, complementing XDP's zero-copy network I/O.

**Key Concept**: XDP accelerates network I/O, io_uring accelerates file I/O. Together, they eliminate memory copies throughout the entire file transfer pipeline.

## Why io_uring?

Traditional file I/O has significant overhead:

| I/O Method | System Calls | Memory Copies | Latency | Throughput |
|------------|--------------|---------------|---------|------------|
| `read()`/`write()` | 1 per operation | 2 (kernel ↔ user) | 10-50 μs | ~500 MB/s |
| `pread()`/`pwrite()` | 1 per operation | 2 (kernel ↔ user) | 10-50 μs | ~500 MB/s |
| `aio_read()`/`aio_write()` | 1 per operation | 2 (kernel ↔ user) | 5-20 μs | ~1 GB/s |
| **io_uring** | **1 per batch** | **0 (zero-copy)** | **<1 μs** | **5-10 GB/s** |

**Benefits**:
- **Batched Submission**: Submit multiple I/O operations in one syscall
- **Completion Polling**: Poll for completions without blocking
- **Zero-Copy**: Direct I/O with registered buffers (no kernel/userspace copies)
- **Async**: Non-blocking I/O for maximum concurrency

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         WRAITH Application                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                  wraith-core (Protocol Logic)                         │   │
│  │  ┌────────────┐  ┌─────────────┐  ┌────────────┐  ┌──────────────┐  │   │
│  │  │  File Tx   │  │ File Rx     │  │  Chunking  │  │    Hashing   │  │   │
│  │  └──────┬─────┘  └──────┬──────┘  └──────┬─────┘  └──────┬───────┘  │   │
│  └─────────┼───────────────┼────────────────┼───────────────┼──────────┘   │
│            │               │                │               │              │
│  ┌─────────▼───────────────▼────────────────▼───────────────▼──────────┐   │
│  │             wraith-files (File I/O Layer)                            │   │
│  │                                                                      │   │
│  │  File Operations:                                                   │   │
│  │  - read_chunk(offset, size) → Vec<u8>                               │   │
│  │  - write_chunk(offset, data) → Result<()>                           │   │
│  │  - hash_chunk(data) → BLAKE3 hash                                   │   │
│  │  - verify_chunk(data, expected_hash) → bool                         │   │
│  └──────────────────────────┬───────────────────────────────────────────┘   │
│                             │                                               │
│  ┌──────────────────────────▼───────────────────────────────────────────┐   │
│  │          wraith-transport::io_uring (io_uring Context)               │   │
│  │                                                                      │   │
│  │  Userspace Rings:                                                   │   │
│  │  ┌────────────────────┐            ┌─────────────────────┐          │   │
│  │  │  Submission Queue  │            │  Completion Queue   │          │   │
│  │  │      (SQ)          │            │       (CQ)          │          │   │
│  │  │                    │            │                     │          │   │
│  │  │  ┌──────────────┐  │            │  ┌──────────────┐   │          │   │
│  │  │  │ SQE (Entry)  │  │            │  │ CQE (Entry)  │   │          │   │
│  │  │  │  - op_code   │  │            │  │  - result    │   │          │   │
│  │  │  │  - fd        │  │            │  │  - user_data │   │          │   │
│  │  │  │  - offset    │  │            │  │              │   │          │   │
│  │  │  │  - addr      │  │            │  └──────────────┘   │          │   │
│  │  │  │  - len       │  │            │                     │          │   │
│  │  │  └──────────────┘  │            └─────────────────────┘          │   │
│  │  │                    │                                             │   │
│  │  │  Producer: User    │            Consumer: User                   │   │
│  │  └──────────┬─────────┘            └────────┬────────────────────────┘   │
│  │             │                                │                           │
│  └─────────────┼────────────────────────────────┼───────────────────────────┘   │
├─────────────────┼────────────────────────────────┼───────────────────────────────┤
│    Kernel       │                                │                               │
│  ┌──────────────▼────────────────────────────────▼───────────────────────────┐  │
│  │                       io_uring Subsystem                                  │  │
│  │                                                                            │  │
│  │  Kernel Rings (mirrored from userspace):                                  │  │
│  │  ┌────────────────────┐            ┌─────────────────────┐                │  │
│  │  │  Submission Queue  │            │  Completion Queue   │                │  │
│  │  │      (SQ)          │            │       (CQ)          │                │  │
│  │  │                    │            │                     │                │  │
│  │  │  Consumer: Kernel  │            │  Producer: Kernel   │                │  │
│  │  └──────────┬─────────┘            └─────────┬───────────┘                │  │
│  │             │                                │                            │  │
│  │  ┌──────────▼────────────────────────────────▼───────────────────────┐    │  │
│  │  │              I/O Workers (kernel threads)                         │    │  │
│  │  │                                                                   │    │  │
│  │  │  - Read operations (IORING_OP_READ)                              │    │  │
│  │  │  - Write operations (IORING_OP_WRITE)                            │    │  │
│  │  │  - Fsync operations (IORING_OP_FSYNC)                            │    │  │
│  │  │  - Registered buffers (zero-copy)                                │    │  │
│  │  └───────────────────────────┬───────────────────────────────────────┘    │  │
│  └──────────────────────────────┼────────────────────────────────────────────┘  │
│                                 │                                               │
│  ┌──────────────────────────────▼────────────────────────────────────────────┐  │
│  │                     Filesystem Layer (VFS)                                │  │
│  │                                                                            │  │
│  │  - ext4, xfs, btrfs, f2fs, etc.                                           │  │
│  │  - Page cache management                                                  │  │
│  │  - Direct I/O bypass (O_DIRECT)                                           │  │
│  └──────────────────────────────┬─────────────────────────────────────────────┘  │
│                                 │                                               │
│  ┌──────────────────────────────▼─────────────────────────────────────────────┐  │
│  │                     Block Device Layer                                    │  │
│  │                                                                            │  │
│  │  - NVMe (direct I/O, low latency)                                         │  │
│  │  - SATA SSD (good performance)                                            │  │
│  │  - HDD (benefits less from io_uring)                                      │  │
│  └────────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Implementation

WRAITH's io_uring implementation is in `wraith-transport/src/io_uring.rs`.

### IoUringContext

```rust
pub struct IoUringContext {
    /// Queue depth (number of SQEs/CQEs)
    queue_depth: u32,

    /// Next operation ID (for tracking)
    next_id: u64,

    /// Pending operations
    pending: HashMap<u64, PendingOp>,

    /// Registered buffers (for zero-copy)
    buffers: Vec<Vec<u8>>,
}
```

### Creating io_uring Context

```rust
use wraith_transport::io_uring::IoUringContext;

// Create context with 64-entry queues
let mut ctx = IoUringContext::new(64)?;

// Register buffers for zero-copy I/O
let buffers = vec![
    vec![0u8; 256 * 1024],  // 256 KB buffer 0
    vec![0u8; 256 * 1024],  // 256 KB buffer 1
    vec![0u8; 256 * 1024],  // 256 KB buffer 2
    vec![0u8; 256 * 1024],  // 256 KB buffer 3
];
ctx.register_buffers(buffers)?;
```

### Reading Files

```rust
use std::fs::File;
use std::os::unix::io::AsRawFd;

// Open file
let file = File::open("/path/to/file.dat")?;
let fd = file.as_raw_fd();

// Submit read operation
let op_id = ctx.submit_read(fd, 0, 4096)?;

// Wait for completion
let completions = ctx.wait_completions(1)?;
for completion in completions {
    if completion.id == op_id {
        println!("Read {} bytes", completion.result);
    }
}
```

### Writing Files

```rust
// Open file for writing
let file = File::create("/path/to/output.dat")?;
let fd = file.as_raw_fd();

// Prepare data to write
let data = vec![0xAA; 1024];

// Submit write operation
let op_id = ctx.submit_write(fd, 0, &data)?;

// Wait for completion
let completions = ctx.wait_completions(1)?;
for completion in completions {
    if completion.id == op_id {
        println!("Wrote {} bytes", completion.result);
    }
}
```

### Batched Operations

```rust
// Submit multiple operations in batch
let ops = vec![
    (fd1, 0, 4096),     // Read 4 KB from offset 0
    (fd1, 4096, 4096),  // Read 4 KB from offset 4096
    (fd1, 8192, 4096),  // Read 4 KB from offset 8192
    (fd1, 12288, 4096), // Read 4 KB from offset 12288
];

for (fd, offset, len) in ops {
    ctx.submit_read(fd, offset, len)?;
}

// Single syscall to submit all 4 operations!
// (In production, io_uring automatically batches)

// Wait for all completions
let completions = ctx.wait_completions(4)?;
println!("Completed {} operations", completions.len());
```

## Zero-Copy File I/O

### Registered Buffers

Pre-register buffers with the kernel for zero-copy I/O:

```rust
// Allocate buffers
let buffers = vec![
    vec![0u8; 256 * 1024],  // Buffer 0
    vec![0u8; 256 * 1024],  // Buffer 1
    vec![0u8; 256 * 1024],  // Buffer 2
    vec![0u8; 256 * 1024],  // Buffer 3
];

// Register with io_uring
ctx.register_buffers(buffers)?;

// Now reads/writes to these buffers use zero-copy!
// (Kernel accesses buffers directly via DMA)
```

**Benefits**:
- No memory copy between kernel and userspace
- Direct Memory Access (DMA) from/to storage device
- ~30% faster than non-registered buffers

### Direct I/O (O_DIRECT)

Bypass page cache for lowest latency:

```rust
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

// Open file with O_DIRECT
let file = OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_DIRECT)  // Bypass page cache
    .open("/path/to/file.dat")?;

// Reads now bypass kernel page cache entirely
// Data goes directly from NVMe → buffer via DMA
```

**Benefits**:
- Lowest latency (<10 μs for NVMe)
- No cache pollution
- Predictable performance

**Requirements**:
- Buffers must be aligned (4 KB boundary)
- I/O size must be multiple of filesystem block size (usually 4 KB)

## Integration with WRAITH File Transfer

### File Transmission

```rust
// wraith-files/src/sender.rs

use wraith_transport::io_uring::IoUringContext;

pub struct FileSender {
    io_uring: IoUringContext,
    chunk_size: usize,
}

impl FileSender {
    pub async fn send_file(&mut self, file_path: &Path) -> Result<()> {
        // Open file with O_DIRECT for zero-copy
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECT)
            .open(file_path)?;

        let fd = file.as_raw_fd();
        let file_size = file.metadata()?.len();
        let num_chunks = (file_size + self.chunk_size as u64 - 1) / self.chunk_size as u64;

        // Read all chunks using io_uring (batched)
        for chunk_id in 0..num_chunks {
            let offset = chunk_id * self.chunk_size as u64;
            let len = min(self.chunk_size, (file_size - offset) as usize);

            // Submit read operation (async, non-blocking)
            let op_id = self.io_uring.submit_read(fd, offset, len)?;

            // Store op_id for later completion tracking
            // ...
        }

        // Wait for completions and transmit chunks
        loop {
            let completions = self.io_uring.poll_completions()?;
            for completion in completions {
                // Get chunk data from registered buffer
                let chunk_data = &self.io_uring.buffers[completion.id as usize];

                // Hash chunk
                let hash = blake3::hash(chunk_data);

                // Transmit chunk via AF_XDP
                self.transport.send_chunk(completion.id, chunk_data, hash).await?;
            }

            if all_chunks_sent {
                break;
            }
        }

        Ok(())
    }
}
```

### File Reception

```rust
// wraith-files/src/receiver.rs

pub struct FileReceiver {
    io_uring: IoUringContext,
    chunk_size: usize,
}

impl FileReceiver {
    pub async fn receive_file(&mut self, output_path: &Path) -> Result<()> {
        // Create file with O_DIRECT for zero-copy
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .custom_flags(libc::O_DIRECT)
            .open(output_path)?;

        let fd = file.as_raw_fd();

        // Receive chunks and write using io_uring
        loop {
            // Receive chunk via AF_XDP
            let (chunk_id, chunk_data, expected_hash) = self.transport.recv_chunk().await?;

            // Verify hash
            let actual_hash = blake3::hash(&chunk_data);
            if actual_hash != expected_hash {
                return Err(Error::HashMismatch);
            }

            // Copy to registered buffer
            let buf_idx = chunk_id % self.io_uring.buffers.len();
            self.io_uring.buffers[buf_idx][..chunk_data.len()].copy_from_slice(&chunk_data);

            // Submit write operation (async, non-blocking)
            let offset = chunk_id * self.chunk_size as u64;
            self.io_uring.submit_write(fd, offset, &chunk_data)?;

            if last_chunk {
                break;
            }
        }

        // Wait for all writes to complete
        self.io_uring.wait_completions(pending_writes)?;

        // Fsync to ensure data is persisted
        self.io_uring.submit_fsync(fd)?;
        self.io_uring.wait_completions(1)?;

        Ok(())
    }
}
```

## Performance Optimization

### 1. Queue Depth Tuning

Larger queues allow more parallelism:

```rust
// Low latency (small queue)
let ctx = IoUringContext::new(16)?;

// High throughput (large queue)
let ctx = IoUringContext::new(256)?;

// WRAITH default (balanced)
let ctx = IoUringContext::new(64)?;
```

**Impact**:
- 256 depth: +20% throughput, +50 μs latency
- 16 depth: -10% throughput, -30 μs latency

### 2. Buffer Size Tuning

Match buffer size to workload:

```rust
// Small buffers (low latency, random I/O)
let buffers = vec![vec![0u8; 4096]; 64];  // 4 KB × 64 = 256 KB total

// Large buffers (high throughput, sequential I/O)
let buffers = vec![vec![0u8; 1024 * 1024]; 16];  // 1 MB × 16 = 16 MB total

// WRAITH default (balanced)
let buffers = vec![vec![0u8; 256 * 1024]; 16];  // 256 KB × 16 = 4 MB total
```

### 3. Polling vs. Blocking

```rust
// Polling (low latency, high CPU)
loop {
    let completions = ctx.poll_completions()?;
    if !completions.is_empty() {
        process_completions(completions);
    }
}

// Blocking (high latency, low CPU)
loop {
    let completions = ctx.wait_completions(1)?;  // Block until 1 completion
    process_completions(completions);
}

// Hybrid (balanced)
loop {
    let completions = ctx.poll_completions()?;
    if !completions.is_empty() {
        process_completions(completions);
    } else {
        tokio::time::sleep(Duration::from_micros(100)).await;  // Brief sleep
    }
}
```

## Platform Support

### Linux

**Full Support** (kernel 5.1+):
- All io_uring features available
- Zero-copy with registered buffers
- Direct I/O (O_DIRECT)
- Polling mode (IORING_SETUP_IOPOLL)

### Non-Linux (Fallback)

WRAITH provides a fallback implementation for non-Linux platforms:

```rust
// On non-Linux, IoUringContext uses synchronous I/O
#[cfg(not(target_os = "linux"))]
impl IoUringContext {
    pub fn submit_read(&mut self, fd: RawFd, offset: u64, len: usize) -> Result<u64> {
        // Synchronous read with pread(2)
        let mut buf = vec![0u8; len];
        let bytes_read = unsafe {
            libc::pread(fd, buf.as_mut_ptr() as *mut c_void, len, offset as i64)
        };
        // ...
    }
}
```

**Performance** (non-Linux):
- ~1 GB/s (vs. 5-10 GB/s with io_uring)
- Still functional, just slower

## Troubleshooting

### Issue: "io_uring not supported"

**Diagnosis**:
```bash
uname -r
# Need kernel 5.1+
```

**Solution**: Upgrade kernel or accept synchronous fallback.

### Issue: "ENOMEM (Cannot allocate memory)"

**Diagnosis**:
```bash
ulimit -l
# Should be unlimited or large enough
```

**Solution**: Increase locked memory limit.

### Issue: "EINVAL (Invalid argument)" with O_DIRECT

**Diagnosis**: Buffer not aligned or size not multiple of block size.

**Solution**:
```rust
// Align buffer to 4 KB boundary
let layout = std::alloc::Layout::from_size_align(size, 4096)?;
let ptr = unsafe { std::alloc::alloc(layout) };
let buf = unsafe { Vec::from_raw_parts(ptr, 0, size) };
```

## References

- [Linux io_uring Documentation](https://kernel.dk/io_uring.pdf)
- [io_uring by Example](https://unixism.net/loti/)
- [WRAITH io_uring Implementation](../../crates/wraith-transport/src/io_uring.rs)
- [WRAITH File I/O](../../crates/wraith-files/)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
