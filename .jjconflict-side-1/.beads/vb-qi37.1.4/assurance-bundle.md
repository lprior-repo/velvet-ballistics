# Assurance Bundle — vb-qi37.1.4

bead_id: vb-qi37.1.4
source_checkout: /home/lewis/src/Velvet-ballistics
isolated_workspace: /home/lewis/src/vb-qi37-1-4-fresh
commit_or_change: GAP-2 fix applied (line 84 of recovery.rs) + DEFECT-1 fix applied (test-plan.md:77)

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| Fail-closed when slot_taint unsupported | POST-001, INV-GAP1-001 | VERUS-INV-RC-002 (PASS) | proof-review.md:APPROVED | COVERED |
| Fail-closed when pending_actions unsupported regardless of is_empty | POST-002, INV-GAP2-001 | VERUS-INV-RC-004 (PASS) | proof-review.md:APPROVED | COVERED |
| Fail-closed when slot_values unsupported | INV-RC-005 | VERUS-INV-RC-005 (PASS) | proof-review.md:APPROVED | COVERED |
| verify_digests Full verifies action ABI digests | POST-003, INV-RC-008 | VERUS-INV-RC-008 (PASS) | proof-review.md:APPROVED | WAIVED (GAP-3) |
| verify_digests Full verifies policy digests | POST-003, INV-RC-009 | VERUS-INV-RC-009 (PASS) | proof-review.md:APPROVED | WAIVED (GAP-3) |
| GAP-2 fix: pending_actions guard fires regardless of is_empty | INV-GAP2-001 | VERUS-GAP2-001 (PASS) | black-hat-review.md:APPROVED | FIXED |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| VERUS-GAP1-001 | verus | verus verification/verus/recovery_verification.rs | verification-ledger.jsonl | PASS | No |
| VERUS-GAP2-001 | verus | verus verification/verus/recovery_verification.rs | verification-ledger.jsonl | PASS | No |
| VERUS-GAP3-001 | verus | verus verification/verus/recovery_verification.rs | verification-ledger.jsonl | PASS | No |
| VERUS-GAP3-002 | verus | verus verification/verus/recovery_verification.rs | verification-ledger.jsonl | PASS | No |
| WAIVER-GAP3-ABI | waiver | N/A | formal-verification-report.md | WAIVED | Yes (expiry 2026-07-01) |
| WAIVER-LEAN | waiver | N/A | formal-verification-report.md | WAIVED | Yes |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| cargo test -p vb_runtime --lib | cargo test | verification-ledger.jsonl | FAIL_LOCAL (verus dependency not on crates.io) |
| cargo test -p vb_storage --lib -- recovery | cargo test | verification-ledger.jsonl | FAIL_LOCAL (verus dependency not on crates.io) |
| cargo check -p vb_storage | cargo check | Active context | FAIL_LOCAL (verus dependency blocks workspace resolution) |
| cargo clippy -- -D warnings | cargo clippy | Active context | UNRUN (verus dependency blocks workspace resolution) |

**Tooling Limitation**: The `verus = "^1"` dependency is not available on crates.io. This is a pre-existing environmental issue that blocks all cargo-based verification commands at the workspace level.

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| contract-verification-review | contract-verification-review.md | APPROVED | Contract parity confirmed |
| proof-review | proof-review.md | APPROVED | 7 Verus proofs verified, 0 errors |
| test-plan-review | test-plan-review.md | APPROVED WITH MINOR FINDINGS | MINOR-1, MINOR-2 documented |
| formal-verification-report | formal-verification-report.md | REJECTED | GAP-2 bug identified (FIXED) |
| black-hat-review | black-hat-review.md (this state) | APPROVED | DEFECT-1: test fixed |

---

## Defects and Blockers

| Defect | Severity | Type | Description | Status |
|---|---|---|---|---|
| DEFECT-1 | BLOCKING | test | test-plan.md:73-80 expected `Ok` but POST-002 requires `Err` after GAP-2 fix | FIXED |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| GAP-3: Action ABI digest verification | Not implemented; formal waiver obtained | TBD | 2026-07-01 | Verus spec documents the deferred behavior |
| GAP-3: Policy digest verification | Not implemented; formal waiver obtained | TBD | 2026-07-01 | Verus spec documents the deferred behavior |
| verus dependency | Not on crates.io; blocks cargo gates | TBD | Unknown | Verus verification passes independently |

---

## Truth Serum Audit

- report: `.beads/vb-qi37.1.4/truth-serum-report.md`
- status: UNVERIFIED — tooling limitation prevents command execution in active context
- DEFECT-1 fix verified by code inspection

**Note**: Due to pre-existing `verus = "^1"` dependency not being on crates.io, no cargo-based verification commands can run in the active execution context. The GAP-2 fix (line 84 of recovery.rs) and DEFECT-1 fix (test-plan.md:77) were verified by code inspection only, not by command execution.

---

## GAP-2 Fix Verification

**Location**: `crates/vb_runtime/src/recovery.rs:84`

**User-reported fix verified**:
- Before: `|| (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)` — BUGGY
- After: `|| seed.unsupported.pending_actions` — CORRECT

**Code inspection evidence**:
- Line 78: Verus spec correctly captures POST-001, POST-002
- Line 84: Fix correctly removes `is_empty()` check, enforcing POST-002 regardless of pending_actions state

**Verification method**: Code inspection (command execution blocked by tooling limitation)

---

## DEFECT-1 Fix Verification

**Location**: `.beads/vb-qi37.1.4/test-plan.md:73-80`

**Fix applied**:
- Scenario name changed from `reject_returns_ok_when_pending_actions_unsupported_but_empty` to `reject_returns_err_when_pending_actions_unsupported_but_empty`
- Line 77 changed from `Then: returns Ok(())` to `Then: returns Err(RuntimeError::InvalidRecoveryHydration)`
- Note updated from documenting buggy behavior to documenting correct POST-002 behavior

**Code inspection evidence**:
- test-plan.md:77 now expects `Err(RuntimeError::InvalidRecoveryHydration)` as required by POST-002

**Verification method**: Code inspection

---

*assurance-bundle.md — State 13 evidence packaging for vb-qi37.1.4 — ALL DEFECTS FIXED*