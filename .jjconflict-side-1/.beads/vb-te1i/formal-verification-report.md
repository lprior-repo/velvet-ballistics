# Formal Verification Report: vb-te1i — Binary IPC BDD Acceptance

**STATUS**: REJECTED — BLOCKED (pending formatting and clippy fixes)

## Inputs
- proof-obligations.jsonl: 28 obligations (21 required, 7 optional)
- delivery-scope.jsonl: vb_ipc primary scope + workspace_tests/vb_te1i_binary_ipc_acceptance.rs
- baseline-report.md: Not present in bead directory
- tla-spec.md: Present, TLA+ deemed non-applicable (pure codec layer)
- contract-verification-review.md: **STATUS: APPROVED** (Attempt 2/7)

---

## Tool Availability
- cargo: AVAILABLE
- rustc: AVAILABLE
- cargo clippy: AVAILABLE (workspace lint: 2 errors)
- cargo fmt: AVAILABLE (format check: 6 files with issues)
- cargo kani: NOT AVAILABLE (KAN-001/002/003 blocked)
- verus: NOT AVAILABLE (VERUS-001/002/003/004 blocked)
- TLA+ tools: N/A (temporal behavior not in scope)

---

## Obligation Results

### Unit Test Obligations (14 required, PASS)

| ID | Target | Command | Result |
|----|--------|---------|--------|
| UNIT-001 | frame::tests | cargo test --package vb_ipc -- frame::tests --nocapture | PASS |
| UNIT-002 | decode_rejects_invalid_magic | cargo test --package vb_ipc -- decode_rejects_invalid_magic --nocapture | PASS |
| UNIT-003 | decode_rejects_unsupported_version | cargo test --package vb_ipc -- decode_rejects_unsupported_version --nocapture | PASS |
| UNIT-004 | commands | cargo test --package vb_ipc -- commands --nocapture | PASS |
| UNIT-005 | decode_rejects_nonzero_reserved_field | cargo test --package vb_ipc -- decode_rejects_nonzero_reserved_field --nocapture | PASS |
| UNIT-006 | decode_rejects_payload_too_large | cargo test --package vb_ipc -- decode_rejects_payload_too_large --nocapture | PASS |
| UNIT-007 | new_rejects_payload_length_mismatch | cargo test --package vb_ipc -- new_rejects_payload_length_mismatch --nocapture | PASS |
| UNIT-008 | array_queue_tests | cargo test --package vb_ipc -- array_queue_tests --nocapture | PASS |
| UNIT-009 | constants | cargo test --package vb_ipc -- constants --nocapture | PASS |
| UNIT-010 | constants | cargo test --package vb_ipc -- constants --nocapture | PASS |
| BDD-001 | ipc_health_and_shutdown_return_expected_responses | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_health_and_shutdown_return_expected_responses -- --nocapture | PASS* |
| BDD-002 | ipc_submit_run_roundtrips_when_frame_is_valid | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_submit_run_roundtrips_when_frame_is_valid -- --nocapture | PASS* |
| BDD-003 | ipc_rejects_bad_magic_before_payload_allocation | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_rejects_bad_magic_before_payload_allocation -- --nocapture | PASS* |
| BDD-004 | ipc_returns_queue_full_when_backpressure_limit_is_hit | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_returns_queue_full_when_backpressure_limit_is_hit -- --nocapture | PASS* |
| BDD-005 | ipc_all_16_commands_have_typed_responses | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_all_16_commands_have_typed_responses -- --nocapture | PASS* |
| BDD-006 | ipc_correlation_ids_preserved_across_roundtrip | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_correlation_ids_preserved_across_roundtrip -- --nocapture | PASS* |
| BDD-007 | ipc_rejects_oversize_payload | cargo test --package workspace_tests vb_te1i_binary_ipc_acceptance::ipc_rejects_oversize_payload -- --nocapture | PASS* |

*BDD tests require formatting fix before they will compile cleanly. Currently failing `cargo fmt --check` prevents clean CI run.

### Static Scan Obligations (1 required)

| ID | Target | Command | Result |
|----|--------|---------|--------|
| STATIC-001 | vb_ipc constants | cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings | PASS (vb_ipc crate only) |

### Formal Proof Obligations (6 required, WAIVED due to BLOCKED_TOOLING)

| ID | Target | Waiver Reason | Compensating Evidence |
|----|--------|---------------|----------------------|
| KAN-001 | IpcFrameHeader::decode | BLOCKED_TOOLING: Kani not available | UNIT-002 (decode_rejects_invalid_magic) |
| KAN-002 | IpcFrameHeader::decode | BLOCKED_TOOLING: Kani not available | UNIT-006 (decode_rejects_payload_too_large) |
| KAN-003 | IpcFrameHeader::decode | BLOCKED_TOOLING: Kani not available | UNIT-002/003/005/006 |
| VERUS-001 | IpcCommand::from_u16 | BLOCKED_TOOLING: Cannot run Verus on single files with external deps | UNIT-004 + BDD-005 |
| VERUS-002 | BoundedPayload::new | BLOCKED_TOOLING: Cannot run Verus on single files with external deps | bounded_payload_new_* tests |
| VERUS-003 | IpcFrameHeader | BLOCKED_TOOLING: Cannot run Verus on single files with external deps | frame_types inline tests |
| VERUS-004 | IpcFrame::new | BLOCKED_TOOLING: Cannot run Verus on single files with external deps | UNIT-007 |

### Optional Obligations (7 optional)

| ID | Target | Layer | Required | Status |
|----|--------|-------|---------|--------|
| PROPTEST-001 | array_queue_tests | proptest | false | NOT RUN |
| LOOM-001 | queue/mod.rs | loom | false | NOT RUN |
| FUZZ-001 | frame.rs | cargo-fuzz | false | NOT RUN |

---

## Machine Gate Results

| Gate | Status | Notes |
|------|--------|-------|
| cargo build --workspace | PASS | 0 errors, 2 warnings (pre-existing dead_code in vb_cli/lifecycle.rs) |
| cargo test -p vb_ipc | PASS | 686 tests passed |
| cargo clippy --workspace -D warnings | FAIL_REGRESSION | vb_cli/lifecycle.rs dead_code (NOT in bead scope) |
| cargo fmt --check | FAIL_LOCAL | vb_te1i_binary_ipc_acceptance.rs formatting (IN bead scope) |

---

## Waivers

**Formal Waivers** (from proof-obligations.jsonl):
- KAN-001, KAN-002, KAN-003: BLOCKED_TOOLING — Kani unavailable
- VERUS-001, VERUS-002, VERUS-003, VERUS-004: BLOCKED_TOOLING — Verus cannot run on single files with external crate dependencies

---

## Residual Risk

1. **BLOCKED (MUST FIX)**: Formatting in vb_te1i_binary_ipc_acceptance.rs prevents clean CI
2. **BLOCKED (MUST FIX)**: Clippy dead_code in vb_cli/lifecycle.rs fails workspace-wide -D warnings
3. **DEFERRED_GLOBAL**: Kani/Verus formal proofs remain unverified due to tooling gaps (waived)
4. **DEFERRED_GLOBAL**: Proptest/Loom/Fuzz optional obligations not run (not required)

---

## Follow-Up Required

1. Fix formatting: `cargo fmt -- workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs`
2. Fix dead_code: Add `#[allow(dead_code)]` or implement the unused functions in vb_cli/lifecycle.rs
3. Re-run formal verification gates after fixes
4. Track workspace-wide formatting debt in separate bead
