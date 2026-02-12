//! Transport layer benchmarks for Phase 3
//!
//! Benchmarks UDP throughput, worker pool performance, and MTU discovery.
//!
//! Run with: `cargo bench --bench transport_benchmarks`

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;
use wraith_transport::mtu::MtuDiscovery;
use wraith_transport::udp::UdpTransport;
use wraith_transport::worker::{Task, WorkerConfig, WorkerPool};

/// Benchmark UDP send/receive throughput
fn bench_udp_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("udp_throughput");

    // Test different packet sizes
    for size in [512, 1024, 1280, 1500] {
        let data = vec![0xAA; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            // Set up server and client
            let server = UdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap()).unwrap();
            let server_addr = server.local_addr().unwrap();

            let client = UdpTransport::bind("127.0.0.1:0".parse::<SocketAddr>().unwrap()).unwrap();

            b.iter(|| {
                // Send packet
                let sent = client.send_to(data, server_addr).unwrap();
                black_box(sent);
            });
        });
    }

    group.finish();
}

/// Benchmark UDP round-trip latency
fn bench_udp_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("udp_latency");
    group.measurement_time(Duration::from_secs(10));

    let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let server_addr = server_socket.local_addr().unwrap();
    server_socket.set_nonblocking(true).unwrap();

    let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    client_socket.set_nonblocking(false).unwrap();
    client_socket
        .set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();

    // Spawn echo server
    std::thread::spawn(move || {
        let mut buf = [0u8; 2048];
        loop {
            if let Ok((size, from)) = server_socket.recv_from(&mut buf) {
                let _ = server_socket.send_to(&buf[..size], from);
            }
        }
    });

    // Give server time to start
    std::thread::sleep(Duration::from_millis(50));

    group.bench_function("round_trip_1280", |b| {
        let data = vec![0xBB; 1280];
        let mut recv_buf = vec![0u8; 2048];

        b.iter(|| {
            // Send
            client_socket.send_to(&data, server_addr).unwrap();

            // Receive
            if let Ok((size, _)) = client_socket.recv_from(&mut recv_buf) {
                black_box(size);
            }
        });
    });

    group.finish();
}

/// Benchmark worker pool task processing
fn bench_worker_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("worker_pool");

    // Test different worker counts
    for num_workers in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_workers),
            &num_workers,
            |b, &workers| {
                let config = WorkerConfig {
                    num_workers: workers,
                    queue_capacity: 1000,
                    pin_to_cpu: false,
                    numa_aware: false,
                    buffer_pool: None,
                };

                let pool = WorkerPool::new(config);

                b.iter(|| {
                    for i in 0..100 {
                        let task = Task::ProcessPacket {
                            data: vec![0; 1280],
                            source: i,
                        };
                        let _ = pool.submit(task);
                    }

                    // Wait for processing
                    std::thread::sleep(Duration::from_millis(10));
                });

                // Clean shutdown
                pool.shutdown();
            },
        );
    }

    group.finish();
}

/// Benchmark MTU discovery cache performance
fn bench_mtu_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("mtu_cache");

    group.bench_function("cache_lookup", |b| {
        let discovery = MtuDiscovery::new();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        b.iter(|| {
            let mtu = discovery.get_cached(&addr);
            black_box(mtu);
        });
    });

    group.finish();
}

/// Benchmark frame encoding (using wraith-core)
fn bench_frame_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_encoding");

    // Test different payload sizes
    for size in [256, 1024, 4096] {
        let payload = vec![0x42; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &payload, |b, payload| {
            b.iter(|| {
                // Simulate frame encoding overhead
                // In actual implementation, this would call wraith_core::Frame::encode()
                let mut frame = Vec::with_capacity(payload.len() + 64);
                frame.extend_from_slice(&[0u8; 28]); // Frame header
                frame.extend_from_slice(payload);
                frame.extend_from_slice(&[0u8; 16]); // Auth tag

                black_box(frame);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_udp_throughput,
    bench_udp_latency,
    bench_worker_pool,
    bench_mtu_cache,
    bench_frame_encoding
);
criterion_main!(benches);
