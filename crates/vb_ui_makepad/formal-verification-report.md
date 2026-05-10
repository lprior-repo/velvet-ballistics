# Formal Verification Report

STATUS: REJECTED

## Inputs
- proof-obligations.jsonl: MISSING (no beads directory for vb_ui_makepad)
- contract-verification-review.md: MISSING
- traceability-matrix.jsonl: MISSING
- TEST-PLAN.md: PRESENT (defines 425 required tests, 0 exist)

## Tool Availability
- lake: NOT_APPLICABLE (no Lean proof project)
- rust-verification-gauntlet.sh: EXISTS (/home/lewis/src/Velvet-ballistics/scripts/rust-verification-gauntlet.sh)
- cargo kani: AVAILABLE (cargo-kani 0.67.0)
- moon: AVAILABLE (moon v2)
- cargo fuzz: NOT_FOUND (no fuzz directory in crate)
- cargo bolero: NOT_FOUND (no bolero markers)
- lockbud: NOT_APPLICABLE (no concurrency primitives requiring lockbud)
- cargo mutants: AVAILABLE (cargo-mutants smoke task exists)
- cargo llvm-cov: AVAILABLE (coverage task exists)
- cargo asm / cargo-show-asm: AVAILABLE

## Obligation Results

### Test Existence Obligation
- layer: unit-test
- checker: cargo test -p vb_ui_makepad
- command: `cargo test -p vb_ui_makepad --no-run && cargo test -p vb_ui_makepad`
- result: FAIL
- evidence: |
  - Compiled successfully (target/debug/deps/vb_ui_makepad-33254a0c7f926352)
  - cargo test output: "0 passed (2 suites, 0.00s)"
  - No #[test] functions found via grep
  - TEST-PLAN.md specifies 425 required tests, 0 exist

### Kani Proof Obligation
- layer: kani
- checker: cargo kani -p vb_ui_makepad
- command: `cargo kani -p vb_ui_makepad`
- result: FAIL
- evidence: |
  - "Manual Harness Summary: No proof harnesses (functions with #[kani::proof]) were found to verify."
  - No kani/ directory in crate
  - TEST-PLAN.md lists 3 kani harnesses (kani_test_packet_dot_animation.rs, kani_test_graph_canvas_bounds.rs, kani_test_zoom_bounds.rs) - none exist

### Fuzz Obligation
- layer: fuzz
- checker: directory inspection
- command: N/A (no fuzz directory found)
- result: FAIL
- evidence: |
  - No fuzz/ directory in /home/lewis/src/Velvet-ballistics/crates/vb_ui_makepad
  - No cargo fuzz build targets
  - TEST-PLAN.md lists 2 fuzz targets (parse_hex, from_toml) - none exist

### Integration Test Obligation
- layer: integration-test
- checker: directory inspection
- command: N/A (no tests/ directory found)
- result: FAIL
- evidence: |
  - No tests/ directory in /home/lewis/src/Velvet-ballistics/crates/vb_ui_makepad
  - TEST-PLAN.md specifies ~140 integration tests - 0 exist

### Proptest Obligation
- layer: proptest
- checker: directory inspection
- command: N/A
- result: FAIL
- evidence: |
  - No proptest strategies found in crate
  - TEST-PLAN.md specifies 8 proptest invariants - 0 exist

## Waivers
- None. No formal-waivers.jsonl exists for this crate.

## Residual Risk
- ZERO tests exist for 85 public functions across tokens.rs, shell.rs, packet_dot.rs, graph_canvas.rs, graph_node.rs, graph_edge.rs, error.rs
- No formal specification artifacts (proof-obligations.jsonl, contract-verification-review.md with STATUS: APPROVED, traceability-matrix.jsonl)
- No Kani harnesses to verify bounds safety, overflow safety, or contract properties
- No fuzz targets to catch parse_hex or from_toml edge cases
- No mutation testing coverage
- No code coverage metrics available (cargo llvm-cov would return 0%)

## Verification Gate Summary

| Obligation | Required | Found | Status |
|------------|----------|-------|--------|
| Unit Tests | 260 | 0 | FAIL |
| Integration Tests | 140 | 0 | FAIL |
| Proptest Invariants | 8 | 0 | FAIL |
| Fuzz Targets | 2 | 0 | FAIL |
| Kani Harnesses | 3 | 0 | FAIL |
| proof-obligations.jsonl | 1 | 0 | FAIL |
| contract-verification-review.md (APPROVED) | 1 | 0 | FAIL |
| traceability-matrix.jsonl | 1 | 0 | FAIL |

VERDICT: REJECTED — This crate has zero tests, zero proof harnesses, and no formal verification artifacts. The TEST-PLAN.md specifies 425 required tests across unit/integration/proptest/fuzz layers but none have been implemented.
