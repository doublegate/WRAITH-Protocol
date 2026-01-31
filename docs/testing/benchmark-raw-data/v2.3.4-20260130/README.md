# v2.3.4 Benchmark Raw Results - 2026-01-30

**Date Collected:** 2026-01-30 01:44:16 UTC
**Version:** v2.3.4 (Performance Optimizations & Security Hardening)
**Run Type:** Final post-fix benchmark suite
**Machine:** Intel i9-10850K, 64 GB RAM

## Files

- `wraith-core.txt` (81 KB) - Core protocol benchmarks (frame, session, congestion, migration)
- `wraith-crypto.txt` (48 KB) - Cryptographic operations (Noise, AEAD, ratcheting)
- `wraith-obfuscation.txt` (36 KB) - Obfuscation layer benchmarks (padding, timing, mimicry)
- `wraith-files.txt` (27 KB) - File transfer benchmarks (chunking, hashing, reassembly)

## Data Format

Each file contains raw `cargo bench` output including:
- Test execution results (ignored benchmark tests)
- Criterion.rs benchmark measurements with timing statistics
- Performance comparisons vs. baseline
- Outlier analysis

## Analysis

This raw data should be synthesized into `docs/testing/BENCHMARK-ANALYSIS-v2.3.4.md` by:

1. Extracting key performance metrics from criterion output
2. Computing percentage improvements vs. v2.3.3 baseline
3. Creating benchmark tables and visualizations
4. Documenting optimization proposals and their implementations
5. Cross-referencing with CHANGELOG.md v2.3.4 changes

See `BENCHMARK-ANALYSIS-v2.3.2-optimized.md` for analysis document structure and methodology.

## Notes

- This represents the FINAL v2.3.4 benchmark run (20260130-completion)
- Earlier runs (v2.3.3 baseline, v2.3.4 pre-fix) were superseded
- The corresponding analysis document `BENCHMARK-ANALYSIS-v2.3.4.md` referenced in CHANGELOG.md has not yet been created
