# AF_XDP Architecture

## Overview

This document provides a detailed technical description of WRAITH's AF_XDP (Address Family XDP) implementation in the `wraith-transport` crate. AF_XDP enables zero-copy packet processing between the kernel and userspace, providing the foundation for WRAITH's high-performance network I/O.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         WRAITH Application                                  │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                  wraith-core (Protocol Logic)                         │   │
│  │  ┌────────────┐  ┌─────────────┐  ┌────────────┐  ┌──────────────┐  │   │
│  │  │   Frames   │  │  Sessions   │  │  Streams   │  │      BBR     │  │   │
│  │  └──────┬─────┘  └──────┬──────┘  └──────┬─────┘  └──────┬───────┘  │   │
│  └─────────┼───────────────┼────────────────┼───────────────┼──────────┘   │
│            │               │                │               │              │
│  ┌─────────▼───────────────▼────────────────▼───────────────▼──────────┐   │
│  │               wraith-transport (Transport Layer)                     │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │              AF_XDP Socket (`af_xdp.rs`)                       │  │   │
│  │  │                                                                │  │   │
│  │  │  Userspace Ring Buffers:                                      │  │   │
│  │  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐│  │   │
│  │  │  │  RX Ring   │  │  TX Ring   │  │ Fill Ring  │  │Comp Ring ││  │   │
│  │  │  │  (receive) │  │ (transmit) │  │  (free)    │  │(complete)││  │   │
│  │  │  │            │  │            │  │            │  │          ││  │   │
│  │  │  │  Consumer  │  │  Producer  │  │  Producer  │  │ Consumer ││  │   │
│  │  │  └──────┬─────┘  └─────┬──────┘  └─────┬──────┘  └────┬─────┘│  │   │
│  │  │         │               │                │              │      │  │   │
│  │  │         └───────────────┼────────────────┼──────────────┘      │  │   │
│  │  │                         │                │                     │  │   │
│  │  │  UMEM (Shared Memory):  │                │                     │  │   │
│  │  │  ┌──────────────────────▼────────────────▼──────────────────┐ │  │   │
│  │  │  │  Packet Buffer Pool (4 MB, 2048 frames of 2 KB each)     │ │  │   │
│  │  │  │                                                           │ │  │   │
│  │  │  │  ┌────────┐  ┌────────┐  ┌────────┐       ┌────────┐    │ │  │   │
│  │  │  │  │ Frame0 │  │ Frame1 │  │ Frame2 │  ...  │Frame2047│    │ │  │   │
│  │  │  │  │ 2KB    │  │ 2KB    │  │ 2KB    │       │ 2KB    │    │ │  │   │
│  │  │  │  └────────┘  └────────┘  └────────┘       └────────┘    │ │  │   │
│  │  │  │                                                           │ │  │   │
│  │  │  │  (Shared between kernel and userspace via mmap)          │ │  │   │
│  │  │  └───────────────────────────────────────────────────────────┘ │  │   │
│  │  │                                                                │  │   │
│  │  │  Socket Configuration:                                        │  │   │
│  │  │  - Queue ID: 0 (default)                                      │  │   │
│  │  │  - Interface: eth0 (configurable)                             │  │   │
│  │  │  - Flags: XDP_ZEROCOPY | XDP_USE_NEED_WAKEUP                  │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────────────────┤
│                        Linux Kernel Space                                   │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                     XDP Subsystem                                    │   │
│  │  ┌────────────────────────────────────────────────────────────────┐  │   │
│  │  │  XDP Program (eBPF - Future Implementation)                    │  │   │
│  │  │                                                                 │  │   │
│  │  │  Actions:                                                       │  │   │
│  │  │  - XDP_PASS: Pass packet to network stack                      │  │   │
│  │  │  - XDP_DROP: Drop packet immediately                           │  │   │
│  │  │  - XDP_TX: Transmit on same interface                          │  │   │
│  │  │  - XDP_REDIRECT: Redirect to AF_XDP socket                     │  │   │
│  │  └────────────────────────────────────────────────────────────────┘  │   │
│  │                                                                      │   │
│  │  Kernel Ring Buffers (mirrored from userspace):                     │   │
│  │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────────┐  │   │
│  │  │  RX Ring   │  │  TX Ring   │  │ Fill Ring  │  │  Comp Ring   │  │   │
│  │  │            │  │            │  │            │  │              │  │   │
│  │  │  Producer  │  │  Consumer  │  │  Consumer  │  │  Producer    │  │   │
│  │  └──────┬─────┘  └─────┬──────┘  └─────┬──────┘  └──────┬───────┘  │   │
│  │         │               │                │                │          │   │
│  └─────────┼───────────────┼────────────────┼────────────────┼──────────┘   │
│            │               │                │                │              │
│  ┌─────────▼───────────────▼────────────────▼────────────────▼──────────┐   │
│  │                    Network Driver (e1000, ixgbe, etc)                │   │
│  │                                                                       │   │
│  │  - RSS (Receive Side Scaling) for multi-queue distribution           │   │
│  │  - DMA (Direct Memory Access) to/from UMEM                           │   │
│  │  - Interrupt coalescing for batching                                 │   │
│  └───────────────────────────────────────────┬───────────────────────────┘   │
└───────────────────────────────────────────────┼───────────────────────────────┘
                                                │
                                     ┌──────────▼──────────┐
                                     │  Network Interface  │
                                     │    (eth0, etc)      │
                                     │                     │
                                     │  - Hardware RX/TX   │
                                     │  - Checksum offload │
                                     │  - Segmentation     │
                                     └─────────────────────┘
```

## UMEM (User Memory)

UMEM is the shared memory region that holds packet buffers. Both the kernel and userspace have direct access to this memory, enabling zero-copy packet processing.

### UMEM Structure

```rust
pub struct Umem {
    /// Configuration
    config: UmemConfig,

    /// Memory-mapped region (shared with kernel)
    buffer: *mut u8,

    /// Fill ring (userspace produces, kernel consumes)
    fill_ring: RingBuffer,

    /// Completion ring (kernel produces, userspace consumes)
    comp_ring: RingBuffer,
}
```

### UMEM Configuration

```rust
pub struct UmemConfig {
    /// Total UMEM size in bytes (default: 4 MB)
    pub size: usize,

    /// Size of each frame/chunk (default: 2048 bytes, must be power of 2)
    pub frame_size: usize,

    /// Headroom before packet data (default: 256 bytes for custom headers)
    pub headroom: usize,

    /// Fill ring size (default: 2048 entries, must be power of 2)
    pub fill_ring_size: u32,

    /// Completion ring size (default: 2048 entries, must be power of 2)
    pub comp_ring_size: u32,
}
```

### UMEM Memory Layout

```
UMEM (4 MB total):
┌────────────────────────────────────────────────────────────────┐
│ Frame 0 (2048 bytes):                                          │
│  ┌──────────┬───────────────────────┬──────────────────────┐   │
│  │ Headroom │     Packet Data       │      Padding         │   │
│  │ (256 B)  │     (up to 1792 B)    │  (to frame boundary) │   │
│  └──────────┴───────────────────────┴──────────────────────┘   │
├────────────────────────────────────────────────────────────────┤
│ Frame 1 (2048 bytes):                                          │
│  ┌──────────┬───────────────────────┬──────────────────────┐   │
│  │ Headroom │     Packet Data       │      Padding         │   │
│  └──────────┴───────────────────────┴──────────────────────┘   │
├────────────────────────────────────────────────────────────────┤
│ ...                                                            │
├────────────────────────────────────────────────────────────────┤
│ Frame 2047 (2048 bytes):                                       │
│  ┌──────────┬───────────────────────┬──────────────────────┐   │
│  │ Headroom │     Packet Data       │      Padding         │   │
│  └──────────┴───────────────────────┴──────────────────────┘   │
└────────────────────────────────────────────────────────────────┘

Total frames: 4 MB / 2048 B = 2048 frames
```

### UMEM Creation

```rust
let config = UmemConfig {
    size: 4 * 1024 * 1024,         // 4 MB
    frame_size: 2048,              // 2 KB per frame
    headroom: 256,                 // 256 B headroom
    fill_ring_size: 2048,          // 2048 fill descriptors
    comp_ring_size: 2048,          // 2048 completion descriptors
};

let umem = config.create()?;
```

UMEM is allocated using `mmap(2)` with:
- `PROT_READ | PROT_WRITE`: Read/write access
- `MAP_PRIVATE | MAP_ANONYMOUS`: Private anonymous mapping
- `MAP_POPULATE`: Prefault pages to avoid page faults later
- `MAP_LOCKED` (optional): Lock pages in RAM (requires CAP_IPC_LOCK)

## Ring Buffers

AF_XDP uses four lock-free ring buffers for communication between userspace and kernel:

### 1. RX Ring (Receive)

**Direction**: Kernel → Userspace

**Purpose**: Deliver received packets to userspace

**Producer**: Kernel (network driver)

**Consumer**: Userspace (WRAITH)

```rust
pub struct RxDescriptor {
    /// Address of packet in UMEM (offset from UMEM base)
    pub addr: u64,

    /// Length of received packet
    pub len: u32,

    /// Options (flags for packet processing)
    pub options: u32,
}
```

**Workflow**:
1. Kernel receives packet from NIC via DMA
2. Kernel writes packet to UMEM frame
3. Kernel produces descriptor to RX ring with frame address and length
4. Userspace consumes descriptor from RX ring
5. Userspace processes packet from UMEM
6. Userspace returns frame to Fill ring (reuse)

### 2. TX Ring (Transmit)

**Direction**: Userspace → Kernel

**Purpose**: Submit packets for transmission

**Producer**: Userspace (WRAITH)

**Consumer**: Kernel (network driver)

```rust
pub struct TxDescriptor {
    /// Address of packet in UMEM (offset from UMEM base)
    pub addr: u64,

    /// Length of packet to transmit
    pub len: u32,

    /// Options (flags for transmission)
    pub options: u32,
}
```

**Workflow**:
1. Userspace allocates frame from free pool
2. Userspace writes packet to UMEM frame
3. Userspace produces descriptor to TX ring with frame address and length
4. Kernel consumes descriptor from TX ring
5. Kernel transmits packet via NIC DMA
6. Kernel produces descriptor to Completion ring (frame free to reuse)

### 3. Fill Ring (Free Buffers)

**Direction**: Userspace → Kernel

**Purpose**: Provide free UMEM frames for incoming packets

**Producer**: Userspace (WRAITH)

**Consumer**: Kernel (network driver)

```rust
pub struct FillDescriptor {
    /// Address of free frame in UMEM (offset from UMEM base)
    pub addr: u64,
}
```

**Workflow**:
1. Userspace allocates free frames from pool
2. Userspace produces descriptors to Fill ring
3. Kernel consumes descriptors from Fill ring
4. Kernel uses frames for incoming packets
5. Kernel produces RX descriptors when packets arrive

**Critical**: The Fill ring must be kept populated! If the Fill ring is empty, incoming packets will be dropped.

### 4. Completion Ring (Transmission Complete)

**Direction**: Kernel → Userspace

**Purpose**: Notify userspace when transmitted packets are complete

**Producer**: Kernel (network driver)

**Consumer**: Userspace (WRAITH)

```rust
pub struct CompDescriptor {
    /// Address of completed frame in UMEM (offset from UMEM base)
    pub addr: u64,
}
```

**Workflow**:
1. Kernel completes packet transmission via NIC
2. Kernel produces descriptor to Completion ring
3. Userspace consumes descriptor from Completion ring
4. Userspace returns frame to free pool (available for reuse)

## Ring Buffer Implementation

All four rings use the same underlying lock-free ring buffer structure:

```rust
pub struct RingBuffer {
    /// Producer index (incremented by producer)
    producer: AtomicU32,

    /// Consumer index (incremented by consumer)
    consumer: AtomicU32,

    /// Ring size (must be power of 2)
    size: u32,

    /// Mask for wrapping (size - 1)
    mask: u32,

    /// Descriptor array (shared memory)
    descriptors: *mut Descriptor,
}
```

### Lock-Free Operations

**Produce (add descriptor)**:
```rust
pub fn produce(&mut self, desc: Descriptor) -> Result<(), RingError> {
    let producer = self.producer.load(Acquire);
    let consumer = self.consumer.load(Acquire);

    // Check if ring is full
    if producer - consumer >= self.size {
        return Err(RingError::Full);
    }

    // Calculate descriptor index (wrap around)
    let idx = producer & self.mask;

    // Write descriptor to ring
    unsafe {
        ptr::write_volatile(self.descriptors.add(idx as usize), desc);
    }

    // Publish new producer index
    self.producer.store(producer + 1, Release);

    Ok(())
}
```

**Consume (get descriptor)**:
```rust
pub fn consume(&mut self) -> Option<Descriptor> {
    let producer = self.producer.load(Acquire);
    let consumer = self.consumer.load(Acquire);

    // Check if ring is empty
    if producer == consumer {
        return None;
    }

    // Calculate descriptor index (wrap around)
    let idx = consumer & self.mask;

    // Read descriptor from ring
    let desc = unsafe {
        ptr::read_volatile(self.descriptors.add(idx as usize))
    };

    // Publish new consumer index
    self.consumer.store(consumer + 1, Release);

    Some(desc)
}
```

**Batch Operations**:
```rust
pub fn produce_batch(&mut self, descs: &[Descriptor]) -> Result<usize, RingError> {
    let mut produced = 0;
    for desc in descs {
        match self.produce(*desc) {
            Ok(()) => produced += 1,
            Err(RingError::Full) => break,
            Err(e) => return Err(e),
        }
    }
    Ok(produced)
}

pub fn consume_batch(&mut self, max: usize) -> Vec<Descriptor> {
    let mut descs = Vec::with_capacity(max);
    for _ in 0..max {
        match self.consume() {
            Some(desc) => descs.push(desc),
            None => break,
        }
    }
    descs
}
```

## AF_XDP Socket

The AF_XDP socket ties together UMEM and ring buffers:

```rust
pub struct AfXdpSocket {
    /// Configuration
    config: SocketConfig,

    /// UMEM region (shared memory)
    umem: Arc<Umem>,

    /// Socket file descriptor
    fd: RawFd,

    /// RX ring (kernel → userspace)
    rx_ring: RingBuffer,

    /// TX ring (userspace → kernel)
    tx_ring: RingBuffer,

    /// Free frame pool (for allocation)
    free_frames: Vec<u64>,
}
```

### Socket Configuration

```rust
pub struct SocketConfig {
    /// RX ring size (must be power of 2)
    pub rx_ring_size: u32,

    /// TX ring size (must be power of 2)
    pub tx_ring_size: u32,

    /// Socket flags (XDP_ZEROCOPY, XDP_COPY, XDP_USE_NEED_WAKEUP)
    pub flags: u32,

    /// Bind flags (XDP_SHARED_UMEM, XDP_COPY, XDP_ZEROCOPY)
    pub bind_flags: u32,
}
```

### Socket Creation

```rust
// 1. Create UMEM
let umem_config = UmemConfig::default();
let umem = umem_config.create()?;

// 2. Create AF_XDP socket
let socket_config = SocketConfig::default();
let socket = AfXdpSocket::new("eth0", 0, umem, socket_config)?;

// Socket is now ready to send/receive packets!
```

### Packet Reception

```rust
// Receive packets in batch
let packets = socket.rx_batch(32)?;

for pkt in packets {
    // Process packet
    println!("Received {} bytes at offset {}", pkt.len, pkt.addr);

    // Access packet data in UMEM
    let data = &umem.buffer[pkt.addr..pkt.addr + pkt.len];
    process_packet(data);

    // Return frame to Fill ring for reuse
    socket.fill_ring.produce(FillDescriptor { addr: pkt.addr })?;
}
```

### Packet Transmission

```rust
// Transmit packets in batch
let frames = socket.alloc_frames(10)?;  // Allocate 10 frames

for (i, frame_addr) in frames.iter().enumerate() {
    // Write packet to UMEM
    let packet_data = create_packet(i);
    let frame = &mut umem.buffer[*frame_addr..*frame_addr + packet_data.len()];
    frame.copy_from_slice(&packet_data);

    // Submit TX descriptor
    socket.tx_ring.produce(TxDescriptor {
        addr: *frame_addr,
        len: packet_data.len() as u32,
        options: 0,
    })?;
}

// Kick kernel to start transmission
socket.tx_kick()?;

// Wait for completions
let completed = socket.comp_batch(10)?;
for comp in completed {
    // Frame is now free for reuse
    socket.free_frames.push(comp.addr);
}
```

## Zero-Copy vs. Copy Mode

AF_XDP supports two modes:

### Zero-Copy Mode (XDP_ZEROCOPY)

**Advantages**:
- True zero-copy: Packets go directly from NIC to UMEM via DMA
- Maximum performance (10-40 Gbps single core)
- Minimal CPU overhead

**Requirements**:
- NIC driver support (ixgbe, i40e, ice, mlx5)
- Dedicated RX/TX queues (cannot share with kernel)
- Sufficient locked memory (`ulimit -l unlimited`)

**Setup**:
```rust
let config = SocketConfig {
    flags: XDP_ZEROCOPY | XDP_USE_NEED_WAKEUP,
    ..Default::default()
};
```

### Copy Mode (XDP_COPY)

**Advantages**:
- Works with all NICs
- Can share queues with kernel
- Lower privilege requirements

**Disadvantages**:
- One extra memory copy (still faster than standard sockets)
- Lower throughput (1-5 Gbps single core)

**Setup**:
```rust
let config = SocketConfig {
    flags: XDP_COPY | XDP_USE_NEED_WAKEUP,
    ..Default::default()
};
```

**WRAITH Behavior**: Attempts zero-copy first, falls back to copy mode if unavailable.

## Performance Optimization

### 1. Batch Processing

Process packets in batches to amortize syscall overhead:

```rust
// Bad: Process one packet at a time
for _ in 0..1000 {
    let pkt = socket.rx_batch(1)?;  // 1000 syscalls!
    process_packet(pkt[0]);
}

// Good: Process packets in batches
while total < 1000 {
    let packets = socket.rx_batch(32)?;  // ~32 syscalls
    for pkt in packets {
        process_packet(pkt);
    }
}
```

### 2. Ring Size Tuning

Larger rings reduce wakeups but increase latency:

```rust
// Low latency (small rings)
let config = SocketConfig {
    rx_ring_size: 512,
    tx_ring_size: 512,
    ..Default::default()
};

// High throughput (large rings)
let config = SocketConfig {
    rx_ring_size: 4096,
    tx_ring_size: 4096,
    ..Default::default()
};
```

### 3. CPU Affinity

Pin worker threads to specific CPUs for cache locality:

```rust
// Pin thread to CPU 0
let cpuset = nix::sched::CpuSet::from_slice(&[0]);
nix::sched::sched_setaffinity(nix::unistd::Pid::from_raw(0), &cpuset)?;
```

### 4. NUMA Awareness

Allocate UMEM on same NUMA node as NIC:

```rust
// Get NIC's NUMA node
let numa_node = get_nic_numa_node("eth0")?;

// Bind UMEM allocation to that node
set_numa_policy(numa_node)?;
let umem = umem_config.create()?;
```

### 5. Interrupt Coalescing

Reduce interrupt rate with NIC settings:

```bash
# Reduce interrupt rate (higher latency, higher throughput)
ethtool -C eth0 rx-usecs 50 tx-usecs 50

# Increase interrupt rate (lower latency, lower throughput)
ethtool -C eth0 rx-usecs 1 tx-usecs 1
```

## Error Handling

Common AF_XDP errors and handling:

```rust
match socket.rx_batch(32) {
    Ok(packets) => process_packets(packets),

    Err(AfXdpError::RingBufferError(msg)) => {
        // Ring full/empty - retry later
        log::warn!("Ring buffer error: {}", msg);
    }

    Err(AfXdpError::Io(io_err)) if io_err.kind() == io::ErrorKind::WouldBlock => {
        // No packets available - wait for wakeup
        socket.wait_for_rx()?;
    }

    Err(e) => {
        // Fatal error - fall back to UDP
        log::error!("AF_XDP error: {}, falling back to UDP", e);
        return Err(e);
    }
}
```

## Integration with WRAITH Protocol

WRAITH uses AF_XDP for the transport layer:

```rust
// wraith-core sends frame
let frame = wraith_core::Frame::new_data(stream_id, data);
let serialized = frame.serialize()?;

// wraith-transport transmits via AF_XDP
let frame_addr = socket.alloc_frame()?;
umem.write_frame(frame_addr, &serialized)?;
socket.tx(frame_addr, serialized.len())?;

// Receive side
let packets = socket.rx_batch(32)?;
for pkt in packets {
    let data = umem.read_frame(pkt.addr, pkt.len)?;
    let frame = wraith_core::Frame::parse(&data)?;
    wraith_core::process_frame(frame)?;
}
```

## Future Enhancements

### 1. eBPF Program Integration

Custom XDP programs for packet filtering:

```c
// wraith-xdp/src/xdp_filter.c

SEC("xdp")
int wraith_xdp_filter(struct xdp_md *ctx) {
    void *data = (void *)(long)ctx->data;
    void *data_end = (void *)(long)ctx->data_end;

    // Parse Ethernet header
    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end)
        return XDP_DROP;

    // Only process IPv4/IPv6
    if (eth->h_proto != htons(ETH_P_IP) && eth->h_proto != htons(ETH_P_IPV6))
        return XDP_PASS;

    // Redirect WRAITH packets to AF_XDP socket
    if (is_wraith_packet(data, data_end))
        return bpf_redirect_map(&xsks_map, ctx->rx_queue_index, 0);

    // Pass all other packets to kernel
    return XDP_PASS;
}
```

### 2. Multi-Queue Support

Distribute load across multiple RX/TX queues:

```rust
// Create AF_XDP socket per queue
let num_queues = get_nic_queue_count("eth0")?;
let sockets: Vec<AfXdpSocket> = (0..num_queues)
    .map(|queue_id| {
        let umem = create_per_queue_umem()?;
        AfXdpSocket::new("eth0", queue_id, umem, config.clone())
    })
    .collect::<Result<Vec<_>, _>>()?;

// One worker thread per queue
for (i, socket) in sockets.into_iter().enumerate() {
    std::thread::spawn(move || {
        pin_to_cpu(i);
        run_worker(socket);
    });
}
```

### 3. Hardware Offload

For NICs with XDP offload support (SmartNICs):

```rust
// Offload XDP program to NIC hardware
let prog = compile_xdp_program("wraith_filter.c")?;
load_xdp_offload("eth0", prog, XDP_FLAGS_HW_MODE)?;

// XDP program runs in NIC ASIC/FPGA - zero CPU overhead!
```

## References

- [Linux AF_XDP Documentation](https://www.kernel.org/doc/html/latest/networking/af_xdp.html)
- [XDP Hands-On Tutorial](https://github.com/xdp-project/xdp-tutorial)
- [AF_XDP Performance Analysis](https://dl.acm.org/doi/10.1145/3281411.3281443)
- [WRAITH AF_XDP Implementation](../../crates/wraith-transport/src/af_xdp.rs)
- [WRAITH Transport Layer](../../crates/wraith-transport/src/lib.rs)

---

**Last Updated**: 2025-12-06
**WRAITH Version**: 0.9.0 Beta
**Sprint**: 11.5 - XDP Documentation & CLI
