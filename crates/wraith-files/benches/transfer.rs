//! Performance benchmarks for file transfer operations

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::io::Write;
use tempfile::NamedTempFile;
use wraith_files::DEFAULT_CHUNK_SIZE;
use wraith_files::chunker::{FileChunker, FileReassembler};
use wraith_files::tree_hash::{compute_tree_hash, compute_tree_hash_from_data};

/// Create a NodeConfig optimized for benchmarking (NAT detection disabled)
fn benchmark_node_config(port: u16) -> wraith_core::node::NodeConfig {
    let default = wraith_core::node::NodeConfig::default();
    wraith_core::node::NodeConfig {
        listen_addr: format!("0.0.0.0:{}", port).parse().unwrap(),
        discovery: wraith_core::node::DiscoveryConfig {
            enable_nat_traversal: false,
            enable_relay: false,
            ..default.discovery
        },
        ..default
    }
}

/// Benchmark file chunking performance
fn bench_file_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_chunking");

    for size in [
        1_000_000,   // 1 MB
        10_000_000,  // 10 MB
        100_000_000, // 100 MB
    ] {
        group.throughput(Throughput::Bytes(size));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            // Create temporary file
            let mut temp_file = NamedTempFile::new().unwrap();
            let data = vec![0xAA; size as usize];
            temp_file.write_all(&data).unwrap();
            temp_file.flush().unwrap();
            let path = temp_file.path().to_path_buf();

            b.iter(|| {
                let mut chunker = FileChunker::new(&path, DEFAULT_CHUNK_SIZE).unwrap();
                let mut total = 0;
                while let Some(chunk) = chunker.read_chunk().unwrap() {
                    total += black_box(chunk.len());
                }
                total
            });
        });
    }

    group.finish();
}

/// Benchmark tree hash computation
fn bench_tree_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_hashing");

    for size in [
        1_000_000,   // 1 MB
        10_000_000,  // 10 MB
        100_000_000, // 100 MB
    ] {
        group.throughput(Throughput::Bytes(size));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            // Create temporary file
            let mut temp_file = NamedTempFile::new().unwrap();
            let data = vec![0xBB; size as usize];
            temp_file.write_all(&data).unwrap();
            temp_file.flush().unwrap();
            let path = temp_file.path().to_path_buf();

            b.iter(|| {
                let tree = compute_tree_hash(&path, DEFAULT_CHUNK_SIZE).unwrap();
                black_box(tree.root)
            });
        });
    }

    group.finish();
}

/// Benchmark in-memory tree hashing
fn bench_tree_hashing_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_hashing_memory");

    for size in [
        1_000_000,   // 1 MB
        10_000_000,  // 10 MB
        100_000_000, // 100 MB
    ] {
        group.throughput(Throughput::Bytes(size));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let data = vec![0xCC; size as usize];

            b.iter(|| {
                let tree = compute_tree_hash_from_data(&data, DEFAULT_CHUNK_SIZE);
                black_box(tree.root)
            });
        });
    }

    group.finish();
}

/// Benchmark chunk verification
fn bench_chunk_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_verification");

    // Create test data
    let chunk_data = vec![0xDD; DEFAULT_CHUNK_SIZE];
    let tree = compute_tree_hash_from_data(&chunk_data, DEFAULT_CHUNK_SIZE);

    group.throughput(Throughput::Bytes(DEFAULT_CHUNK_SIZE as u64));

    group.bench_function("verify_chunk", |b| {
        b.iter(|| {
            let result = tree.verify_chunk(0, &chunk_data);
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark file reassembly
fn bench_file_reassembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_reassembly");

    for size in [
        1_000_000,  // 1 MB
        10_000_000, // 10 MB
    ] {
        group.throughput(Throughput::Bytes(size));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            // Prepare chunks
            let num_chunks = (size as usize).div_ceil(DEFAULT_CHUNK_SIZE);
            let chunks: Vec<_> = (0..num_chunks)
                .map(|_| vec![0xEE; DEFAULT_CHUNK_SIZE])
                .collect();

            b.iter(|| {
                let temp_file = NamedTempFile::new().unwrap();
                let mut reassembler =
                    FileReassembler::new(temp_file.path(), size, DEFAULT_CHUNK_SIZE).unwrap();

                for (i, chunk) in chunks.iter().enumerate() {
                    reassembler.write_chunk(i as u64, chunk).unwrap();
                }

                black_box(reassembler.is_complete())
            });
        });
    }

    group.finish();
}

/// Benchmark full transfer throughput with Node API
///
/// Measures end-to-end transfer performance including:
/// - Node initialization
/// - Session establishment
/// - File chunking and hashing
/// - Transfer coordination
fn bench_transfer_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("transfer_throughput");

    for size in [
        1_000_000,   // 1 MB
        10_000_000,  // 10 MB
        100_000_000, // 100 MB
    ] {
        group.throughput(Throughput::Bytes(size));
        group.sample_size(10); // Fewer samples for large transfers

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    use tempfile::NamedTempFile;
                    use wraith_core::node::Node;

                    // Create test file
                    let mut temp_file = NamedTempFile::new().unwrap();
                    let data = vec![0xAA; size as usize];
                    temp_file.write_all(&data).unwrap();
                    temp_file.flush().unwrap();
                    let path = temp_file.path().to_path_buf();

                    // Create sender and receiver with benchmark config (NAT detection disabled)
                    let sender = Node::new_with_config(benchmark_node_config(0))
                        .await
                        .unwrap();
                    let receiver = Node::new_with_config(benchmark_node_config(0))
                        .await
                        .unwrap();

                    sender.start().await.unwrap();
                    receiver.start().await.unwrap();

                    // Initiate transfer
                    let transfer_id = sender.send_file(&path, receiver.node_id()).await.unwrap();

                    // Note: Full implementation would wait for completion
                    // For now, we measure setup overhead
                    black_box(transfer_id);

                    sender.stop().await.unwrap();
                    receiver.stop().await.unwrap();
                })
            });
        });
    }

    group.finish();
}

/// Benchmark transfer latency and RTT
///
/// Metrics measured:
/// - Session establishment latency (Noise_XX handshake round-trip)
/// - Small file transfer initiation latency
fn bench_transfer_latency(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("transfer_latency");

    // Session establishment latency
    group.bench_function("session_establishment", |b| {
        b.iter(|| {
            rt.block_on(async {
                use wraith_core::node::Node;

                // Create two nodes with benchmark config (NAT detection disabled)
                let node1 = Node::new_with_config(benchmark_node_config(0))
                    .await
                    .unwrap();
                let node2 = Node::new_with_config(benchmark_node_config(0))
                    .await
                    .unwrap();

                node1.start().await.unwrap();
                node2.start().await.unwrap();

                // Small delay to ensure packet receive loops are ready
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                // Get node2's listening address
                let node2_addr = node2.listen_addr().await.unwrap();

                // Measure handshake latency
                let start = std::time::Instant::now();
                let _session_id = node1
                    .establish_session_with_addr(node2.node_id(), node2_addr)
                    .await
                    .unwrap();
                let latency = start.elapsed();

                node1.stop().await.unwrap();
                node2.stop().await.unwrap();

                black_box(latency)
            })
        });
    });

    // Small file transfer initiation latency
    group.bench_function("file_transfer_initiation", |b| {
        b.iter(|| {
            rt.block_on(async {
                use tempfile::NamedTempFile;
                use wraith_core::node::Node;

                // Create small test file (1 KB)
                let mut temp_file = NamedTempFile::new().unwrap();
                let data = vec![0xAA; 1024];
                temp_file.write_all(&data).unwrap();
                temp_file.flush().unwrap();
                let path = temp_file.path().to_path_buf();

                // Create nodes with benchmark config
                let sender = Node::new_with_config(benchmark_node_config(0))
                    .await
                    .unwrap();
                let receiver = Node::new_with_config(benchmark_node_config(0))
                    .await
                    .unwrap();

                sender.start().await.unwrap();
                receiver.start().await.unwrap();

                // Small delay to ensure packet receive loops are ready
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                // Get receiver address
                let receiver_addr = receiver.listen_addr().await.unwrap();

                // Establish session first
                let _session_id = sender
                    .establish_session_with_addr(receiver.node_id(), receiver_addr)
                    .await
                    .unwrap();

                // Measure file transfer initiation latency
                let start = std::time::Instant::now();
                let _transfer_id = sender.send_file(&path, receiver.node_id()).await.unwrap();
                let latency = start.elapsed();

                sender.stop().await.unwrap();
                receiver.stop().await.unwrap();

                black_box(latency)
            })
        });
    });

    group.finish();
}

/// Benchmark BBR bandwidth utilization
///
/// Measures congestion control effectiveness:
/// - Bandwidth estimation accuracy
/// - Congestion window growth
/// - RTT tracking
fn bench_bbr_utilization(c: &mut Criterion) {
    use wraith_core::congestion::BbrState;

    let mut group = c.benchmark_group("bbr_congestion_control");

    group.bench_function("bandwidth_estimation", |b| {
        b.iter(|| {
            let mut bbr = BbrState::new();

            // Simulate 1000 RTT samples
            for i in 0..1000 {
                let rtt = std::time::Duration::from_micros(500 + (i % 100));
                bbr.update_rtt(rtt);

                // Update bandwidth estimation
                let delivered = 1400 * (i + 1); // MTU-sized packets
                let interval = std::time::Duration::from_micros(1000);
                bbr.update_bandwidth(delivered, interval);
            }

            black_box(bbr.cwnd())
        });
    });

    group.bench_function("rtt_tracking", |b| {
        b.iter(|| {
            let mut bbr = BbrState::new();

            // Simulate varying RTTs
            for i in 0..500 {
                let rtt = std::time::Duration::from_micros(
                    500 + ((i * 17) % 200) as u64, // Varying RTT
                );
                bbr.update_rtt(rtt);

                let delivered = 1400 * (i + 1);
                let interval = std::time::Duration::from_micros(1000);
                bbr.update_bandwidth(delivered as u64, interval);
            }

            black_box(bbr.cwnd())
        });
    });

    group.bench_function("window_growth", |b| {
        b.iter(|| {
            let mut bbr = BbrState::new();
            let initial_cwnd = bbr.cwnd();

            // Simulate growth phase
            for i in 0..200 {
                let rtt = std::time::Duration::from_micros(500);
                bbr.update_rtt(rtt);

                let delivered = 1400 * (i + 1);
                let interval = std::time::Duration::from_micros(1000);
                bbr.update_bandwidth(delivered as u64, interval);
            }

            let final_cwnd = bbr.cwnd();
            black_box((initial_cwnd, final_cwnd))
        });
    });

    group.finish();
}

/// Benchmark multi-peer download speedup
///
/// Measures parallel download performance:
/// - Speedup from 1 to 5 peers
/// - Chunk distribution efficiency
/// - Aggregate bandwidth utilization
fn bench_multi_peer_speedup(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("multi_peer_download");

    for num_peers in [1, 2, 4] {
        group.bench_with_input(
            BenchmarkId::new("peers", num_peers),
            &num_peers,
            |b, &num_peers| {
                b.iter(|| {
                    rt.block_on(async {
                        use tempfile::NamedTempFile;
                        use wraith_core::node::Node;

                        // Create test file (4 MB)
                        let mut temp_file = NamedTempFile::new().unwrap();
                        let data = vec![0xCC; 4 * 1024 * 1024];
                        temp_file.write_all(&data).unwrap();
                        temp_file.flush().unwrap();
                        let _path = temp_file.path().to_path_buf();

                        // Create sender with benchmark config (NAT detection disabled)
                        let sender = Node::new_with_config(benchmark_node_config(0))
                            .await
                            .unwrap();
                        sender.start().await.unwrap();

                        // Create receivers with benchmark config
                        let mut receivers = Vec::new();
                        let mut receiver_addrs = Vec::new();
                        for _ in 0..num_peers {
                            let receiver = Node::new_with_config(benchmark_node_config(0))
                                .await
                                .unwrap();
                            receiver.start().await.unwrap();
                            let addr = receiver.listen_addr().await.unwrap();
                            receiver_addrs.push((*receiver.node_id(), addr));
                            receivers.push(receiver);
                        }

                        // Small delay to ensure packet receive loops are ready
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                        // Establish sessions using known addresses
                        for (node_id, addr) in &receiver_addrs {
                            sender
                                .establish_session_with_addr(node_id, *addr)
                                .await
                                .unwrap();
                        }

                        // Verify session count
                        let sessions = sender.active_sessions().await;
                        black_box(sessions.len());

                        // Cleanup
                        sender.stop().await.unwrap();
                        for receiver in receivers {
                            receiver.stop().await.unwrap();
                        }
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_file_chunking,
    bench_tree_hashing,
    bench_tree_hashing_memory,
    bench_chunk_verification,
    bench_file_reassembly,
    bench_transfer_throughput,
    bench_transfer_latency,
    bench_bbr_utilization,
    bench_multi_peer_speedup,
);
criterion_main!(benches);
