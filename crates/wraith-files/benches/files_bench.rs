//! Performance benchmarks for wraith-files optimizations.
//!
//! Run with: `cargo bench -p wraith-files`
//!
//! These benchmarks verify the performance improvements from:
//! - O(m) missing chunks calculation (FileReassembler)
//! - Slice-based hashing (IncrementalTreeHasher)
//! - Pre-allocated Merkle tree computation

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::RngCore;
use std::hint::black_box;
use std::io::Write;
use tempfile::NamedTempFile;
use wraith_files::DEFAULT_CHUNK_SIZE;
use wraith_files::chunker::{FileChunker, FileReassembler};
use wraith_files::tree_hash::{
    IncrementalTreeHasher, compute_merkle_root, compute_tree_hash_from_data,
};

// ============================================================================
// FileReassembler Benchmarks (O(m) optimization)
// ============================================================================

/// Benchmark missing_chunks() for various completion percentages
///
/// This verifies that missing_chunks() is O(m) where m is the number of
/// missing chunks, not O(n) where n is total chunks.
fn bench_missing_chunks_by_completion(c: &mut Criterion) {
    let mut group = c.benchmark_group("missing_chunks_completion");

    let total_chunks = 10_000u64; // 2.5 GB file with 256 KB chunks
    let chunk_size = DEFAULT_CHUNK_SIZE;
    let total_size = total_chunks * chunk_size as u64;

    // Test at various completion percentages
    for completion_pct in [0, 50, 90, 95, 99, 100] {
        let chunks_to_receive = (total_chunks as f64 * completion_pct as f64 / 100.0) as u64;
        let missing_count = total_chunks - chunks_to_receive;

        group.throughput(Throughput::Elements(missing_count));
        group.bench_with_input(
            BenchmarkId::new("completion_pct", completion_pct),
            &completion_pct,
            |b, &pct| {
                // Setup: create reassembler and mark chunks as received
                let temp_file = NamedTempFile::new().unwrap();
                let mut reassembler =
                    FileReassembler::new(temp_file.path(), total_size, chunk_size).unwrap();

                let chunks_to_recv = (total_chunks as f64 * pct as f64 / 100.0) as u64;
                let dummy_data = vec![0u8; chunk_size];

                for i in 0..chunks_to_recv {
                    reassembler.write_chunk(i, &dummy_data).ok();
                }

                b.iter(|| {
                    let missing = reassembler.missing_chunks();
                    black_box(missing.len())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark missing_count() - should be O(1)
fn bench_missing_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("missing_count_o1");

    let total_chunks = 10_000u64;
    let chunk_size = DEFAULT_CHUNK_SIZE;
    let total_size = total_chunks * chunk_size as u64;

    for completion_pct in [0, 50, 99] {
        group.bench_with_input(
            BenchmarkId::new("completion_pct", completion_pct),
            &completion_pct,
            |b, &pct| {
                let temp_file = NamedTempFile::new().unwrap();
                let mut reassembler =
                    FileReassembler::new(temp_file.path(), total_size, chunk_size).unwrap();

                let chunks_to_recv = (total_chunks as f64 * pct as f64 / 100.0) as u64;
                let dummy_data = vec![0u8; chunk_size];

                for i in 0..chunks_to_recv {
                    reassembler.write_chunk(i, &dummy_data).ok();
                }

                b.iter(|| {
                    let count = reassembler.missing_count();
                    black_box(count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark is_chunk_missing() - should be O(1)
fn bench_is_chunk_missing(c: &mut Criterion) {
    let mut group = c.benchmark_group("is_chunk_missing_o1");

    let total_chunks = 10_000u64;
    let chunk_size = DEFAULT_CHUNK_SIZE;
    let total_size = total_chunks * chunk_size as u64;

    // Create reassembler with 50% completion
    let temp_file = NamedTempFile::new().unwrap();
    let mut reassembler = FileReassembler::new(temp_file.path(), total_size, chunk_size).unwrap();

    let dummy_data = vec![0u8; chunk_size];
    for i in 0..total_chunks / 2 {
        reassembler.write_chunk(i, &dummy_data).ok();
    }

    group.bench_function("check_missing", |b| {
        let mut idx = 0u64;
        b.iter(|| {
            let is_missing = reassembler.is_chunk_missing(idx);
            idx = (idx + 1) % total_chunks;
            black_box(is_missing)
        });
    });

    group.bench_function("check_received", |b| {
        b.iter(|| {
            // Check a chunk that is received
            let has = reassembler.has_chunk(0);
            black_box(has)
        });
    });

    group.finish();
}

// ============================================================================
// IncrementalTreeHasher Benchmarks (allocation optimization)
// ============================================================================

/// Benchmark IncrementalTreeHasher update performance
fn bench_incremental_hasher_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_hasher_update");

    // Test with different update sizes
    for update_size in [1024, 4096, 16384, 65536] {
        let data = vec![0xAAu8; update_size];

        group.throughput(Throughput::Bytes(update_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(update_size),
            &data,
            |b, data| {
                b.iter_batched(
                    || IncrementalTreeHasher::new(DEFAULT_CHUNK_SIZE),
                    |mut hasher| {
                        hasher.update(black_box(data));
                        black_box(hasher.chunk_count())
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark full incremental hashing workflow
fn bench_incremental_hasher_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_hasher_full");

    for total_size in [1_000_000, 10_000_000, 100_000_000] {
        let mut data = vec![0u8; total_size];
        rand::thread_rng().fill_bytes(&mut data);

        group.throughput(Throughput::Bytes(total_size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(total_size), &data, |b, data| {
            b.iter(|| {
                let mut hasher = IncrementalTreeHasher::new(DEFAULT_CHUNK_SIZE);
                // Feed in 64 KB chunks (simulating network transfer)
                for chunk in data.chunks(65536) {
                    hasher.update(chunk);
                }
                let tree = hasher.finalize();
                black_box(tree.root)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Merkle Tree Benchmarks (pre-allocation optimization)
// ============================================================================

/// Benchmark compute_merkle_root with various leaf counts
fn bench_merkle_root_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_root_computation");

    for num_leaves in [4, 16, 64, 256, 1024, 4096] {
        let leaves: Vec<[u8; 32]> = (0..num_leaves as u64)
            .map(|i: u64| {
                let mut hash = [0u8; 32];
                hash[0..8].copy_from_slice(&i.to_le_bytes());
                hash
            })
            .collect();

        group.throughput(Throughput::Elements(num_leaves as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_leaves),
            &leaves,
            |b, leaves| {
                b.iter(|| {
                    let root = compute_merkle_root(black_box(leaves));
                    black_box(root)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark tree hash from data (end-to-end)
fn bench_tree_hash_from_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_hash_from_data");

    for size in [1_000_000, 10_000_000, 100_000_000] {
        let data = vec![0xBBu8; size];

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| {
                let tree = compute_tree_hash_from_data(black_box(data), DEFAULT_CHUNK_SIZE);
                black_box(tree.root)
            });
        });
    }

    group.finish();
}

// ============================================================================
// FileChunker Benchmarks
// ============================================================================

/// Benchmark sequential file chunking
fn bench_file_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_chunking");

    for size in [1_000_000, 10_000_000, 100_000_000] {
        group.throughput(Throughput::Bytes(size as u64));

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

/// Benchmark random access chunking (seek + read)
fn bench_random_access_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("random_access_chunking");

    // Create a 100 MB file
    let size = 100_000_000u64;
    let mut temp_file = NamedTempFile::new().unwrap();
    let data = vec![0xBBu8; size as usize];
    temp_file.write_all(&data).unwrap();
    temp_file.flush().unwrap();
    let path = temp_file.path().to_path_buf();

    let num_chunks = size / DEFAULT_CHUNK_SIZE as u64;

    group.throughput(Throughput::Bytes(DEFAULT_CHUNK_SIZE as u64));

    group.bench_function("seek_and_read", |b| {
        let mut chunker = FileChunker::new(&path, DEFAULT_CHUNK_SIZE).unwrap();
        let mut chunk_idx = 0u64;

        b.iter(|| {
            // Read random chunk
            let chunk = chunker.read_chunk_at(chunk_idx).unwrap();
            chunk_idx = (chunk_idx + 7) % num_chunks; // Pseudo-random pattern
            black_box(chunk.len())
        });
    });

    group.finish();
}

// ============================================================================
// FileReassembler Write Performance
// ============================================================================

/// Benchmark chunk write performance
fn bench_chunk_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_write");

    let chunk_data = vec![0xCCu8; DEFAULT_CHUNK_SIZE];
    let total_size = 100 * DEFAULT_CHUNK_SIZE as u64; // 100 chunks

    group.throughput(Throughput::Bytes(DEFAULT_CHUNK_SIZE as u64));

    group.bench_function("sequential_write", |b| {
        b.iter_batched(
            || {
                let temp_file = NamedTempFile::new().unwrap();
                FileReassembler::new(temp_file.path(), total_size, DEFAULT_CHUNK_SIZE).unwrap()
            },
            |mut reassembler| {
                for i in 0..100u64 {
                    reassembler.write_chunk(i, black_box(&chunk_data)).unwrap();
                }
                black_box(reassembler.is_complete())
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.bench_function("random_write", |b| {
        b.iter_batched(
            || {
                let temp_file = NamedTempFile::new().unwrap();
                FileReassembler::new(temp_file.path(), total_size, DEFAULT_CHUNK_SIZE).unwrap()
            },
            |mut reassembler| {
                // Write in reverse order (worst case for sequential assumptions)
                for i in (0..100u64).rev() {
                    reassembler.write_chunk(i, black_box(&chunk_data)).unwrap();
                }
                black_box(reassembler.is_complete())
            },
            criterion::BatchSize::PerIteration,
        );
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    reassembler_benches,
    bench_missing_chunks_by_completion,
    bench_missing_count,
    bench_is_chunk_missing,
    bench_chunk_write,
);

criterion_group!(
    hasher_benches,
    bench_incremental_hasher_update,
    bench_incremental_hasher_full,
);

criterion_group!(
    merkle_benches,
    bench_merkle_root_computation,
    bench_tree_hash_from_data,
);

criterion_group!(
    chunker_benches,
    bench_file_chunking,
    bench_random_access_chunking,
);

criterion_main!(
    reassembler_benches,
    hasher_benches,
    merkle_benches,
    chunker_benches,
);
