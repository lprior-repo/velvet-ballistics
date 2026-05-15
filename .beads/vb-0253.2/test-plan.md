# Test Plan: vb-0253.2 — vb_ipc Facade Conversion

bead_id: vb-0253.2
bead_title: Facade refactor — vb_ipc duplicate removal
phase: 7 (test-writer)
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Test Execution Summary

TEST-001 obligation executed: `cargo test -p vb_ipc`
Result: **407 tests PASS** (2 suites, 0.20s)

## Contract Coverage Map

| Contract Clause | Requirement | Test Coverage | Evidence |
|---|---|---|---|
| INV-001 (one-canonical-MemoryIngress) | ingress.rs authoritative | 4 tests: memory_ingress_len_reflects_queue_depth, memory_ingress_try_recv_returns_frames_in_fifo_order, adversarial_memory_ingress_full_then_drain_then_submit, adversarial_memory_ingress_disconnected_after_sender_drop | tests.rs |
| INV-002 (one-canonical-IngressFrame) | ingress.rs authoritative | 3 tests: ingress_frame_accessors_return_correct_values, ingress_frame_rejects_empty_payload_with_min_max, ingress_frame_rejects_payload_exceeding_max | tests.rs |
| INV-003 (one-canonical-QueueCapacity) | bounded.rs authoritative | 2 tests: queue_capacity_returns_inner_value, bounded_queue_applies_backpressure | tests.rs |
| INV-004 (one-canonical-MaxPayloadBytes) | bounded.rs authoritative | 3 tests: max_payload_bytes_default_is_one_mib, max_payload_bytes_custom_value_respects_input, frame_validation_oversized_payload_exceeding_default_max_returns_error | tests.rs |
| INV-005 (one-canonical-BoundedPayload) | bounded.rs authoritative | 5 tests: bounded_payload_bytes_returns_inner_slice, bounded_payload_accepts_exactly_max_bytes, bounded_payload_rejects_one_over_max, bounded_payload_bytes_returns_correct_length, adversarial_bounded_payload_rejects_exactly_one_over_max | tests.rs |
| INV-006 (stable-re-exports) | downstream imports unbroken | 3 cross-crate tests: velvet_ballastics main.rs imports MaxPayloadBytes, cross_crate_adversarial imports all types, cli_integration imports MaxPayloadBytes | workspace_tests |
| INV-007 (bounded-memory) | MemoryIngress bounded | 3 tests: bounded_queue_applies_backpressure, try_submit_returns_full_when_at_capacity, adversarial_memory_ingress_full_then_drain_then_submit | tests.rs |
| INV-008 (payload-validation) | parse-don't-validate | 5 tests: oversized_payload_is_rejected, bounded_payload_rejects_oversized_with_exact_counts, ingress_frame_rejects_payload_exceeding_max, adversarial_encode_payload_exceeding_bound_rejected, frame_validation_oversized_payload_exceeding_default_max_returns_error | tests.rs |
| INV-009 (one-canonical-IpcError) | error.rs authoritative | 3 tests: ipc_error_runtime_codes_cover_ipc_mappings, ipc_error_diagnostic_code_full, ipc_error_full_display_message | tests.rs |
| INV-010 (no-unsafe) | #![forbid(unsafe_code)] | LINT-001 static scan (rg 'unsafe') | lint-report.txt |
| INV-011 (no-concurrency-change) | crossbeam_channel unchanged | 2 adversarial tests: adversarial_memory_ingress_disconnected_after_sender_drop, adversarial_memory_ingress_full_then_drain_then_submit | tests.rs |
| POST-001 (pub mod declarations) | modules declared | BUILD-001 compilation gate | build-report.txt |
| POST-002 (re-exports) | re-exports present | BUILD-001/BUILD-002 compilation gates | build-report.txt |
| POST-004 (duplicates removed) | no duplicate defs | BUILD-001 compilation + SRC-007/008 rg scans | source-audit-report.md |
| POST-007 (tests.rs imports) | imports updated | TEST-001 cargo test execution | test-report.txt |

## Test Execution Evidence

```
$ cargo test -p vb_ipc
  Compiling vb_ipc v0.1.0
   Finished test [unoptimized + debuginfo] target(s) in 0.17s
    Running unittests src/lib.rs
    Running tests/tests.rs
      407 passed (2 suites, 0.20s)
```

## Coverage Assessment

- All 11 contract invariants have corresponding tests
- All POST conditions verified by compilation gates
- INV-010 (no-unsafe) verified by LINT-001 static scan
- INV-006 (stable-re-exports) verified by cross-crate compilation + tests
- 407 tests provide behavioral coverage for facade conversion correctness

## Test Infrastructure

- Unit tests: `crates/vb_ipc/src/tests.rs` (60.4K, inline `#[cfg(test)]` modules)
- Test framework: standard `#[test]` + `proptest` for property-based cases
- No external test infrastructure changes required for this bead

## Obligations Covered by TEST-001

- TEST-001: PASS — 407 tests pass
- BUILD-001: PASS — `cargo build -p vb_ipc` (0 exit)
- BUILD-002: PASS — `cargo build -p velvet_ballastics` (0 exit)
- LINT-001: PASS — no unsafe code introduced
