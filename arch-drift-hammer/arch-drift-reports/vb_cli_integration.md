# Architectural Drift Report: `vb_cli/tests/cli_integration.rs`

**File:** `crates/vb_cli/tests/cli_integration.rs`  
**Date:** 2026-05-29  
**Status:** DRIFT DETECTED — REFACTOR REQUIRED

---

## Metrics

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | **3990** | 300 | ❌ OVERSIZED (13.3×) |
| Test Count | **106** | — | informational |
| File Size (bytes) | ~110KB | — | informational |

---

## Findings

### 1. File Size Violation (CRITICAL)

The file is **3990 lines**, exceeding the mandated **300-line maximum** by a factor of **13.3×**.

Per the architectural-drift skill:
> "Any file > 300 lines MUST be split."

This is a severe structural violation. A test file of this size indicates the integration tests have grown beyond maintainability.

### 2. Test Distribution by Phase

The file is organized into labeled phases (found via comment headers):

| Phase | Lines (approx) | Test Count |
|-------|----------------|------------|
| Helpers / Setup | 1–162 | — |
| Phase 1: YAML parsing (vb_yaml) | ~819–980 | ~12 |
| Phase 2: Validation (vb_validate) | ~985–1000 | ~1 |
| Phase 3: Expression engine (vb_expr) | ~1006–1124 | ~5 |
| Phase 4: Core IR validation | ~1130–1160 | ~2 |
| Phase 5: Compile pipeline | ~1166–1199 | ~3 |
| Phase 6: IPC frame encode/decode | ~1205–1235 | ~1 |
| Phase 7: Storage record encode/decode | ~1241–1311 | ~2 |
| Phase 8: Runtime engine signals | ~1317–1329 | ~2 |
| CLI integration tests | ~1331–3990 | ~78+ |

### 3. DDD Cohesion Analysis

The file bundles **8 distinct phase categories** into a single monolithic test module:
- `vb_yaml` parsing tests
- `vb_validate` schema tests
- `vb_expr` lexer/parser/bytecode/eval tests
- `vb_core` IR validation tests
- `vb_compile` pipeline tests
- `vb_ipc` frame tests
- `vb_storage` record tests
- `vb_core` runtime signal tests
- Full CLI integration tests

This violates Scott Wlaschin DDD cohesion principles — each **bounded context** should have its own test artifact.

### 4. Helper Function Bloat

The file contains ~12 helper functions (lines 29–160) that serve all phases:
- `input_slot_parts()`, `minimal_parts()` — workflow construction
- `resolve_test_reference()` — expression resolution
- `forced_assertion_failure()` — assertion trap
- `cli_tempdir()`, `write_test_file()` — file I/O
- `run_cli()` — process execution
- `output_stdout()`, `output_stderr()`, `first_stderr_line()` — output parsing
- `assert_cli_success()`, `assert_cli_failure_contains()` — assertions

These helpers are phase-agnostic and should be in a shared `test_helpers` module.

---

## Recommendations

### Immediate (Required)

1. **Split into phase-specific test modules** under `crates/vb_cli/tests/`:

```
crates/vb_cli/tests/
  cli_integration.rs          # Main CLI tests only (status, action, run, etc.)
  cli_yaml_parse_tests.rs     # Phase 1: vb_yaml integration
  cli_expr_tests.rs           # Phase 3: vb_expr integration  
  cli_core_ir_tests.rs        # Phase 4: vb_core IR validation
  cli_compile_tests.rs        # Phase 5: vb_compile pipeline
  cli_ipc_tests.rs            # Phase 6: vb_ipc frame roundtrip
  cli_storage_tests.rs        # Phase 7: vb_storage record roundtrip
  cli_signal_tests.rs          # Phase 8: runtime signal types
  helpers/
    mod.rs                    # Shared test helpers
    cli_helpers.rs            # CLI-specific helpers (run_cli, tempdir, etc.)
    workflow_helpers.rs        # WorkflowParts construction helpers
```

2. **Extract shared helpers** into `tests/helpers/` module to eliminate duplication across split files.

3. **Each new file must remain ≤300 lines.**

### Rationale

- **Maintainability**: 3990-line files are untestable in isolation; each phase can now be run/targeted independently.
- **Parallel CI**: Phase-specific test files enable targeted test runs.
- **Cohesion**: Each test module maps to a single bounded context (vb_yaml, vb_expr, vb_core, vb_compile, vb_ipc, vb_storage).
- **Tooling**: Verification tools (Kani, Miri, Loom) can target specific phases without loading the entire test suite.

---

## Verification Checklist

- [ ] `crates/vb_cli/tests/cli_integration.rs` reduced to ≤300 lines
- [ ] `crates/vb_cli/tests/cli_yaml_parse_tests.rs` created (phase 1)
- [ ] `crates/vb_cli/tests/cli_expr_tests.rs` created (phase 3)
- [ ] `crates/vb_cli/tests/cli_core_ir_tests.rs` created (phase 4)
- [ ] `crates/vb_cli/tests/cli_compile_tests.rs` created (phase 5)
- [ ] `crates/vb_cli/tests/cli_ipc_tests.rs` created (phase 6)
- [ ] `crates/vb_cli/tests/cli_storage_tests.rs` created (phase 7)
- [ ] `crates/vb_cli/tests/cli_signal_tests.rs` created (phase 8)
- [ ] `crates/vb_cli/tests/helpers/` module extracted
- [ ] All new files ≤300 lines
- [ ] All 106 tests still pass after refactor
- [ ] `cargo test -p vb_cli` passes

---

**STATUS: DRIFT DETECTED — SPLIT REQUIRED**
