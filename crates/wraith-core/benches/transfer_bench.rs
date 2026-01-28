//! Performance benchmarks for TransferSession optimizations.
//!
//! Run with: `cargo bench -p wraith-core transfer`
//!
//! These benchmarks verify the O(m) missing chunks optimization
//! and transfer session operations performance.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::PathBuf;
use wraith_core::transfer::TransferSession;

const CHUNK_SIZE: usize = 256 * 1024; // 256 KB

// ============================================================================
// Missing Chunks Performance Benchmarks
// ============================================================================

/// Benchmark missing_chunks() at various completion percentages
///
/// Verifies O(m) complexity where m is the number of missing chunks.
fn bench_transfer_missing_chunks(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer_missing_chunks");

    let total_chunks = 10_000u64; // 2.5 GB file
    let file_size = total_chunks * CHUNK_SIZE as u64;

    for completion_pct in [0, 50, 90, 95, 99, 100] {
        let chunks_to_transfer = (total_chunks as f64 * completion_pct as f64 / 100.0) as u64;
        let missing_count = total_chunks - chunks_to_transfer;

        group.throughput(Throughput::Elements(missing_count));
        group.bench_with_input(
            BenchmarkId::new("completion_pct", completion_pct),
            &completion_pct,
            |b, &pct| {
                // Setup session with partial completion
                let mut session = TransferSession::new_receive(
                    [1u8; 32],
                    PathBuf::from("/tmp/bench.dat"),
                    file_size,
                    CHUNK_SIZE,
                );

                let chunks_to_xfer = (total_chunks as f64 * pct as f64 / 100.0) as u64;
                for i in 0..chunks_to_xfer {
                    session.mark_chunk_transferred(i, CHUNK_SIZE);
                }

                b.iter(|| {
                    let missing = session.missing_chunks();
                    black_box(missing.len())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark missing_count() - O(1) operation
fn bench_transfer_missing_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer_missing_count");

    let total_chunks = 10_000u64;
    let file_size = total_chunks * CHUNK_SIZE as u64;

    for completion_pct in [0, 50, 99] {
        group.bench_with_input(
            BenchmarkId::new("completion_pct", completion_pct),
            &completion_pct,
            |b, &pct| {
                let mut session = TransferSession::new_receive(
                    [1u8; 32],
                    PathBuf::from("/tmp/bench.dat"),
                    file_size,
                    CHUNK_SIZE,
                );

                let chunks_to_xfer = (total_chunks as f64 * pct as f64 / 100.0) as u64;
                for i in 0..chunks_to_xfer {
                    session.mark_chunk_transferred(i, CHUNK_SIZE);
                }

                b.iter(|| {
                    let count = session.missing_count();
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark is_chunk_missing() - O(1) lookup
fn bench_transfer_is_chunk_missing(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer_is_chunk_missing");

    let total_chunks = 10_000u64;
    let file_size = total_chunks * CHUNK_SIZE as u64;

    // 50% completion
    let mut session = TransferSession::new_receive(
        [1u8; 32],
        PathBuf::from("/tmp/bench.dat"),
        file_size,
        CHUNK_SIZE,
    );

    for i in 0..total_chunks / 2 {
        session.mark_chunk_transferred(i, CHUNK_SIZE);
    }

    group.bench_function("check_missing", |b| {
        let mut idx = total_chunks / 2; // First missing chunk
        b.iter(|| {
            let is_missing = session.is_chunk_missing(idx);
            idx = (idx + 1) % total_chunks;
            black_box(is_missing)
        });
    });

    group.bench_function("check_transferred", |b| {
        let mut idx = 0u64; // First transferred chunk
        b.iter(|| {
            let is_missing = session.is_chunk_missing(idx);
            idx = (idx + 1) % (total_chunks / 2);
            black_box(is_missing)
        });
    });

    group.finish();
}

// ============================================================================
// Transfer Operations Benchmarks
// ============================================================================

/// Benchmark mark_chunk_transferred() performance
fn bench_mark_chunk_transferred(c: &mut Criterion) {
    let mut group = c.benchmark_group("mark_chunk_transferred");

    let total_chunks = 10_000u64;
    let file_size = total_chunks * CHUNK_SIZE as u64;

    group.throughput(Throughput::Elements(1));

    group.bench_function("mark_single", |b| {
        b.iter_batched(
            || {
                TransferSession::new_receive(
                    [1u8; 32],
                    PathBuf::from("/tmp/bench.dat"),
                    file_size,
                    CHUNK_SIZE,
                )
            },
            |mut session| {
                session.mark_chunk_transferred(0, CHUNK_SIZE);
                black_box(session.progress())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("mark_batch_100", |b| {
        b.iter_batched(
            || {
                TransferSession::new_receive(
                    [1u8; 32],
                    PathBuf::from("/tmp/bench.dat"),
                    file_size,
                    CHUNK_SIZE,
                )
            },
            |mut session| {
                for i in 0..100 {
                    session.mark_chunk_transferred(i, CHUNK_SIZE);
                }
                black_box(session.progress())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark progress calculation
fn bench_progress_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("progress_calculation");

    let total_chunks = 10_000u64;
    let file_size = total_chunks * CHUNK_SIZE as u64;

    let mut session = TransferSession::new_receive(
        [1u8; 32],
        PathBuf::from("/tmp/bench.dat"),
        file_size,
        CHUNK_SIZE,
    );

    // 50% complete
    for i in 0..total_chunks / 2 {
        session.mark_chunk_transferred(i, CHUNK_SIZE);
    }

    group.bench_function("progress", |b| {
        b.iter(|| {
            let progress = session.progress();
            black_box(progress)
        });
    });

    group.bench_function("transferred_count", |b| {
        b.iter(|| {
            let count = session.transferred_count();
            black_box(count)
        });
    });

    group.bench_function("bytes_transferred", |b| {
        b.iter(|| {
            let bytes = session.bytes_transferred();
            black_box(bytes)
        });
    });

    group.finish();
}

// ============================================================================
// Multi-Peer Coordination Benchmarks
// ============================================================================

/// Benchmark peer operations
fn bench_peer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("peer_operations");

    let total_chunks = 10_000u64;
    let file_size = total_chunks * CHUNK_SIZE as u64;

    group.bench_function("add_peer", |b| {
        b.iter_batched(
            || {
                TransferSession::new_receive(
                    [1u8; 32],
                    PathBuf::from("/tmp/bench.dat"),
                    file_size,
                    CHUNK_SIZE,
                )
            },
            |mut session| {
                for i in 0..10u8 {
                    let mut peer_id = [0u8; 32];
                    peer_id[0] = i;
                    session.add_peer(peer_id);
                }
                black_box(session.peer_count())
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Setup session with 5 peers
    let mut session = TransferSession::new_receive(
        [1u8; 32],
        PathBuf::from("/tmp/bench.dat"),
        file_size,
        CHUNK_SIZE,
    );

    for i in 0..5u8 {
        let mut peer_id = [0u8; 32];
        peer_id[0] = i;
        session.add_peer(peer_id);
    }

    let peer_id = [0u8; 32];

    group.bench_function("assign_chunk", |b| {
        let mut chunk_idx = 0u64;
        b.iter(|| {
            session.assign_chunk_to_peer(&peer_id, chunk_idx);
            chunk_idx = (chunk_idx + 1) % total_chunks;
            black_box(true)
        });
    });

    group.bench_function("next_chunk_to_request", |b| {
        b.iter(|| {
            let next = session.next_chunk_to_request();
            black_box(next)
        });
    });

    group.bench_function("assigned_chunks", |b| {
        b.iter(|| {
            let assigned = session.assigned_chunks();
            black_box(assigned.len())
        });
    });

    group.bench_function("aggregate_peer_speed", |b| {
        b.iter(|| {
            let speed = session.aggregate_peer_speed();
            black_box(speed)
        });
    });

    group.finish();
}

// ============================================================================
// Session Creation Benchmarks
// ============================================================================

/// Benchmark session creation (includes missing_chunks_set initialization)
fn bench_session_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_creation");

    for total_chunks in [100, 1_000, 10_000, 100_000] {
        let file_size = total_chunks as u64 * CHUNK_SIZE as u64;

        group.throughput(Throughput::Elements(total_chunks as u64));
        group.bench_with_input(
            BenchmarkId::new("chunks", total_chunks),
            &total_chunks,
            |b, _| {
                b.iter(|| {
                    let session = TransferSession::new_receive(
                        [1u8; 32],
                        PathBuf::from("/tmp/bench.dat"),
                        file_size,
                        CHUNK_SIZE,
                    );
                    black_box(session.total_chunks)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    missing_chunks_benches,
    bench_transfer_missing_chunks,
    bench_transfer_missing_count,
    bench_transfer_is_chunk_missing,
);

criterion_group!(
    transfer_ops_benches,
    bench_mark_chunk_transferred,
    bench_progress_calculation,
);

criterion_group!(peer_benches, bench_peer_operations,);

criterion_group!(creation_benches, bench_session_creation,);

criterion_main!(
    missing_chunks_benches,
    transfer_ops_benches,
    peer_benches,
    creation_benches,
);
