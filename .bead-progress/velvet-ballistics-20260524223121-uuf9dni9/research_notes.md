# Research Notes: arch: Enumerate all first-party Rust files over 300 lines

**Bead:** vb-zxgb
**Date:** 2026-06-01 (final corrected)
**Researcher:** Lewis

## Executive Summary

- **Total files exceeding 300 lines:** 498
- **Hot paths** (non-test-like, >300 lines): 121 files
- **Cold paths** (test-like, >300 lines): 377 files

## Methodology

Used exact `is_excluded()`, `is_test_like()`, and `is_hot_source()` functions from `scripts/source_length_scan.rs`:

### Exclusions (is_excluded)
```rust
file.starts_with("target/")
    || file.starts_with(".jj/")
    || file.starts_with(".beads/")
    || file.starts_with(".evidence/")
    || file.starts_with(".cargo_temp/")
    || file.starts_with("arch-drift-")
    || file.starts_with("cargo-home/")
    || file.starts_with("cargo_home/")
    || file.starts_with(".cargo/registry/")
    || file.contains("/target/")
    || file.contains("/.jj/")
    || file.contains("/.beads/")
    || file.contains("/.evidence/")
    || file.contains("/.cargo_temp/")
```

### Cold Test/Diagnostic Paths (is_test_like)
Token-based matching for: diagnostic, diagnostics, fixture, fixtures, harness, harnesses, kani, loom, model, models, proof, proofs, property, properties, proptest, proptests, support, test, tests, verification, benches

### Hot Paths (is_hot_source)
Files in vb_runtime/src, or vb_* crates with first path component being: engine.rs, engine, runtime, generated, perf

## Accurate Enumeration

### Hot Paths (121 files > 300 lines)

Files in vb_runtime/src or vb_* crates matching hot path patterns, excluding test-like files.

### Cold Paths (377 files > 300 lines)

All test-like files (tests, diagnostics, fixtures, harnesses, kani, loom, models, proofs, properties, proptests, support, verification, benches).

## Key Findings

1. **Test density**: 76% of large files (377/498) are test/diagnostic code
2. **Hot path concentration**: The 121 hot files represent the core code needing architectural attention
3. **Exception ledger bloat**: `.config/source-length-exceptions.txt` has ~480 entries

## Verification

```bash
# Using the compiled check-source-length tool:
target/gate-tools/check-source-length

# Or enumerate manually using is_excluded and is_test_like from source_length_scan.rs
```

**Verified counts:** 498 total | 121 hot | 377 cold

## Notes

This research epic produces enumeration data as a research artifact. The data is intended for:
- Architectural drift detection planning
- Refactoring prioritization
- Hot/cold path analysis for performance work

Note: The `is_test_like` function uses token-based matching which may classify some files differently than simple pattern matching. Use the compiled tool for definitive enumeration.
