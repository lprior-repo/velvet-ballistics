# Research Notes: arch: Enumerate all first-party Rust files over 300 lines

**Bead:** vb-zxgb  
**Date:** 2026-06-01  
**Researcher:** Lewis

## Executive Summary

- **Total files exceeding 300 lines:** 477
- **Hot runtime paths** (engine/runtime/generated/perf): 93 files
- **Cold test/diagnostic paths** (tests/benchmarks/verification): 270 files
- **Other** (compile, validate, storage, ipc, core, cli, yaml, doc): 114 files

## Research Methodology

1. Used `git ls-files '*.rs'` to enumerate all tracked Rust files
2. Filtered to first-party paths: `crates/`, `xtask/`, `scripts/`, `workspace_tests/`, `reference/`, `contracts/`
3. Excluded standard exclusions: `target/`, `.jj/`, `.beads/`, `.evidence/`, etc.
4. Counted physical lines per file using `wc -l`
5. Categorized based on path patterns defined in `scripts/check-source-length.sh`

## Categorization Criteria

### Hot Runtime Paths
Pattern matches from `check-source-length.sh::hot_files()`:
- `crates/vb_*/src/engine.rs`
- `crates/vb_*/src/engine/**`
- `crates/vb_runtime/src/**`
- `crates/vb_*/src/runtime/**`
- `crates/vb_*/src/generated/**`
- `crates/vb_*/src/perf/**`
- `crates/vb_cli/src/engine/**`
- `crates/vb_cli/src/runtime/**`
- `crates/vb_cli/src/generated/**`
- `crates/vb_cli/src/perf/**`

### Cold Test/Diagnostic Paths
Pattern matches from `check-source-length.sh::is_test_like_source_path()`:
- `*/tests.rs`, `*/*_tests.rs`, `*/tests/**`
- `*/kani/**`, `*/kani*`
- `*/verification/**`
- `*/benches/**`
- `proptest` files
- `workspace_tests/`

### Other Paths
All first-party files that don't match hot or cold patterns, including:
- `crates/vb_compile/src/**` - compiler implementation
- `crates/vb_validate/src/**` - validation logic
- `crates/vb_storage/src/**` - storage implementation
- `crates/vb_ipc/src/**` - IPC implementation
- `crates/vb_core/src/**` - core types and logic
- `crates/vb_cli/src/**` - CLI implementation (non-engine)
- `crates/vb_yaml/src/**` - YAML handling
- `crates/vb_doc/src/**` - documentation
- `xtask/src/**` - build tooling

## Notable Findings

1. **Largest files overall:**
   - `crates/vb_storage/src/tests.rs` (7743 lines) - storage tests
   - `crates/vb_core/src/budget/tests.rs` (7227 lines) - budget tests
   - `crates/vb_core/src/replay/tests.rs` (4224 lines) - replay tests

2. **Largest hot runtime files:**
   - `crates/vb_runtime/src/collect_tests.rs` (3753 lines)
   - `crates/vb_runtime/src/primitives/collect/tests.rs` (3716 lines)
   - `crates/vb_runtime/src/engine/drive.rs` (1385 lines)

3. **Test density:** ~57% of large files (270/477) are test/diagnostic code

## Usage

This enumeration data is intended for:
- Architectural drift detection (enforcing 300-line limit)
- Hot/cold path analysis for performance optimization
- Test coverage planning
- Refactoring prioritization

## Related Files

- Source script: `scripts/check-source-length.sh`
- Exception ledger: `.config/source-length-exceptions.txt`
