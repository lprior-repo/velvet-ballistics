# Proof-to-Rust Bridge Review: vb-y9d3v (vb-y4pa) — Attempt 2

## Review Provenance

- **Reviewer**: proof-reviewer (femdation delegate, seq 13)
- **Bead**: vb-y9d3v (vb-y4pa) — ActionTicket fence / Body re-entry state reset
- **State**: 7 (proof-to-rust bridge review — RETRY after fixes)
- **Date**: 2026-05-30
- **Bridge artifact reviewed**: `bd/vb-y4pa/proof-obligations.planned.jsonl` (15 POs)
- **Prior review**: proof-review.md (STATUS: REJECTED, state 5/6)
- **Prior bridge attempt**: Rejected with 3 blocking findings (phantom engine.rs→engine/types.rs, kani-list.sh→cargo kani, duplicate seq 8)

---

## STATUS: REJECTED

---

## Executive Summary

**The 3 original blocking findings from bridge attempt 1 are RESOLVED.** However, independent re-review reveals **7 new critical bridge mapping failures** (4 phantom file targets, 3 phantom Kani harness names) that prevent verification execution via the mapped commands. The Rust implementation fixes are correctly applied, but the bridge obligation map does not accurately reflect the actual source file structure or harness naming.

---

## Attempt 1 Blocker Resolution Verification

### Finding 1: phantom engine.rs→engine/types.rs (8 RROs) — RESOLVED ✓

The original bridge mapped 8 obligations to `crates/vb_core/src/engine.rs`. All 8 now correctly point to existing files (`crates/vb_core/src/frame.rs`, `crates/vb_proof_kernels/src/step_state.rs`, etc.). No remaining `engine.rs` phantom references.

**Evidence**: All 15 target file paths in proof-obligations.planned.jsonl were verified against actual filesystem with `test -f`.

### Finding 2: kani-list.sh→cargo kani (10 RROs) — RESOLVED ✓

All verification commands now use `cargo kani` (correct). No `kani-list.sh` references remain. The sole non-Kani command (PO-015) correctly uses `verus` for the Verus proof kernel obligation.

**Evidence**: Grep of all command fields in proof-obligations.planned.jsonl shows only `cargo kani`, `cargo test`, and `verus`.

### Finding 3: duplicate seq 8 — RESOLVED ✓

No duplicate sequence numbers in the 15 PO IDs (PO-001 through PO-015, sequential, no gaps, no duplicates).

**Evidence**: Parsed all `id` fields in proof-obligations.planned.jsonl; all are unique and sequential.

---

## New Critical Bridge Mapping Failures (7 RROs)

### NF-1: Phantom Kani Harness File Targets (4 RROs) — CRITICAL

| PO | Bridge Target (NONEXISTENT) | Actual Location |
|----|---------------------------|-----------------|
| PO-011 | `crates/vb_runtime/src/kani_y4pa_for_each_reentry.rs` | `crates/vb_runtime/src/primitives/reentry_proofs.rs` |
| PO-012 | `crates/vb_runtime/src/kani_y4pa_reduce_reentry.rs` | `crates/vb_runtime/src/primitives/reentry_proofs.rs` |
| PO-013 | `crates/vb_runtime/src/kani_y4pa_collect_reentry.rs` | `crates/vb_runtime/src/primitives/reentry_proofs.rs` |
| PO-014 | `crates/vb_runtime/src/kani_y4pa_repeat_reentry.rs` | `crates/vb_runtime/src/primitives/reentry_proofs.rs` |

**Finding code**: BRDG/NEXIST/FILE/v1
**Severity**: CRITICAL
**Required fix**: Update target paths to `crates/vb_runtime/src/primitives/reentry_proofs.rs` for PO-011 through PO-014. All 4 Kani harnesses (`for_each_next_reentry`, `reduce_next_reentry`, `collect_next_reentry`, `collect_page_reentry`) are consolidated in that single file under `pub mod reentry_harnesses` (declared in `primitives/mod.rs:14` with `#[cfg(kani)]`).

### NF-2: Phantom Kani Harness Name — PO-001 — CRITICAL

| Field | Value |
|-------|-------|
| **PO** | PO-001 |
| **Bridge harness name** | `state_machine_succeeded_pending` |
| **Target file** | `crates/vb_proof_kernels/src/step_state.rs` |
| **Actual status** | `step_state.rs` contains NO `#[kani::proof]` harnesses whatsoever. The file has unit tests (`test_invalid_transitions`, `test_terminal_immutable`, etc.) but zero Kani proofs. The named harness does not exist. |

**Finding code**: BRDG/NEXIST/HARNESS/v1
**Severity**: CRITICAL
**Required fix**: Either implement `#[kani::proof] fn state_machine_succeeded_pending()` in `step_state.rs` or update the bridge to reference an existing harness. The `test_invalid_transitions` unit test (line 207) does verify `Succeeded→Running` is invalid and `test_terminal_immutable` (line 217) verifies the terminal property, so partial test coverage exists.

### NF-3: Phantom Kani Harness Name — PO-002 — CRITICAL

| Field | Value |
|-------|-------|
| **PO** | PO-002 |
| **Bridge harness name** | `mark_pending_harness` |
| **Target file** | `crates/vb_core/src/frame.rs` |
| **Actual status** | `frame.rs` contains 11 `#[kani::proof]` harnesses (K-F2 through K-F5, K-PC1 through K-PC3, K-S1, K-S2, plus 2 from `parallel_in_flight_kani` module). NONE are named `mark_pending_harness`. The `mark_pending` method exists at line 395 but has no dedicated Kani harness. |

**Finding code**: BRDG/NEXIST/HARNESS/v1
**Severity**: CRITICAL
**Required fix**: Either create `#[kani::proof] fn mark_pending_harness()` in `frame.rs` or update the bridge to reference an existing harness that validates `mark_pending` behavior. The existing `validate_transition_terminal_blocks_all` harness (line 1931) does test `Succeeded→Pending` for the state machine side, but not `mark_pending` specifically.

### NF-4: Phantom Kani Harness Name — PO-003 — CRITICAL

| Field | Value |
|-------|-------|
| **PO** | PO-003 |
| **Bridge harness name** | `jump_to_body_reset` |
| **Target file** | `crates/vb_runtime/src/primitives/helpers.rs` |
| **Actual status** | `helpers.rs` has 5 unit tests for `jump_to_body` (tc001 through tc005, lines 426-525) but NO `#[kani::proof]` harnesses. The named harness `jump_to_body_reset` does not exist. |

**Finding code**: BRDG/NEXIST/HARNESS/v1
**Severity**: CRITICAL
**Required fix**: Either create `#[kani::proof] fn jump_to_body_reset()` in `helpers.rs` or update the bridge to reference existing unit tests. The tc001-tc005 tests cover Succeeded→Pending, Pending idempotency, Waiting re-entry, and Asking re-entry cases.

---

## Verified Bridge Mappings (Pass)

The following obligation bridge mappings are accurate:

| PO | Target | Harness / Test | Status |
|----|--------|---------------|--------|
| PO-004 | `for_each.rs` | `for_each_next_reentry` (reentry_proofs.rs:67) | ✓ |
| PO-005 | `reduce.rs` | `reduce_next_reentry` (reentry_proofs.rs:162) | ✓ |
| PO-006 | `collect.rs` | `collect_next_reentry` (reentry_proofs.rs:251) | ✓ |
| PO-007 | `collect.rs` | `collect_page_reentry` (reentry_proofs.rs:357) | ✓ |
| PO-008 | `repeat.rs` | `repeat_attempt_reentry` (reentry_proofs.rs:454) | ✓ |
| PO-009 | `repeat.rs` | `repeat_check_reentry` (reentry_proofs.rs:525) | ✓ |
| PO-010 | `for_each/tests.rs` | Unit test file exists | ✓ |
| PO-015 | `step_state.rs` (proof kernel) | Verus target exists | ✓ (target file, Verus execution not verified) |

For PO-004 through PO-009: The harness names match actual `#[kani::proof]` functions in `reentry_proofs.rs`. The test commands (`cargo test -p vb_runtime <test_name>`) would match existing test functions in `reentry_tests.rs` by substring, but the test names in the commands (e.g., `for_each_two_item_reentry`) match the OLD naming convention (pre-rename). The actual tests use names like `vb_y4pa_001_for_each_two_item_reentry`.

---

## Implementation Bridge Status

The Rust implementation correctly applies all contract fixes:

| Fix | Location | Status |
|-----|----------|--------|
| `Succeeded→Pending` in VALID_TRANSITIONS | `step_state.rs:48` | ✓ Present |
| `mark_pending()` method | `frame.rs:395-397` | ✓ Present |
| `jump_to_body()` helper | `helpers.rs:60-69` | ✓ Present |
| `jump_to_body` in for_each_next | `for_each.rs:86` | ✓ Present |
| `jump_to_body` in reduce_next | `reduce.rs:84` | ✓ Present |
| `jump_to_body` in collect_page | `collect.rs:428` | ✓ Present |
| `jump_to_body` in collect_next | `collect.rs:552` | ✓ Present |
| `jump_to_body` in repeat_attempt | `repeat.rs:88` | ✓ Present |
| `jump_to_body` in repeat_check | `repeat.rs:115` | ✓ Present |
| `reentry_proofs` module declared | `mod.rs:14` | ✓ `#[cfg(kani)]` |
| `reentry_tests` module declared | `mod.rs:17` | ✓ `#[cfg(test)]` |
| `is_valid_step_state_transition` | `frame.rs:34` | ✓ Present |

The implementation fixes are complete and correct. The bridge mapping issues are purely documentation/reference errors in the obligation plan file.

---

## Bridge Evidence Gaps

| Gap | Detail |
|-----|--------|
| No Kani execution evidence | None of the 14 Kani-using POs have raw `cargo kani` output logs |
| No Verus execution evidence | PO-015 has no `verus` execution evidence |
| No `kani::cover` reachability evidence | `reentry_proofs.rs` uses `kani::cover!` but no execution logs confirm coverage |
| `step_state_from_u8` dead code | `reentry_proofs.rs:46` — defined but never called (the harnesses use `kani::any::<StepState>()` directly) |
| Test commands mismatch actual test names | PO commands reference `for_each_two_item_reentry` but actual test is `vb_y4pa_001_for_each_two_item_reentry` |
| Weak Kani assertions | Harnesses check `state.is_ok()` (readability) but not the actual state transition outcome |

---

## Required Remediation

### Phase 1: Fix Phantom File Targets
Update PO-011 through PO-014 target fields in proof-obligations.planned.jsonl from:
- `crates/vb_runtime/src/kani_y4pa_for_each_reentry.rs` → `crates/vb_runtime/src/primitives/reentry_proofs.rs`
- `crates/vb_runtime/src/kani_y4pa_reduce_reentry.rs` → `crates/vb_runtime/src/primitives/reentry_proofs.rs`
- `crates/vb_runtime/src/kani_y4pa_collect_reentry.rs` → `crates/vb_runtime/src/primitives/reentry_proofs.rs`
- `crates/vb_runtime/src/kani_y4pa_repeat_reentry.rs` → `crates/vb_runtime/src/primitives/reentry_proofs.rs`

### Phase 2: Fix Phantom Harness Names or Implement Missing Harnesses
- **PO-001**: Either implement `#[kani::proof] fn state_machine_succeeded_pending()` in `step_state.rs` OR update command to use existing unit tests
- **PO-002**: Either implement `#[kani::proof] fn mark_pending_harness()` in `frame.rs` OR update command to use an existing harness (e.g., `validate_transition_terminal_blocks_all`)
- **PO-003**: Either implement `#[kani::proof] fn jump_to_body_reset()` in `helpers.rs` OR update command to use existing unit tests

### Phase 3: Execute and Capture Evidence
Run all bridge-mapped commands and capture raw verifier output as proof evidence.

---

## Finding Summary

| # | Code | Severity | PO(s) | Description |
|---|------|----------|-------|-------------|
| 1 | BRDG/NEXIST/FILE/v1 | CRITICAL | PO-011-014 | 4 phantom Kani harness file targets |
| 2 | BRDG/NEXIST/HARNESS/v1 | CRITICAL | PO-001 | Phantom harness `state_machine_succeeded_pending` |
| 3 | BRDG/NEXIST/HARNESS/v1 | CRITICAL | PO-002 | Phantom harness `mark_pending_harness` |
| 4 | BRDG/NEXIST/HARNESS/v1 | CRITICAL | PO-003 | Phantom harness `jump_to_body_reset` |
| 5 | BRDG/MISMATCH/TEST/v1 | MEDIUM | PO-004-009 | Test command names use old naming convention |
| 6 | BRDG/DEADCODE/KANI/v1 | LOW | reentry_proofs.rs | `step_state_from_u8` defined but never called |
| 7 | BRDG/NOEVIDENCE/KANI/v1 | HIGH | All | No raw Kani execution logs in evidence |

---

## Verdict

**REJECTED** — While the 3 original blocking findings from attempt 1 are resolved, 7 new bridge mapping failures prevent approval:

1. **4 phantom file targets** (PO-011 through PO-014) point to nonexistent `kani_y4pa_*.rs` files when all harnesses are in `reentry_proofs.rs`
2. **3 phantom harness names** (PO-001 through PO-003) reference harnesses that do not exist in their target files
3. No raw verifier execution evidence exists for any obligation

The implementation fixes are sound, but the bridge obligation map must accurately reflect the actual file structure and harness naming before the verification commands can be executed and evidence captured.
