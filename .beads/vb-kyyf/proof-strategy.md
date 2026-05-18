# vb-kyyf Proof Strategy (Attempt 3 — COMMAND REPAIR)

## Rerun Note

**ATTEMPT 3 — COMMAND REPAIR AFTER STATE 11 REJECTION.**
State 11 formal-verifier rejected obligations due to invalid `-p workspace_tests` package name. Controller patched `proof-obligations.planned.jsonl` to use `-p velvet-ballastics-workspace-tests`. This attempt repairs proof-strategy.md to match.

**Repair summary:**
- Line 54: `cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism` → `cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism`
- Line 58: `cargo test -p workspace_tests --test vb_hxm0_acceptance_catalog` → `cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog`
- proof-obligations.planned.jsonl already correctly patched by controller (verified: 0 invalid commands)

---

## Strategic Assessment

**Bead**: vb-kyyf (Cross-run determinism and reproducibility BDD)
**State**: 4 (proof-planning)
**Rerun from**: Attempt 2 — all prior proof-planning artifacts discarded

### Discovery Evidence (Mandatory Gate)

```
DISCOVERY: crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs   MISSING (BDD-KYYF-001/003/006 target)
DISCOVERY: crates/vb_storage/tests/replay_resume.rs                          EXISTS (BDD-KYYF-002 target)
DISCOVERY: crates/vb_storage/tests/recovery_bdd_tests.rs                     EXISTS (BDD-KYYF-004 target)
DISCOVERY: crates/vb_codegen/src/tests.rs                                    EXISTS (BDD-KYYF-005 target)
DISCOVERY: crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs        EXISTS (BDD-KYYF-007 target)
DISCOVERY: verification/tla/VbKyyfReplayDeterminism.tla                       MISSING (TLA-KYYF-001 target)
DISCOVERY: crates/workspace_tests/src/vb_kyyf_normalization.rs                MISSING (VERUS-KYYF-001 target)
DISCOVERY: verification/verus/ (existing files: idempotency_replay_tracker.rs, recovery_verification.rs, etc.)
```

---

## Verification Lane Selection

### Lane 1: BDD Execution (MANDATORY — public surface evidence)

**Why required**: INV-001 (public-surface-only), INV-007 (evidence traceability), POST-001..POST-006 require executable Given/When/Then scenarios through documented public surfaces. BDD is the release gate evidence for contract clause observability.

**Coverage**:
- BDD-KYYF-001 → `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` **(FILE MISSING — blocked until State 5)**
- BDD-KYYF-002 → `crates/vb_storage/tests/replay_resume.rs` (EXISTS)
- BDD-KYYF-003 → `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` **(FILE MISSING — blocked until State 5)**
- BDD-KYYF-004 → `crates/vb_storage/tests/recovery_bdd_tests.rs` (EXISTS)
- BDD-KYYF-005 → `crates/vb_codegen/src/tests.rs` (EXISTS)
- BDD-KYYF-006 → `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` **(FILE MISSING — blocked until State 5)**
- BDD-KYYF-007 → `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` (EXISTS)

**Commands**:
```
cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism
cargo test -p vb_storage --test replay_resume
cargo test -p vb_storage --test recovery_bdd_tests
cargo test -p vb_codegen
cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog
```
**Owner state**: 8 (tests authored in State 5, pass in State 8)
**Rerun from**: 5

---

### Lane 2: TLA+ (MANDATORY — temporal/replay/recovery)

**Why required**: Contract explicitly assigns INV-003 (journal sequence well-formedness), INV-004 (digest mismatch never continues), INV-005 (no unsafe side-effect reexecution), POST-002 (replay reproducibility), POST-003 (stable blocked outcome), POST-004 (stable error convergence) to TLA+. These are state-machine temporal properties.

**Coverage**: Replay/recovery state transitions, action class policy, typed error state convergence, unsupported generated subset fail-closed.

**Run command**: `tlc -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla`

**Status**: `blocked_tooling` — spec file `verification/tla/VbKyyfReplayDeterminism.tla` does NOT exist in workspace. Lane activates after State 5 spec authoring.

**Owner state**: 5
**Rerun from**: 3

---

### Lane 3: Verus (MANDATORY — pure normalization kernel)

**Why required**: Contract assigns PRE-004 (normalization whitelist exhaustive), INV-002 (semantic delta rejection), POST-001/POST-002/POST-005 (reflexive/symmetric comparison) to Verus. Pure Rust normalization/comparison is the small trusted kernel.

**Coverage**: spec_allowed_metadata_delta, spec_normalized_observation_eq, proof_normalization_rejects_semantic_delta, proof_normalized_equality_is_stable, proof_journal_signature_monotonic_contiguous.

**Run command**: `moon run :verify-proof`

**Status**: `blocked_tooling` — normalized observation kernel target file does NOT exist. Implementation must select location in State 5.

**Owner state**: 5
**Rerun from**: 3

---

### Lane 4: GATE (MANDATORY — release closure)

**Why required**: POST-006 (runner output traceability), INV-007 (evidence artifact traceability) require canonical workspace release gate.

**Run command**: `moon ci`

**Expected evidence**: moon ci exits 0 for release closure, or formal-verifier records only unrelated DEFERRED_GLOBAL failures after all vb-kyyf scoped commands pass.

**Owner state**: 11
**Rerun from**: 8

---

## Pruned / Waived Lanes

| Lane | Reason | Waiver Trigger |
|------|--------|----------------|
| Kani | Verus spec functions cover normalization kernel; BDD runner provides execution panic-freedom. No independent Kani obligation in proof-obligations.jsonl. | Re-activate only if proof-reviewer finds bounded panic-freedom gap not covered by Verus/BDD |
| proptest | BDD runner fixture isolation covers PRE-002. No independent proptest obligation in proof-obligations.jsonl. | Re-activate only if proof-reviewer finds combinatorial isolation gap not covered by BDD fixture design |
| Fuzz | Not release gate per verification-layers.md; deferred to formal-verifier for timeout policy | N/A — deferred |
| Lean/Aeneas/Hax | Waived in contract (WAIVE-THM-001); Verus+TLA+ cover relevant math | N/A |

---

## Summary: Cheapest Sufficient Lane Set

| Lane | Obligations | Command | Owner State | Rerun From | Status |
|------|------------|---------|-------------|------------|--------|
| BDD | BDD-KYYF-001..007 (PO-001..007) | See Lane 1 commands above | 8 | 5 | blocked_file_missing |
| TLA+ | INV-003,INV-004,INV-005,POST-002,POST-003,POST-004 (PO-008) | `tlc -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla` | 5 | 3 | blocked_tooling |
| Verus | PRE-004,INV-002,POST-001,POST-002,POST-005 (PO-009) | `moon run :verify-proof` | 5 | 3 | blocked_tooling |
| GATE | POST-006,INV-007 (PO-010) | `moon ci` | 11 | 8 | planned |

---

## Key Assumptions

1. TLA+ spec `verification/tla/VbKyyfReplayDeterminism.tla` authored in State 5.
2. Verus normalized observation kernel location selected by implementation in State 5.
3. `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs` authored in State 5 (for BDD-KYYF-001/003/006).
4. CLI binary test harness shape confirmed by downstream test state (contract Open Question 1).
5. `compare_generated_to_ir` API confirmed by implementation (contract Open Question 2).

---

## Lane Critical Path

```
State 5: Author TLA+ spec + cfg → TLA lane activates
State 5: Implementation selects Verus normalization kernel location → Verus lane activates
State 5: Author vb_kyyf_cross_run_determinism.rs → BDD lane fully unblocked
State 5: Run TLA+ lane → evidence
State 5: Run Verus lane → evidence
State 8: Run BDD lane → evidence
State 11: Run GATE lane → release closure
```
