# Contract Verification Review — vb-core-strict-ack-ordering

**Bead ID**: vb-core-strict-ack-ordering
**Reviewer**: proof-reviewer (contract-verification-reviewer)
**Date**: 2026-05-15
**Files Reviewed**:
- `contract.md`
- `tla-spec.md`
- `lean-contract.md`
- `verification-layers.md`
- `proof-obligations.planned.jsonl`
- `traceability-matrix.jsonl`

---

## Command Evidence

```bash
$ jq -c . /tmp/vb-ws/vb-core-strict-ack-ordering/.beads/vb-core-strict-ack-ordering/proof-obligations.planned.jsonl >/dev/null && echo "valid"
valid
$ jq -c . /tmp/vb-ws/vb-core-strict-ack-ordering/.beads/vb-core-strict-ack-ordering/traceability-matrix.jsonl >/dev/null && echo "valid"
valid
```

---

## Contract Coverage Assessment

### All Contract Clauses Traced?

| Clause | Traced in obligations.jsonl? | Traced in traceability.jsonl? |
|--------|-------------------------------|------------------------------|
| ACK-ORDER-001 | ✓ | ✓ |
| ACK-ORDER-002 | ✓ | ✓ |
| PRE-001/002/003/004 | ✓ | ✓ |
| POST-001 through POST-010 | ✓ | ✓ |
| INV-001 through INV-010 | ✓ | ✓ |
| DISPATCH-001, DISPATCH-002 | ✓ | ✓ |
| FAIL-001, FAIL-002 | ✓ | ✓ |
| RECOVERY-001, RECOVERY-002, RECOVERY-003 | ✓ | ✓ |

**Coverage verdict**: All contract clauses have at least one proof obligation. ✓

---

## Layer Fit Assessment

### DEF-VERUS-DEAD: Verus Lane Non-Functional (All Specs Commented)

**Rule**: `verus_first` — "For Rust-local pure/core logic, Verus is the default required proof layer. Reject high, proof, critical, unsafe-boundary, changed-api, parser/codec, Rust-local state-transition, or data-invariant obligations that omit Verus unless a waiver names the Verus limitation, owner, expiry, limitation, and compensating evidence."

**Finding**: 6 critical Verus obligations are planned (VERUS-DM-001/002/003/004, VERUS-JA-001/002) but their artifacts are fully commented-out `verus!` blocks. This is not a waiver — it is a non-functional artifact. The Verus lane produces zero evidence.

**Impact**:
- VERUS-DM-001 (critical, POST-001/002/INV-002): `verify_ack_after_persist` unverified
- VERUS-DM-002 (high, INV-001, PRE-001/002/003): `DURABILITY_MATRIX` completeness unverified
- VERUS-DM-003 (high, POST-009/010, INV-004): `EventSeq` monotonicity unverified
- VERUS-DM-004 (high, INV-005/006): `AckPoint::BeforeJournalAppend` unreachability unverified
- VERUS-JA-001 (critical, POST-006): `append_strict` postcondition unverified
- VERUS-JA-002 (high, POST-007): `append_journaled` no-barrier unverified

**Severity**: LETHAL — central contract (ACK-ORDER-001/002) cannot be approved without Verus evidence.

### DEF-TLA-BARRIER: TLA-BARRIER-001 Model Gap (IF TRUE Makes Barrier Always Succeeds)

**Rule**: `tla_temporal_default` — "tla-spec.md is mandatory as the temporal model boundary. Reject workflow, protocol, scheduler, retry, claim/lease, lifecycle, concurrent, or state-over-time clauses that omit TLA+ unless a waiver names owner, reason, expiry, limitation, and compensating evidence."

**Finding**: `JournalBarrier.tla` `AppendStrict` action uses `IF TRUE`, making the persist success path always taken. The `persistError` action is a separate transition that does not model the case where append succeeds but persist fails within `AppendStrict`. This is a modeling gap.

**tla-spec.md claims**: ACK-ORDER-TLA-001 defines `AppendStrict` as atomic append-then-persist. The implementation in `JournalBarrier.tla` line 56 says `IF TRUE` with comment "persist always succeeds for this obligation."

**Severity**: MAJOR — I1 (`ackSent => persistedEvents = journaledEvents`) is verified only for the success path. Failure path not exercised in the model. The temporal contract (T1, T2) may not hold under all failure interleavings.

### DEF-TLA-QUEUE: TLA-QUEUE-001 CompleteFlush Gap

**Finding**: `QueuedStrictFlush.tla` `CompleteFlush` action (lines 87-97) has no pre-condition requiring all queued strict events to be appended before `strictFlushComplete' = TRUE`. QF1 (`strictFlushComplete => all queued events appended`) checks the resulting state where `queue' = {}`, making the quantifier vacuously true.

**tla-spec.md claims**: ACK-ORDER-TLA-003 (QF1) says `strictFlushComplete = TRUE` implies all queued events were appended. The model does not enforce this.

**Severity**: MAJOR — DISPATCH-002 (queued flush ordering) is not reliably verified.

### DEF-KANI-DISPATCH: KANI-DISPATCH-001/002 Vacuous Harnesses

**Finding**: Both dispatch harnesses use `kani::any()` followed by a trivially-true assertion. No actual dispatch behavior is verified.

**Rule**: `defense_depth` — "Rust-local pure deterministic critical clauses require Verus plus Rust-realization evidence such as proptest, Kani, fuzzing, or a gauntlet lane."

**Severity**: MAJOR — DISPATCH-001 is unverified by Kani.

### DEF-KANI-RECOVERY: KANI-HYDRATE-001, KANI-REPLAY-001 Placeholder Harnesses

**Finding**: Both harnesses are `kani::assert(true, ...)` placeholders. Zero verification evidence for RECOVERY-002/003.

**Severity**: LETHAL for RECOVERY-002/003 coverage.

### DEF-INTEGRATION: Integration Tests are Stubs

**Finding**: All four integration tests only construct error variants, do not call runtime, and do not verify behavioral properties.

**Rule**: `defense_depth` — "Release-critical or cross-layer assurance: gauntlet-all or waiver."

**Severity**: MAJOR — FAIL-001/FAIL-002 behavioral verification missing.

### DEF-LOOM: Loom Tests Use `#[test]` Not `#[loom::test]`

**Finding**: All four loom tests in `queue_concurrency.rs` use `#[test]` annotation instead of `#[loom::test]`. The proof-writer report claims "Loom execution: `cargo loom --test <test_name>`" but this requires `#[loom::test]` to actually explore thread interleavings.

**Severity**: MAJOR — Loom lane is non-functional (not actually exercising concurrency models).

---

## TLA+ Scope Validity

**Rule**: `tla_temporal_default`

- `JournalBarrier` (ACK-ORDER-001): Variables, Init, Next, actions, invariants, temporal properties, and fairness all present. ✓
- `EventSeqOrdering` (POST-009, INV-004): Variables, Init, Next, invariants present. ✓
- `QueuedStrictFlush` (DISPATCH-002): Variables, Init, Next, invariants present, but QF1/QF2 have modeling gap (see DEF-TLA-QUEUE). ⚠

**TLA+ scope is valid** for the protocol layer but with noted modeling gaps.

---

## Verus Scope Validity

**Rule**: `verus_first`

**Critical issue**: All Verus artifacts are commented out. This is not a waiver — it is a non-functional artifact. The scope cannot be validated because the artifacts cannot execute.

Verus is the **correct** layer for:
- `verify_ack_after_persist` purity (POST-001/002)
- `DURABILITY_MATRIX` completeness (INV-001, PRE-001/002/003)
- `EventSeq` monotonicity (POST-009/010)
- `AckPoint` zombie variant (INV-005/006)
- `append_strict`/`append_journaled` postconditions (POST-006/007)

The layer fit is correct, but the artifacts are dead.

---

## Lean/Aeneas/Hax Scope Validity

**Rule**: `lean_scope`

`lean-contract.md` correctly states that Lean is **not required** for this bead because:
1. No algebraic theorem kernels
2. No parser/codec grammar requiring Lean
3. No arithmetic lattice beyond Verus expressiveness

**Lean scope is valid**. ✓

---

## Executable Obligation Schema

**Rule**: `executable_obligation_schema`

All 27 `proof-obligations.planned.jsonl` entries have required fields:
- `id` ✓
- `contract_clause` ✓
- `target` ✓
- `claim` ✓
- `layer` ✓
- `checker` ✓
- `command` ✓
- `evidence` ✓
- `expected_evidence` ✓
- `risk` ✓
- `scope` ✓
- `required` ✓
- `mode` ✓
- `owner_state` ✓
- `rerun_from` ✓
- `status` (all `planned`) ✓

**TLA+ entries** additionally have: `tla_module`, `model`, `config`, `variables`, `actions`, `invariants`, `temporal_properties`, `fairness`, `state_constraints`, `refinement`. ✓

**Executable obligation schema is valid**. ✓

---

## Waiver Quality

**Rule**: `waiver_quality`

- W1 (Fjall fsync internals): Names layer (rust-contract), reason (external crate), compensating evidence (Kani + integration). ✓
- W2 (codec roundtrips): Names layer (rust-contract), reason (serde not temporal), compensating evidence (proptest + Miri). ✓
- W3 (recovery/replay): Names layer (rust-contract), reason (not concurrent protocol), compensating evidence (Kani + integration). ✓
- W-LEAN-001/002/003: All name layer, reason, compensating evidence. ✓

**Waiver quality is sufficient**. ✓

---

## Coverage Decision

- **Contract clauses traced**: All 33 clauses in `contract.md` are present in `traceability-matrix.jsonl`. ✓
- **TLA+-owned clauses covered**: ACK-ORDER-001/002 (barrier semantics), DISPATCH-002 (queued flush). ⚠ modeling gaps noted.
- **Verus-owned clauses covered**: All 6 planned but dead (commented out). ✗
- **Theorem-owned clauses covered**: Lean correctly stated as not required. ✓
- **Proof obligations traced**: 27 obligations in `proof-obligations.planned.jsonl`. ✓
- **TLA+ scope valid**: Valid with modeling gaps (DEF-TLA-BARRIER, DEF-TLA-QUEUE). ⚠
- **Verus scope valid**: Correct layer, dead artifacts. ✗
- **Lean/Aeneas/Hax scope valid**: Correctly stated as not required. ✓
- **Waivers valid**: All waivers have owner, reason, compensating evidence. ✓

---

## Findings Summary

### Severity: LETHAL

| ID | Clause | Layer | Problem |
|----|--------|-------|---------|
| CV-001 | POST-001/002, INV-002 (ACK-ORDER-001/002) | Verus | All 6 Verus spec blocks commented out — lane produces zero evidence. Central contract cannot be approved. |
| CV-002 | RECOVERY-002 | Kani | KANI-HYDRATE-001 is `kani::assert(true)` placeholder — zero verification. |
| CV-003 | RECOVERY-003 | Kani | KANI-REPLAY-001 is `kani::assert(true)` placeholder — zero verification. |
| CV-004 | ACK-ORDER-001 | Integration | INTEGRATION-ACK-001/002/003/004 are stub tests that only construct error variants — no behavioral verification. |

### Severity: MAJOR

| ID | Clause | Layer | Problem |
|----|--------|-------|---------|
| CV-005 | ACK-ORDER-001 (T1/T2 liveness) | TLA+ | `IF TRUE` in `AppendStrict` — persist barrier always succeeds; failure path unreachable. Model may not verify T1 under all interleavings. |
| CV-006 | DISPATCH-002 (QF1/QF2) | TLA+ | `CompleteFlush` can fire without appending events; QF1 vacuously true. DISPATCH-002 not reliably verified. |
| CV-007 | DISPATCH-001 | Kani | KANI-DISPATCH-001/002 are vacuous (`kani::any()` + trivial assertion) — no dispatch behavior verified. |
| CV-008 | DISPATCH-002 | Loom | All 4 loom tests use `#[test]` not `#[loom::test]` — concurrency model not exercised. |
| CV-009 | PRE-004 (journal_events non-empty) | Proptest | Hardcoded variant list in KANI-CODEC-001 may diverge from actual `RecordKind` enum. |

---

## Verdict

**STATUS: REJECTED**

### Blocking Defects

1. **Verus lane is dead** — all `verus!` blocks commented out. This is not a waiver. The central ACK-ORDER-001/002 contract (strict persistence-before-acknowledgement) cannot be approved without Verus evidence for `verify_ack_after_persist`, `DURABILITY_MATRIX` completeness, `EventSeq` monotonicity, `AckPoint` zombie variant, and `append_strict`/`append_journaled` postconditions.

2. **Kani recovery harnesses are placeholders** — KANI-HYDRATE-001 and KANI-REPLAY-001 assert `true`. RECOVERY-002 and RECOVERY-003 are unverified.

3. **Integration tests are stubs** — FAIL-001/FAIL-002 behavioral verification is missing.

4. **TLA-BARRIER-001 has modeling gap** — `IF TRUE` makes persist always succeed. Failure path not exercised.

5. **TLA-QUEUE-001 has modeling gap** — `CompleteFlush` reachable without appending events. QF1 is vacuous.

6. **Kani dispatch harnesses are vacuous** — no actual dispatch behavior verified for DISPATCH-001.

7. **Loom tests are non-functional** — `#[test]` instead of `#[loom::test]`.

### Required Before Re-Approval

1. Uncomment all `verus!` blocks in all three Verus artifact files; fix `assert_seqs_equal()` call in `proof_matrix_completeness`.
2. Replace `kani::assert(true)` harnesses in KANI-HYDRATE-001 and KANI-REPLAY-001 with real verification harnesses.
3. Write behavioral integration tests for INTEGRATION-ACK-001/002/003/004.
4. Fix `IF TRUE` in TLA `AppendStrict` to cover both success and failure paths.
5. Fix `CompleteFlush` pre-condition in TLA `QueuedStrictFlush`.
6. Rewrite KANI-DISPATCH-001/002 to actually call `append_storage_event` and verify dispatch.
7. Re-annotate Loom tests with `#[loom::test]`.

---

*Contract verification reviewer — vb-core-strict-ack-ordering. State 6.*
