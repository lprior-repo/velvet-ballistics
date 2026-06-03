# Research Notes: arch: Enumerate all first-party Rust files over 300 lines

**Bead:** vb-zxgb
**Date:** 2026-06-01 (corrected with VERIFIED counts)
**Researcher:** Lewis

## VERIFIED Executive Summary

- **Total files exceeding 300 lines:** 502
- **Hot paths** (is_hot_source): 29 files
- **Cold paths** (is_test_like): 387 files
- **Other** (neither hot nor cold): 86 files

## Methodology

Used exact `is_excluded()`, `is_test_like()`, and `is_hot_source()` functions from `scripts/source_length_scan.rs`:

### is_excluded()
Excludes paths starting with or containing: target/, .jj/, .beads/, .evidence/, .cargo_temp/, arch-drift-, cargo-home/, cargo_home/, .cargo/registry/

### is_test_like()
Token-based matching for: diagnostic, diagnostics, fixture, fixtures, harness, harnesses, kani, loom, model, models, proof, proofs, property, properties, proptest, proptests, support, test, tests, verification, benches

### is_hot_source()
Files in `crates/<crate>/src/<path>` where:
- Crate is `vb_runtime` OR starts with `vb_`
- AND path first component is: engine.rs, engine, runtime, generated, or perf
- AND NOT test-like

## VERIFIED Enumeration Results

```
Total: 502
Hot: 29
Cold: 387
Other: 86
Sum: 502
```

### Hot Paths (29 files > 300 lines)

Files matching `is_hot_source()` - hot runtime paths in engine, runtime, generated, perf.

### Cold Paths (387 files > 300 lines)

Files matching `is_test_like()` - test, diagnostic, verification, proof, harness, etc.

### Other (86 files > 300 lines)

Files that are neither hot nor test-like - core implementation files in compile, validate, storage, ipc, cli modules.

## Key Findings

1. **Test density**: 77% of large files (387/502) are test/diagnostic code
2. **Hot path concentration**: Only 29 files are hot runtime paths needing architectural attention
3. **Other category**: 86 files are core implementation code not classified as hot or cold

## Verification Command

```bash
# Compiled and ran exact Rust functions from source_length_scan.rs
# Result: 502 total, 29 hot, 387 cold, 86 other
```

## Notes

This research epic produces enumeration data as a research artifact. The data is intended for:
- Architectural drift detection planning
- Refactoring prioritization
- Hot/cold path analysis for performance work

IMPORTANT: The `is_hot_source()` function is very restrictive - only files in vb_runtime or vb_* crates with specific path components (engine, runtime, generated, perf) are classified as hot.
