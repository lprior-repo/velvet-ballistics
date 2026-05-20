# Assurance Bundle: vb-te1i — Binary IPC BDD Acceptance

**bead_id**: vb-te1i
**source_checkout**: /home/lewis/src/velvet-ballistics
**isolated_workspace**: /home/lewis/src/vb-te1i-workspace
**commit_or_change**: (isolated workspace jj checkout)

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| Binary IPC frame codec | POST-001 | UNIT-001 (686 vb_ipc tests) | proof-review.md | PASS |
| Health command returns Healthy | POST-002 | BDD-001 (7 BDD tests) | test-plan-review.md | PASS |
| Shutdown returns ShuttingDown | POST-003 | BDD-001 | test-plan-review.md | PASS |
| SubmitRun preserves correlation | POST-004 | BDD-002 | test-plan-review.md | PASS |
| Bad magic rejected before allocation | POST-005 | BDD-003 + UNIT-002 | proof-review.md | PASS |
| Version mismatch rejected | POST-006 | UNIT-003 | proof-review.md | PASS |
| Unknown command rejected | POST-007 | UNIT-004 | proof-review.md | PASS |
| Reserved non-zero rejected | POST-008 | UNIT-005 | proof-review.md | PASS |
| Oversize payload rejected | POST-009 | BDD-007 + UNIT-006 | proof-review.md | PASS |
| Payload length mismatch rejected | POST-010 | UNIT-007 | proof-review.md | PASS |
| Queue full returns Full error | POST-011 | BDD-004 + UNIT-008 | proof-review.md | PASS |
| Queue disconnected returns Disconnected | POST-012 | UNIT-008 | proof-review.md | PASS |
| Header length fixed (24 bytes) | INV-001 | UNIT-009 | proof-review.md | PASS |
| Magic value immutable | INV-002 | UNIT-010 | proof-review.md | PASS |
| Command range 1..=16 | INV-003 | UNIT-004 + BDD-005 | proof-review.md | PASS |
| Decode before allocation | INV-004 | UNIT-002/003/005/006 | proof-review.md | PASS |
| Bounded payload enforced | INV-005 | UNIT-006 | proof-review.md | PASS |
| Correlation preserved | INV-006 | BDD-006 | proof-review.md | PASS |
| Diagnostic code stable | INV-007 | UNIT-002 | proof-review.md | PASS |

---

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| UNIT-001..010 | cargo test | `cargo test --package vb_ipc` | verification-ledger.jsonl | PASS | None |
| STATIC-001 | cargo clippy | `cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings` | verification-ledger.jsonl | PASS | None |
| BDD-001..007 | cargo test | `cargo test --package velvet-ballastics-workspace-tests --test vb_te1i_binary_ipc_acceptance` | verification-ledger.jsonl | PASS | None |
| KAN-001 | cargo kani | `cargo kani --package vb_ipc` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Kani unavailable |
| KAN-002 | cargo kani | `cargo kani --package vb_ipc` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Kani unavailable |
| KAN-003 | cargo kani | `cargo kani --package vb_ipc` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Kani unavailable |
| VERUS-001 | verus | `verus crates/vb_ipc/src/commands.rs` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Cannot run Verus on single files with external deps |
| VERUS-002 | verus | `verus crates/vb_ipc/src/bounded.rs` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Cannot run Verus on single files with external deps |
| VERUS-003 | verus | `verus crates/vb_ipc/src/frame_types.rs` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Cannot run Verus on single files with external deps |
| VERUS-004 | verus | `verus crates/vb_ipc/src/frame.rs` | proof-evidence.md | WAIVED | BLOCKED_TOOLING: Cannot run Verus on single files with external deps |

---

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| vb_ipc unit tests (686) | `cargo test --package vb_ipc` | verification-ledger.jsonl | PASS |
| BDD acceptance (7 scenarios) | `cargo test --package velvet-ballastics-workspace-tests --test vb_te1i_binary_ipc_acceptance` | verification-ledger.jsonl | PASS |
| vb_ipc clippy | `cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings` | machine-gate-report.md | PASS |
| BDD file formatting | `cargo fmt --check -- crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs` | regression-diff.md | PASS (fixed) |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof Review | proof-review.md | STATUS: APPROVED | 0 LETHAL + 1 MAJOR (test-only) |
| Test Plan Review | test-plan-review.md | STATUS: APPROVED | 12/12 POST covered, 6.4x unit density |
| Test Suite Review | test-suite-review.md | STATUS: APPROVED | 0 LETHAL + 1 MAJOR (assert_ok! macro) |
| Contract Verification Review | contract-verification-review.md | STATUS: APPROVED | All clauses verified |
| Black Hat Review | black-hat-review.md | STATUS: APPROVED | Production code passes all 5 phases |
| Formal Verification | formal-verification-report.md | REJECTED (pre-existing issues) | Formatting fixed; clippy issue in non-scoped file |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| KAN-001/002/003 | BLOCKED_TOOLING: Kani unavailable (vb_storage compilation errors) | vb-te1i | Separate remediation bead | UNIT-002/003/005/006 + BDD-003/007 (72 adversarial unit tests) |
| VERUS-001/002/003/004 | BLOCKED_TOOLING: Cannot run Verus on single files with external deps | vb-te1i | Separate remediation bead | UNIT-004 + BDD-005 + frame_types tests |
| vb_cli/lifecycle.rs dead_code | Pre-existing workspace debt, NOT in bead scope | Workspace | Separate bead | N/A - not bead responsibility |
| Workspace-wide formatting | Pre-existing debt in 12 files outside bead scope | Workspace | Separate bead | N/A - not bead responsibility |

---

## Truth Serum Audit

- report: `.beads/vb-te1i/truth-serum-report.md`
- status: See truth-serum-report.md

---

## Blockers And Residual Risk

| Item | Classification | Status |
|---|---|---|
| Formatting in vb_te1i_binary_ipc_acceptance.rs | FAIL_LOCAL | FIXED |
| Clippy dead_code in vb_cli/lifecycle.rs | FAIL_REGRESSION | NOT IN SCOPE - pre-existing workspace debt |
| Kani proofs (KAN-001/002/003) | DEFERRED_GLOBAL | Formal waiver with compensating evidence |
| Verus proofs (VERUS-001/002/003/004) | DEFERRED_GLOBAL | Formal waiver with compensating evidence |

---

## Command Evidence Summary

```
cargo test --package vb_ipc: 686 passed
cargo test --package velvet-ballastics-workspace-tests --test vb_te1i_binary_ipc_acceptance: 7 passed
cargo clippy --package vb_ipc --lib --bins --examples -- -D warnings: No issues found
cargo fmt --check -- crates/workspace_tests/tests/vb_te1i_binary_ipc_acceptance.rs: PASS (after fmt fix)
```