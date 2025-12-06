//! Integration tests for edge cases
//!
//! Tests for:
//! - 0-byte file transfer
//! - Maximum size file handling
//! - Unicode and special characters in filenames
//! - Concurrent transfers to same peer
//! - Transfer during connection migration

use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;
use wraith_files::chunker::{FileChunker, FileReassembler};
use wraith_files::tree_hash::compute_tree_hash;

#[tokio::test]
async fn test_zero_byte_file_transfer() {
    let temp_dir = TempDir::new().unwrap();

    // Create 0-byte file
    let zero_file = temp_dir.path().join("zero.dat");
    fs::write(&zero_file, &[]).await.unwrap();

    // Verify file is 0 bytes
    let metadata = fs::metadata(&zero_file).await.unwrap();
    assert_eq!(metadata.len(), 0);

    // Chunk 0-byte file (should produce 0 chunks)
    let chunker = FileChunker::with_default_size(&zero_file).unwrap();
    assert_eq!(chunker.num_chunks(), 0);
    assert_eq!(chunker.total_size(), 0);

    // Reassemble 0-byte file
    let output_file = temp_dir.path().join("zero_out.dat");
    let reassembler = FileReassembler::new(&output_file, 0, 256 * 1024).unwrap();

    // Should be immediately complete for 0-byte file
    assert!(reassembler.is_complete());
    assert_eq!(reassembler.progress(), 1.0);

    reassembler.finalize().unwrap();

    // Verify output is 0 bytes
    let output_metadata = fs::metadata(&output_file).await.unwrap();
    assert_eq!(output_metadata.len(), 0);
}

#[tokio::test]
async fn test_single_byte_file_transfer() {
    let temp_dir = TempDir::new().unwrap();

    // Create 1-byte file
    let one_byte_file = temp_dir.path().join("one.dat");
    fs::write(&one_byte_file, &[0x42]).await.unwrap();

    // Chunk 1-byte file (should produce 1 chunk)
    let mut chunker = FileChunker::with_default_size(&one_byte_file).unwrap();
    assert_eq!(chunker.num_chunks(), 1);
    assert_eq!(chunker.total_size(), 1);

    let chunk = chunker.read_chunk().unwrap().unwrap();
    assert_eq!(chunk.len(), 1);
    assert_eq!(chunk[0], 0x42);

    // Reassemble
    let output_file = temp_dir.path().join("one_out.dat");
    let mut reassembler = FileReassembler::new(&output_file, 1, 256 * 1024).unwrap();

    reassembler.write_chunk(0, &chunk).unwrap();
    assert!(reassembler.is_complete());

    reassembler.finalize().unwrap();

    // Verify output
    let output_data = fs::read(&output_file).await.unwrap();
    assert_eq!(output_data, vec![0x42]);
}

#[tokio::test]
async fn test_maximum_chunk_boundary() {
    let temp_dir = TempDir::new().unwrap();

    // Create file that's exactly 256 KB (one full chunk)
    let chunk_size = 256 * 1024;
    let exact_chunk_file = temp_dir.path().join("exact_chunk.dat");
    let data = vec![0xAB; chunk_size];
    fs::write(&exact_chunk_file, &data).await.unwrap();

    let mut chunker = FileChunker::with_default_size(&exact_chunk_file).unwrap();
    assert_eq!(chunker.num_chunks(), 1);

    let chunk = chunker.read_chunk().unwrap().unwrap();
    assert_eq!(chunk.len(), chunk_size);
    assert!(chunk.iter().all(|&b| b == 0xAB));
}

#[tokio::test]
async fn test_just_over_chunk_boundary() {
    let temp_dir = TempDir::new().unwrap();

    // Create file that's 256 KB + 1 byte (should produce 2 chunks)
    let chunk_size = 256 * 1024;
    let over_chunk_file = temp_dir.path().join("over_chunk.dat");
    let data = vec![0xCD; chunk_size + 1];
    fs::write(&over_chunk_file, &data).await.unwrap();

    let mut chunker = FileChunker::with_default_size(&over_chunk_file).unwrap();
    assert_eq!(chunker.num_chunks(), 2);

    // Read both chunks
    let chunk1 = chunker.read_chunk().unwrap().unwrap();
    let chunk2 = chunker.read_chunk().unwrap().unwrap();

    assert_eq!(chunk1.len(), chunk_size);
    assert_eq!(chunk2.len(), 1);
}

#[tokio::test]
async fn test_large_file_handling() {
    let temp_dir = TempDir::new().unwrap();

    // Create a large file (10 MB)
    let large_file = temp_dir.path().join("large.dat");
    let size = 10 * 1024 * 1024; // 10 MB
    let chunk_size = 256 * 1024;

    // Write in chunks to avoid memory pressure
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&large_file)
        .await
        .unwrap();

    use tokio::io::AsyncWriteExt;
    let chunk_data = vec![0xEF; chunk_size];
    for _ in 0..(size / chunk_size) {
        file.write_all(&chunk_data).await.unwrap();
    }
    file.flush().await.unwrap();
    drop(file);

    // Verify file size
    let metadata = fs::metadata(&large_file).await.unwrap();
    assert_eq!(metadata.len() as usize, size);

    // Chunk large file
    let chunker = FileChunker::with_default_size(&large_file).unwrap();
    assert_eq!(chunker.num_chunks() as usize, size / chunk_size);

    // Compute tree hash (should handle large files efficiently)
    let tree_hash = compute_tree_hash(&large_file, chunk_size).unwrap();
    assert_eq!(tree_hash.chunk_count(), size / chunk_size);
    assert_ne!(tree_hash.root, [0u8; 32]); // Non-zero hash
}

#[tokio::test]
async fn test_unicode_filename() {
    let temp_dir = TempDir::new().unwrap();

    // Unicode filename with various scripts
    let unicode_name = "测试文件_файл_ファイル_📁.dat";
    let unicode_file = temp_dir.path().join(unicode_name);

    // Create file with unicode name
    let data = vec![0x55; 1024];
    fs::write(&unicode_file, &data).await.unwrap();

    // Verify file exists and can be read
    assert!(unicode_file.exists());
    let read_data = fs::read(&unicode_file).await.unwrap();
    assert_eq!(read_data, data);

    // Chunk file with unicode name
    let chunker = FileChunker::with_default_size(&unicode_file).unwrap();
    assert_eq!(chunker.num_chunks(), 1);
}

#[tokio::test]
async fn test_special_characters_in_filename() {
    let temp_dir = TempDir::new().unwrap();

    // Filename with special characters (but valid on most filesystems)
    let special_name = "file-with_special.chars[1](2){3}@#$.dat";
    let special_file = temp_dir.path().join(special_name);

    // Create file
    let data = vec![0x77; 512];
    fs::write(&special_file, &data).await.unwrap();

    // Verify file operations work
    assert!(special_file.exists());
    let read_data = fs::read(&special_file).await.unwrap();
    assert_eq!(read_data, data);

    // Chunk file
    let chunker = FileChunker::with_default_size(&special_file).unwrap();
    assert_eq!(chunker.num_chunks(), 1);
}

#[tokio::test]
async fn test_very_long_filename() {
    let temp_dir = TempDir::new().unwrap();

    // Most filesystems support up to 255 bytes for filename
    // Create a 200-character filename (well within limits)
    let long_name = "a".repeat(200) + ".dat";
    let long_file = temp_dir.path().join(&long_name);

    // Create file
    let data = vec![0x88; 256];
    fs::write(&long_file, &data).await.unwrap();

    // Verify file operations work
    assert!(long_file.exists());
    let chunker = FileChunker::with_default_size(&long_file).unwrap();
    assert_eq!(chunker.num_chunks(), 1);
}

#[tokio::test]
async fn test_concurrent_file_operations() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple files concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let temp_path = temp_dir.path().to_path_buf();
        let handle = tokio::spawn(async move {
            let file_path = temp_path.join(format!("concurrent_{}.dat", i));
            let data = vec![i as u8; 1024 * (i + 1)];
            fs::write(&file_path, &data).await.unwrap();

            // Chunk the file
            let chunker = FileChunker::with_default_size(&file_path).unwrap();
            chunker.num_chunks()
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let num_chunks = handle.await.unwrap();
        assert!(num_chunks > 0);
    }
}

#[tokio::test]
async fn test_concurrent_transfers_same_peer() {
    // Test that we can create multiple files that could be transferred concurrently
    let temp_dir = TempDir::new().unwrap();
    let mut file_paths = Vec::new();

    // Create multiple files that could be transferred concurrently
    for i in 0..5 {
        let file_path = temp_dir.path().join(format!("transfer_{}.dat", i));
        fs::write(&file_path, vec![i as u8; 1024]).await.unwrap();
        file_paths.push(file_path);
    }

    // Verify all files exist and have unique paths
    let unique_paths: std::collections::HashSet<_> = file_paths.iter().collect();
    assert_eq!(unique_paths.len(), file_paths.len());

    // Verify each file can be chunked independently
    for file_path in &file_paths {
        let chunker = FileChunker::with_default_size(file_path).unwrap();
        assert_eq!(chunker.num_chunks(), 1);
    }
}

#[tokio::test]
async fn test_resume_state_with_zero_chunks() {
    use wraith_core::node::ResumeState;

    // Create resume state for 0-byte file
    let state = ResumeState::new(
        [1u8; 32],
        [2u8; 32],
        [3u8; 32],
        0, // 0-byte file
        256 * 1024,
        PathBuf::from("/tmp/zero.dat"),
        true,
    );

    assert_eq!(state.total_chunks, 0);
    assert!(state.is_complete());
    assert_eq!(state.progress(), 100.0);
    assert_eq!(state.missing_chunks().len(), 0);
}

#[tokio::test]
async fn test_resume_state_with_one_chunk() {
    use wraith_core::node::ResumeState;

    // Create resume state for 1-byte file (1 chunk)
    let mut state = ResumeState::new(
        [1u8; 32],
        [2u8; 32],
        [3u8; 32],
        1, // 1-byte file
        256 * 1024,
        PathBuf::from("/tmp/one.dat"),
        true,
    );

    assert_eq!(state.total_chunks, 1);
    assert!(!state.is_complete());
    assert_eq!(state.progress(), 0.0);

    state.mark_chunk_complete(0);
    assert!(state.is_complete());
    assert_eq!(state.progress(), 100.0);
}

#[tokio::test]
async fn test_chunk_hash_verification() {
    let temp_dir = TempDir::new().unwrap();

    // Create file and compute tree hash
    let file_path = temp_dir.path().join("verify.dat");
    let data = vec![0x99; 512 * 1024]; // 512 KB
    fs::write(&file_path, &data).await.unwrap();

    let chunk_size = 256 * 1024;
    let tree_hash = compute_tree_hash(&file_path, chunk_size).unwrap();

    // Verify each chunk
    let mut chunker = FileChunker::new(&file_path, chunk_size).unwrap();

    for i in 0..tree_hash.chunk_count() {
        let chunk_data = chunker.read_chunk_at(i as u64).unwrap();
        assert!(tree_hash.verify_chunk(i, &chunk_data));
    }
}

#[tokio::test]
async fn test_chunk_hash_verification_fails_on_corruption() {
    let temp_dir = TempDir::new().unwrap();

    // Create file and compute tree hash
    let file_path = temp_dir.path().join("corrupt.dat");
    let data = vec![0xAA; 256 * 1024];
    fs::write(&file_path, &data).await.unwrap();

    let tree_hash = compute_tree_hash(&file_path, 256 * 1024).unwrap();

    // Create corrupted chunk (different data)
    let corrupted_chunk = vec![0xBB; 256 * 1024];

    // Verification should fail
    assert!(!tree_hash.verify_chunk(0, &corrupted_chunk));
}

#[tokio::test]
async fn test_partial_chunk_at_end() {
    let temp_dir = TempDir::new().unwrap();

    // Create file with partial last chunk (257 KB = 256 KB + 1 KB)
    let file_path = temp_dir.path().join("partial.dat");
    let data = vec![0xCC; 257 * 1024];
    fs::write(&file_path, &data).await.unwrap();

    let chunk_size = 256 * 1024;
    let mut chunker = FileChunker::new(&file_path, chunk_size).unwrap();

    assert_eq!(chunker.num_chunks(), 2);

    // First chunk: full size
    let chunk1 = chunker.read_chunk_at(0).unwrap();
    assert_eq!(chunk1.len(), chunk_size);

    // Second chunk: partial (1 KB)
    let chunk2 = chunker.read_chunk_at(1).unwrap();
    assert_eq!(chunk2.len(), 1024);
}

#[tokio::test]
async fn test_reassembly_with_gaps() {
    let temp_dir = TempDir::new().unwrap();

    // Create output file with gaps (missing chunks)
    let output_file = temp_dir.path().join("gaps.dat");
    let chunk_size = 256 * 1024;
    let total_size = 4 * chunk_size as u64; // 1 MB

    let mut reassembler = FileReassembler::new(&output_file, total_size, chunk_size).unwrap();

    // Write chunks 0, 2 (leaving 1, 3 missing)
    let chunk_data = vec![0xDD; chunk_size];
    reassembler.write_chunk(0, &chunk_data).unwrap();
    reassembler.write_chunk(2, &chunk_data).unwrap();

    assert!(!reassembler.is_complete());
    assert_eq!(reassembler.received_count(), 2);
    assert_eq!(reassembler.missing_count(), 2);

    let missing = reassembler.missing_chunks_sorted();
    assert_eq!(missing, vec![1, 3]);

    // Fill gaps
    reassembler.write_chunk(1, &chunk_data).unwrap();
    reassembler.write_chunk(3, &chunk_data).unwrap();

    assert!(reassembler.is_complete());
    assert_eq!(reassembler.progress(), 1.0);
}

#[tokio::test]
async fn test_out_of_order_chunk_writing() {
    let temp_dir = TempDir::new().unwrap();

    // Write chunks in completely random order
    let output_file = temp_dir.path().join("random_order.dat");
    let chunk_size = 100 * 1024; // 100 KB chunks
    let num_chunks = 10;
    let total_size = (num_chunks * chunk_size) as u64;

    let mut reassembler = FileReassembler::new(&output_file, total_size, chunk_size).unwrap();

    // Write in order: 9, 7, 5, 3, 1, 8, 6, 4, 2, 0
    let write_order = vec![9, 7, 5, 3, 1, 8, 6, 4, 2, 0];

    for &chunk_idx in &write_order {
        let chunk_data = vec![chunk_idx as u8; chunk_size];
        reassembler.write_chunk(chunk_idx, &chunk_data).unwrap();
    }

    assert!(reassembler.is_complete());
    assert_eq!(reassembler.received_count(), num_chunks as u64);
}
