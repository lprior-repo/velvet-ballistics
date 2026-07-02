# Assurance Bundle

bead_id: vb-09aaz
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz
commit_or_change: qrtqslzp 0af593fc (vb-09aaz: p11-holzman-rust — abort write batch on stage_pending_action_index_op error); review-and-packaging change otxzkxmq 7d9dfb15

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| C1 — Abort-on-Fallible-Step Invariant (cross-method) | contract.md#C1 | `append_event.rs:137-143` new `if let Err(e) = ... { self.aborted = true; return Err(e); }` mirrors 28-instance pattern in `putters.rs`; PS-008 19 verified (post-fix mirrors bound via `production_inner/vb_vzcuf_PS_008_production.rs`); PO-09aaz-001 PASS | formal-verification-report.md; black-hat-review.md Phase 1 C1 | PASS |
| C2 — G8 Guard Precedence (8-guard order G1..G8) | contract.md#C2 | `append_event.rs:18-26` Guard Precedence doc-comment enumerates G1..G8; PS-008 lemma_guard_order_is_valid carries the order; PO-09aaz-001 PASS | black-hat-review.md Phase 1 C2 | PASS |
| C3 — Typed Error Propagation | contract.md#C3 | `JournalError::KeyCapacity` reused at `error/mod.rs:28-29`; no new variant; PO-09aaz-002 PASS (regression test) | black-hat-review.md Phase 1 C3 | PASS |
| C4 — Post-Condition: Aborted State on G8 Err | contract.md#C4 | `append_event.rs:42-49` Postconditions doc-comment documents abort invariant; regression test asserts 1-4; PO-09aaz-002 + PO-09aaz-004 PASS | black-hat-review.md Phase 1 C4 | PASS |
| C5 — No Partial Persistence (Master §49) | contract.md#C5 | `commit.rs:20-23` short-circuit; `all_or_nothing_commit_across_keyspaces` test at `t_append_event.rs:155-191`; `batch_append_event_index_key_error_aborts_commit` assertion 4 (`events_for_run(run).is_empty()`); PO-09aaz-004 PASS | black-hat-review.md Phase 1 C5 | PASS |
| C6 — Public API Stability | contract.md#C6 | signature diff: zero changes (`append_event`, `is_aborted`, `commit`); `JournalError::KeyCapacity` unit variant unchanged; PO-09aaz-005 PASS | black-hat-review.md Phase 1 C6 | PASS |
| C7 — Verus Spec Extension (PS-008, PS-009) | contract.md#C7 | PS-008: 19 verified, 0 errors; PS-009: 22 verified, 0 errors; production-binding gate 0 VACUUM, 71 WEAK_EXTERN; assume_specification at PS-008:180-199 carries G8 post-condition via `spec_state_preserved_except_aborted`; PO-09aaz-001 PASS | black-hat-review.md Phase 1 C7 | PASS |
| C8 — Test Coverage | contract.md#C8 | new test `batch_append_event_index_key_error_aborts_commit` at `t_append_event.rs:232-317` mirrors `t_putters_b.rs:177-209`; cargo test surface 195 batch + 10 t_append_event + 2 batch_index_key all pass; PO-09aaz-002 PASS | black-hat-review.md Phase 1 C8 | PASS |
| C9 — Doc-Comment Update | contract.md#C9 | `append_event.rs:18-26` Guard Precedence (G1..G8); `append_event.rs:33-49` Postconditions (KeyCapacity abort); PO-09aaz-005 PASS | black-hat-review.md Phase 1 C9 | PASS |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-09aaz-001 | verus (WEAK_EXTERN) | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-008.rs` | `state12-verus-PS-008.log` | 19 verified, 0 errors | none |
| PO-09aaz-001 (PS-009) | verus (WEAK_EXTERN) | `verus --crate-type=lib verification/verus/vb-vzcuf-PS-009.rs` | `state12-verus-PS-009.log` | 22 verified, 0 errors | none |
| PO-09aaz-001 (binding gate) | verus production-binding | `bash scripts/check-verus-production-binding.sh` | `state12-check-verus-production-binding.log` | 0 VACUUM, 71 WEAK_EXTERN | none |
| PO-09aaz-002 | rust-local (STRONG) | `cargo test -p vb_storage --lib t_append_event` | `state12-t_append_event.log` | 10 passed, 0 failed | none |
| PO-09aaz-003 | proptest (STRONG) | `cargo test -p vb_storage --lib batch` (195 tests; proptest-regression corpus included) | `state12-batch.log` | 195 passed, 0 failed | none |
| PO-09aaz-004 | persistence (STRONG) | `cargo test -p vb_storage --lib batch` (195 tests; `all_or_nothing_commit_across_keyspaces` covers OwnedWriteBatch atomicity) | `state12-batch.log` | 195 passed, 0 failed | none |
| PO-09aaz-005 | rust-local (STRONG) | `cargo test -p vb_storage --lib batch` (195 tests; doc-comment review) | `state12-batch.log` | 195 passed, 0 failed | none |
| PO-09aaz-001 (drift gate) | production-inner drift | `bash scripts/check-production-inner-drift.sh` | `state12-production-inner-drift.log` | 12 unrelated findings, zero in vb-09aaz blast radius | none |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| `cargo test -p vb_storage --lib batch_index_key` | user-narrowed | `state12-batch_index_key.log` | 2 passed, 1529 filtered |
| `cargo test -p vb_storage --lib t_append_event` | user-narrowed | `state12-t_append_event.log` | 10 passed, 1521 filtered |
| `cargo test -p vb_storage --lib batch` | user-narrowed | `state12-batch.log` | 195 passed, 1336 filtered |
| `cargo build -p vb_storage` | gate | (terminal output) | 4 crates compiled, 4.67s |
| `cargo clippy -p vb_storage` | prior state gate | `.beads/vb-09aaz/evidence/vb_storage-clippy.txt` | No issues found |
| `cargo fmt --check -p vb_storage` | prior state gate | `.beads/vb-09aaz/evidence/vb_storage-fmt.txt` | exit=0 |
| `cargo test -p vb_storage` (full) | prior state gate | `.beads/vb-09aaz/evidence/vb_storage-full-tests.txt` | 1672 passed (17 suites, 10.50s) |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof-plan review (16 rows) | `.beads/vb-09aaz/proof-plan-review.md` | STATUS: APPROVED | 0 findings (all 16 VLR rows accepted) |
| Proof review (gate alias) | `.beads/vb-09aaz/proof-review.md` | STATUS: APPROVED | 0 findings |
| Test-plan review | `.beads/vb-09aaz/test-plan-review.md` | STATUS: APPROVED | 0 findings |
| Formal verification | `.beads/vb-09aaz/formal-verification-report.md` | STATUS: APPROVED | 0 findings |
| Black-hat review | `.beads/vb-09aaz/black-hat-review.md` | STATUS: APPROVED | 0 findings |
| Defects | `.beads/vb-09aaz/defects.md` | empty | 0 findings |

## Findings Disposition

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| None | — | — | — | All four reviewer channels (proof-plan, test-plan, formal-verification, black-hat) returned zero findings. `defects.md` is empty. |

## Waivers And Deferred Work

Waivers and deferred work are not finding dispositions. Findings must use only canonical `finding/v1.disposition` values: `fixed_with_evidence`, `owner_approved_debt`, `owner_approved_no_action`, or `blocker`.

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| None | `formal-waivers.jsonl` is empty. No waivers required. All five proof obligations (PO-09aaz-001..005) closed under user-narrowed scope with PASS classification. | n/a | n/a | n/a |

### Pre-existing workspace-wide FAIL_GLOBAL classifications (NOT deferred work, NOT waivers)

These are reported honestly per the formal-verifier skill rule "Existing unrelated global failures: classify honestly; do not turn them into proof success":

- `bash scripts/check-production-inner-drift.sh` exits 1 with 12 drift findings in `verification/verus/production_inner/{action_replay_tracker, replay_invariants, unsupported_recovery_state}_production.rs` and `extern_{collect_lowering, idempotency_replay_tracker, ipc_runtime_transitions, recovery_verification, vb_rpch_seed_dimensions}.rs`. **Zero findings in `vb_vzcuf_PS_008_production.rs`, `vb_vzcuf_PS_009_production.rs`, or any `vzcuf`/`09aaz`-related mirror.** These are pre-existing workspace-wide failures owned by separate beads in the broader fleet.
- `bash scripts/verify-verus.sh` exits 1 with a Verus toolchain internal panic on `verification/verus/recovery_verification.rs` (DefId `CANNOT_RESUME_REASONS`). **PS-008 (19 verified) and PS-009 (22 verified) both verify cleanly when invoked directly.** The panic is in a different spec file unrelated to vb-09aaz.

Both classifications are **honestly FAIL_GLOBAL but zero impact on vb-09aaz closure**. They are tracked under separate bead owners and do not block vb-09aaz's bead-level STATUS: APPROVED.

## Truth Serum Audit

- report: `.beads/vb-09aaz/truth-serum-report.md`
- status: APPROVED

---

STATUS: APPROVED