# Contract Verification Review — vb-rpch (Attempt 3)

## STATUS: NOT ADEQUATE

---

## Executive Summary

The proof obligations planned in PO-VB-008 through PO-VB-013 are **BLOCKED** — the TLA+ spec `RecoveryReplayFull.tla` does not implement the 6 required invariants (TLA-001 through TLA-006). GAP-3 and TerminalStateMismatch waivers are SOUND. Verus specs exist but lack execution confirmation. Kani harness partially complies with GOD RULE 1. Two traceability entries have unresolved TODOs.

---

## Defect Register

### BLOCKING DEFECT 1 — TLA+ Invariant Mismatch (TLA-001 through TLA-006)

**Severity**: BLOCKING / high risk

**Finding**: `RecoveryReplayFull.tla` (156 lines) does NOT implement the 6 invariants required by the contract:

| Contract Clause | Required Invariant | Actually Implemented |
|---|---|---|
| TLA-001 | `ReplaySeqOrder` — events in ascending seq; steps monotonic per attempt | `StepOrderInvariant` — partial step ordering only; seq ordering NOT enforced |
| TLA-002 | `TailCausalAfterSnapshot` — all tail seq > snapshot seq | **NOT IMPLEMENTED** — no snapshot/tail modeling |
| TLA-003 | `OnlyIncompleteRuns` — only runs without terminal event of max attempt returned | **NOT IMPLEMENTED** — no DiscoverIncomplete action |
| TLA-004 | `NoResolvedReExecution` — resolved (action, step) never in replay output | **NOT IMPLEMENTED** — tracker doesn't filter replay output |
| TLA-005 | RecoveryError state machine exhaustiveness | **NOT IMPLEMENTED** — no RecoveryError variant modeling |
| TLA-006 | Digest verification stage ordering | **NOT IMPLEMENTED** — no CheckDigest action or level parameter |

**Actual Invariants in `RecoveryReplayFull.tla`**:
- `NoDivergenceInvariant` — trivial: `divergence_detected = FALSE`
- `StepOrderInvariant` — step indices monotonically non-decreasing
- `NoDoubleScheduling` — no duplicate ActionScheduled for same (action, step)
- `ActionSafety` — `completed_actions ∩ failed_actions = {}`

**Gap Notes confirm this**: PO-VB-008 gap_note: "TLA-001 NOT currently modelled. Existing spec covers idempotency only." PO-VB-009: "TLA-002 NOT currently modelled. RecoveryReplayFull.tla does not exist." etc.

**Evidence artifact exists but is insufficient**: `specs/RecoveryReplayFull.tla` was created but with wrong invariants. TLC simulation (21,404 states, seed 1365916096378164662) passed on the wrong properties — proving safety of invariants that don't cover the contract clauses.

**Remediation**: Rewrite `RecoveryReplayFull.tla` with the 6 required invariants, or create `RecoveryReplayFull.tla` with `TailCausalAfterSnapshot`, `OnlyIncompleteRuns`, `NoResolvedReExecution`, `RecoveryErrorExhaustive`, `CheckDigest` action with level parameter, and proper `ReplaySeqOrder` enforcing seq ascending order. Then re-run TLC model checking and update proof-evidence.md with INVARIANT declarations from the .cfg file.

---

### DEFECT 2 — Kani Harness Uses Deterministic Events, Not `kani::any()` for JournalEvent Fields

**Severity**: HIGH / GOD RULE 1 violation

**Location**: `evidence/kani/kani_recovery_hydrate.rs`

**Finding**: The harness generates events via deterministic modulo construction:
```rust
let event: vb_storage::JournalEvent = if i % 5 == 0 {
    vb_storage::JournalEvent::StepStarted { step: StepIdx::new((i % 20) as u64), attempt: Some(1) }
} // ... etc
```
Only `run_id` and `len` use `kani::any()`. The actual JournalEvent content is NOT arbitrary — it's a fixed deterministic sequence.

**GOD RULE 1 mandate**: "Kani verification harnesses MUST NOT hardcode structural inputs... You MUST implement and use kani::Arbitrary for core structures, or write safe, exhaustive generator harnesses using kani::any()."

**Compensating factor**: Events are generated in pairs (events1, events2 = identical by construction) to prove determinism. This proves determinism for that specific pattern but does NOT prove absence of panic on arbitrary JournalEvent content.

**Remediation**: Generate each `JournalEvent` field independently using `kani::any()` for step, action, slot, attempt, etc. Enforce preconditions via `kani::assume()`.

---

### DEFECT 3 — Verus Specs Lack Execution Confirmation

**Severity**: MEDIUM / GOD RULE 2 violation

**Location**: `evidence/specs/recovery_state_verus.v`, `hydration_verus.v`, `replay_core_verus.v`

**Finding**: Verus specs are written as standalone `#[verus] pub mod` files referencing actual Rust types (good). However, `proof-evidence.md` states "COMPLETE" without showing:
1. Actual `verus` command output
2. Confirmation that the specs verified against the real Rust code (not just type-checked)

**GOD RULE 2 mandate**: "Verus `proof fn` and `spec fn` models MUST mathematically bind to the actual Rust implementations... The implementation functions must use `requires` and `ensures` to guarantee they satisfy the model."

**No evidence of**: `verus crates/vb_storage/src/recovery/types.rs ...` execution output, error count, or pass/fail status.

**Remediation**: Run `verus` on the source files and record the actual output showing 0 errors.

---

### DEFECT 4 — Two Traceability Entries Have Unresolved TODO Proofs

**Severity**: MEDIUM

**Location**: `traceability-matrix.jsonl`

**Finding**:
- Line 3: `PRE-003` proof: `"VERUS-POST-001-TODO"` — unresolved TODO marker
- Line 4: `PRE-004` proof: `"VERUS-PRE-004-TODO"` — unresolved TODO marker

Both have tests but no actual Verus proof artifact.

**Remediation**: Either produce the Verus proof or record an explicit waiver with rationale.

---

## Sound Waivers Confirmed

### GAP-3 Waivers (SOUND)

| Waiver | Rationale | Status |
|---|---|---|
| WAIVER-GAP3-ABI | ActionAbiMismatch not reachable via public API; `expected_action_abi_digests` lookup not implemented | **SOUND** — contract non-goal explicitly stated; tracked in vb-ty9 |
| WAIVER-GAP3-POL | PolicyDigestMismatch not reachable via public API; `expected_policy_digests` lookup not implemented | **SOUND** — contract non-goal explicitly stated; tracked in vb-ty9 |

### DEFERRED_GLOBAL Waiver (SOUND)

| Waiver | Rationale | Status |
|---|---|---|
| WAIVER-TERM-MISMATCH | TerminalStateMismatch has no public API parameter; DEFERRED_GLOBAL B-017 | **SOUND** — no expected-terminal parameter in `recover_runtime_summary`/`recover_runtime_frame_seed`; deferred to B-017 |

---

## Complete Traceability Assessment

| Clause | Proof Obligation | Status |
|---|---|---|
| PRE-001 | PO-VB-004 (Verus) + PO-VB-014 (Kani) | ⚠️ Kani harness non-compliant (Defect 2); Verus unconfirmed (Defect 3) |
| PRE-002 | PO-VB-005 (Verus) + PO-VB-015 (Kani) | ⚠️ Same as PRE-001 |
| PRE-003 | VERUS-POST-001-TODO | ❌ Unresolved TODO (Defect 4) |
| PRE-004 | VERUS-PRE-004-TODO | ❌ Unresolved TODO (Defect 4) |
| PRE-005 | Tests only | ✅ Adequate (precondition testable) |
| POST-001 | TLA-REPLAY-001 | ❌ TLA-001 not implemented in spec (Defect 1) |
| POST-002 | TLA-REPLAY-001 | ❌ TLA-001 not implemented in spec (Defect 1) |
| POST-003 | TLA + WAIVER-GAP3-ABI/POL | ❌ TLA-001/TLA-006 not implemented (Defect 1) |
| POST-004 | TLA-REPLAY-001 | ❌ TLA-001 not implemented (Defect 1) |
| POST-005 | VERUS-INV-003 + TLA | ⚠️ TLA-001 not implemented (Defect 1); Verus unconfirmed (Defect 3) |
| POST-006 | VERUS-PRE-001 + TLA | ⚠️ TLA-001 not implemented (Defect 1); Verus unconfirmed (Defect 3) |
| POST-007 | VERUS-PRE-002 + TLA | ⚠️ TLA-001 not implemented (Defect 1); Verus unconfirmed (Defect 3) |
| POST-008 | TLA-INCOMPLETE-001 | ❌ TLA-003 not implemented (Defect 1) |
| POST-009 | VERUS-POST-009 + TLA + KANI | ⚠️ TLA-004 not implemented (Defect 1); Verus/Kani issues above |
| POST-010 | VERUS-INV-004 | ⚠️ Verus unconfirmed (Defect 3) |
| INV-001 | static-scan | ✅ Adequate (enum exhaustiveness) |
| INV-002 | VERUS-INV-002 | ⚠️ Verus unconfirmed (Defect 3) |
| INV-003 | VERUS-INV-003 | ⚠️ Verus unconfirmed (Defect 3) |
| INV-004 | VERUS-INV-004 | ⚠️ Verus unconfirmed (Defect 3) |
| INV-005 | VERUS-INV-005 | ⚠️ Verus unconfirmed (Defect 3) |
| INV-006 | TLA-INCOMPLETE-001 | ❌ TLA-003 not implemented (Defect 1) |
| ERR-* | Various | See matrix — GAP/deferrals are sound |

---

## Verdict

**NOT ADEQUATE** for State 6 (Proof and Contract Review) gate.

The TLA+ spec must be rewritten to implement TLA-001 through TLA-006. The Kani harness must use `kani::any()` for JournalEvent fields. Verus execution must be confirmed with actual output. TODO markers must be resolved or explicitly waived.

---

## Required Remediation

1. **Rewrite `RecoveryReplayFull.tla`** to implement the 6 required invariants (TLA-001 through TLA-006) with proper modeling of snapshot_plus_tail, DiscoverIncomplete, CheckDigest, and replay event ordering. Update `.cfg` with correct INVARIANT declarations.
2. **Fix Kani harness** to use `kani::any()` for individual JournalEvent fields. Use `kani::assume()` to enforce preconditions.
3. **Execute Verus** on the source files and record the actual 0-error output.
4. **Resolve or explicitly waive** VERUS-POST-001-TODO and VERUS-PRE-004-TODO.
5. **Update proof-evidence.md** with corrected TLC INVARIANT list matching the contract clauses.
