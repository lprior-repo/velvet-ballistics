# Proof Review Report — vb-core-strict-ack-ordering

**Bead ID**: vb-core-strict-ack-ordering
**Workspace**: /tmp/vb-ws/vb-core-strict-ack-ordering
**State**: 6 (Proof Review)
**Reviewer**: proof-reviewer

---

## Executive Summary

**STATUS: REJECTED — 6 critical defects, 5 moderate defects**

27 obligations planned. 7 have fatal artifacts that will not produce meaningful verification evidence. The Verus lane is completely non-functional (all specs commented out). Two TLA+ specs have material modeling gaps that make their invariant evidence unreliable. Kani dispatch harnesses are vacuous. Kani recovery harnesses are placeholder `assert(true)`. Integration tests are non-executing stubs.

---

## Fatal Defects (Blocking — Must Repair)

### DEF-001: VERUS — All specs commented out (VERUS-DM-001/002/003/004, VERUS-JA-001/002)

**Artifact**: `verus_artifacts/durability_matrix.verus`, `verus_artifacts/types_eventseq.verus`, `verus_artifacts/append_strict_journaled.verus`

**Problem**: All `verus! { ... }` blocks are fully commented out with `//`. Verus will not parse commented code. Every spec function and proof function is inert.

Evidence from `durability_matrix.verus` lines 6–88:
```rust
// verus! {
// use vb_storage::RecordKind;
// ...
// proof fn proof_verify_ack_after_persist()
//     ensures verify_ack_after_persist_spec() == Ok(())
// {
//     assert(forall |row| row in DURABILITY_MATRIX ==> ack_point_is_after_append(row));
// }
// } // verus!
```

Same pattern in `types_eventseq.verus` (lines 7–55) and `append_strict_journaled.verus` (lines 7–58).

**Impact**: 6 of 27 obligations produce zero verification. Critical Inv-002, POST-001/002/006/007, and type-level proofs are unverified.

**Required repair**: Uncomment all `verus!` blocks. Add `verus!` macro invocation to source files at artifact paths. Verify Verus can parse and run each module.

---

### DEF-002: TLA-BARRIER-001 — `IF TRUE` in AppendStrict makes persist always succeed (TLA-BARRIER-001)

**Artifact**: `specs/JournalBarrier.tla`, lines 51–63

**Problem**: The `AppendStrict` action unconditionally takes the success branch:
```tla
IF TRUE  \* in model: persist always succeeds for this obligation
   THEN /\ persistedEvents' = appendedEvents'
        /\ journaledEvents' = appendedEvents'
        /\ persistError' = FALSE
        /\ ackSent' = FALSE
```

The failure path (the `ELSE` branch) is unreachable. The `persistError` flag is never set as a consequence of `AppendStrict`. The only way `persistError = TRUE` in the model is via the independent `PersistError` action, which does not model the case where append succeeds but barrier fails.

**Contract requirement**: POST-006 and ACK-ORDER-001 require that `append_strict` returns `Ok(())` only after **both** `append_unpersisted` **and** `persist_strict` succeed. The model should exercise the failure path where append succeeds but persist fails, verifying `ackSent` stays FALSE.

**Impact**: Invariant I1 (`ackSent => persistedEvents = journaledEvents`) and I2 (`persistError => ~ackSent`) are verified only for the success + separate-PersistError path. The combined success-of-append + failure-of-persist path is not exercised. The liveness proof T2 (`<> ackSent`) is unreliable if the barrier failure path is never reachable in the model.

**Required repair**: Replace `IF TRUE` with a model variable `persistSucceeds: BOOLEAN` and drive it via `AppendStrict`/`PersistError` to cover both success and failure interleavings.

---

### DEF-003: TLA-QUEUE-001 — `CompleteFlush` reachable without appending events (TLA-QUEUE-001)

**Artifact**: `specs/QueuedStrictFlush.tla`, lines 87–97

**Problem**: The `CompleteFlush` action has no pre-condition requiring all queued strict events to be appended:
```tla
CompleteFlush ==
  /\ flushInProgress
  /\ persistBarrierCalled = 1  (* barrier was called *)
  /\ strictFlushComplete' = TRUE
  /\ flushInProgress' = FALSE
  /\ queue' = {}  (* clear queue after successful flush *)
  /\ persistBarrierCalled' = 0
  /\ UNCHANGED appendedEvents
```

`CompleteFlush` can fire when `appendedEvents` is a subset of (or disjoint from) the queued strict events. The invariants QF1 and QF2 check `strictFlushComplete` in the **resulting** state, not the **transition** condition:

```tla
QF1 == strictFlushComplete =>
  \A e \in Nat :
    [profile |-> "Strict", event |-> e] \in queue
    => e \in appendedEvents
```

When `CompleteFlush` fires, `queue' = {}`, so `\A e \in Nat : [profile |-> "Strict", event |-> e] \in queue` is **vacuously true** (quantifies over empty set). QF1 passes even though no event was ever appended.

**Impact**: QF1 and QF2 do not prove DISPATCH-002 flush ordering. The model accepts a non-conforming implementation where `flush_batch` clears the queue and returns success without appending events.

**Required repair**: Add a `QueueAllStrictEventsAppended` action that appends remaining events before `CompleteFlush` can set `strictFlushComplete' = TRUE`, or move the pre-condition into `CompleteFlush` itself.

---

### DEF-004: KANI-DISPATCH-001/002 — Vacuous harnesses (KANI-DISPATCH-001, KANI-DISPATCH-002)

**Artifacts**: `kani_harnesses/verify_strict_profile_dispatches_to_append_strict.rs`, `kani_harnesses/verify_journaled_profile_dispatches_to_append_journaled.rs`

**Problem**: Both harnesses are circular/vacuous:

```rust
let profile: DurabilityProfile = kani::any();
let is_strict = matches!(profile, DurabilityProfile::Strict);
if is_strict {
    kani::assert(
        matches!(profile, DurabilityProfile::Strict),  // trivially true
        "Strict profile confirmed — must call append_strict",
    );
}
```

These verify nothing about dispatch behavior. `kani::any()` picks a value, `matches!` checks it, then `kani::assert` re-checks the same condition. The actual dispatch logic in `chunk_002.rs` (the `if self.profile == Strict { ... } else { ... }`) is never exercised or verified.

**Contract clause**: DISPATCH-001 requires `append_storage_event` dispatches to `append_strict` for Strict and `append_journaled` for Journaled. The harnesses do not verify this.

**Required repair**: Rewrite harnesses to actually call `append_storage_event` on a constructed `StorageRuntimeJournal` with each profile variant and verify the correct method is invoked (using stub/mock FjallJournal).

---

### DEF-005: KANI-HYDRATE-001 — Placeholder harness asserting `true` (KANI-HYDRATE-001)

**Artifact**: `kani_harnesses/verify_hydrate_run_frame_digest_matches.rs`, line 36

```rust
kani::assert(true, "KANI-HYDRATE-001: digest match verified by harness");
```

**Problem**: This produces zero verification evidence. RECOVERY-002 (digest must match on recovery) is not verified. The harness is indistinguishable from a trivially-correct proof.

**Required repair**: Write a real Kani harness that creates a journal with a known sequence of events, calls `recover_runtime_frame_seed_from_events`, and asserts the returned digest equals the expected value.

---

### DEF-006: KANI-REPLAY-001 — Placeholder harness asserting `true` (KANI-REPLAY-001)

**Artifact**: `kani_harnesses/verify_replay_divergence_detected.rs`, line 28

```rust
kani::assert(true, "KANI-REPLAY-001: ReplayDivergence detection verified by harness");
```

**Problem**: Same as DEF-005. RECOVERY-003 (replay divergence detection) is not verified.

**Required repair**: Write a real Kani harness that constructs an out-of-order event sequence and asserts `replay_events` returns `Err(ReplayDivergence)`.

---

## Moderate Defects (Must Address Before Approval)

### DEF-007: Integration tests are stubs — no behavioral verification (INTEGRATION-ACK-001/002/003/004)

**Artifacts**: `integration_tests/submit_direct_durability_test.rs`, `recovery_digest_match_test.rs`, `action_completion_ack_test.rs`, `ask_completion_ack_test.rs`

**Problem**: Each integration test is a placeholder that only constructs error variants:
```rust
let _err = RuntimeError::AdmissionHeaderPersistenceFailed { cause: Box::new(JournalError::QueueCapacity) };
```

No test actually calls the runtime, injects a failure, and verifies the ack is not sent. These will compile but never fail and never verify FAIL-001/ACK-ORDER-001.

**Required repair**: Write real behavioral tests using mock FjallJournal fixtures that inject persist failures and verify typed error propagation and no ack sent.

---

### DEF-008: Loom tests use `#[test]` not `#[loom::test]` — not actual concurrency models (LOOM-QUEUE-001/002/003/004)

**Artifact**: `loom_models/queue_concurrency.rs`

**Problem**: All four tests are annotated with `#[test]`, not `#[loom::test]`. Loom is a permutation testing framework; `#[test]` runs the code once as a regular Rust test without exploring interleavings.

The proof-writer report says "Loom execution: `cargo loom --test <test_name>`" but `cargo loom --test` requires `#[loom::test]` attribute. With `#[test]`, `cargo loom` will either ignore these or fail.

**Evidence**: `flush_batch_strict_ordering` uses `thread::spawn` (real threads), not `loom::thread::spawn`. This bypasses loom's schedule exploration entirely.

**Required repair**: Re-annotate all four tests with `#[loom::test]` and use `loom::thread::spawn` for concurrency.

---

### DEF-009: Proptest `event_seq_monotonic` uses `==>` which can be vacuous (PROPTEST-EVENTSEQ-001)

**Artifact**: `proptest_cases/event_seq_ordering.rs`, lines 40–53

```rust
prop_assert!(v1 < v2 ==> {
    // ...
});
```

When `!(v1 < v2)` (i.e., `v1 >= v2`), the `==>` short-circuits to `Ok(())`. The proptest framework will not count these as failures. However, proptest's `prop_assert!` with `==>` generates cases where the antecedent holds, so some coverage exists. The property is not maximally discriminating.

**Impact**: Moderate — the property still exercises the monotonicity check for many pairs, but the `==>` pattern is non-idiomatic and less rigorous than filtering.

**Required repair**: Use `proptest::prop_assume!(v1 < v2)` to filter before the assertion, then assert monotonicity unconditionally.

---

### DEF-010: Verus `proof_matrix_completeness` calls non-existent `assert_seqs_equal()` (VERUS-DM-002)

**Artifact**: `verus_artifacts/durability_matrix.verus`, line 65

```rust
proof fn proof_matrix_completeness()
    ensures verify_matrix_completeness_spec() == Ok(())
{
    assert(matrix_primitives_set() == required_primitives_set()) by {
        assert_seqs_equal();  // <-- this function does not exist
    }
    assert(verify_matrix_completeness_spec() == Ok(()));
}
```

`assert_seqs_equal()` is not defined in the Verus standard library and would cause a Verus error. This is unreachable code (whole file is commented), but if uncommented, it would fail to compile.

**Required repair**: Remove `assert_seqs_equal()` and use direct structural assertions, e.g., `assert(matrix_primitives_set() == required_primitives_set()) by (/* direct set equality proof */)`.

---

### DEF-011: INV-004 serde roundtrip modeled as identity not tested (TLA-EVENTSEQ-001)

**Artifact**: `specs/EventSeqOrdering.tla`, line 68

```tla
EO3 == TRUE  \* Contiguity enforced by AppendEvent: must append in order
```

EO3 is set to TRUE with a comment. The contract INV-004 requires "serde roundtrip preserves value." The TLA+ model models this as identity but does not actually test a serialize→deserialize sequence. This is acceptable as a modeling simplification but should be documented as such and covered by the Miri/proptest lanes.

**Status**: Covered by MIRI-CODEC-001 and PROPTEST-EVENTSEQ-001 if those lanes work correctly (DEF-001, DEF-012 are concerns for those).

---

### DEF-012: KANI-CODEC-001 hardcoded variant list may diverge from actual RecordKind enum

**Artifact**: `kani_harnesses/verify_record_kind_codec.rs`, lines 18–41

```rust
let variants: [RecordKind; 21] = [
    RecordKind::WorkflowSource,   // 1
    RecordKind::CompiledIr,       // 2
    // ... 19 more hardcoded variants
];
```

If the `RecordKind` enum in `vb_storage/src/records.rs` has different variants or different discriminants, the harness will fail to compile or miss coverage. The harness hardcodes 21 variants but the contract (INV-003) says "all RecordKind values encode and decode correctly" — this should enumerate variants via a const generic or derive, not hand-maintain a list.

**Required repair**: Derive `VariantNames` or use a const array from the enum to auto-discover variants rather than hardcoding.

---

## Artifact Adequacy Assessment

| Lane | Obligation | Artifact | Status | Blocking? |
|------|-----------|----------|--------|-----------|
| TLA+ | TLA-BARRIER-001 | `JournalBarrier.tla` | **DEF-002** | YES |
| TLA+ | TLA-EVENTSEQ-001 | `EventSeqOrdering.tla` | Adequate (INV-004 gap noted) | No |
| TLA+ | TLA-QUEUE-001 | `QueuedStrictFlush.tla` | **DEF-003** | YES |
| Verus | VERUS-DM-001 | `durability_matrix.verus` | **DEF-001**, **DEF-010** | YES |
| Verus | VERUS-DM-002 | `durability_matrix.verus` | **DEF-001**, **DEF-010** | YES |
| Verus | VERUS-DM-003 | `types_eventseq.verus` | **DEF-001** | YES |
| Verus | VERUS-DM-004 | `durability_matrix.verus` | **DEF-001** | YES |
| Verus | VERUS-JA-001 | `append_strict_journaled.verus` | **DEF-001** | YES |
| Verus | VERUS-JA-002 | `append_strict_journaled.verus` | **DEF-001** | YES |
| Kani | KANI-ACK-001 | `verify_no_before_journal_append_in_matrix.rs` | Adequate | No |
| Kani | KANI-DISPATCH-001 | `verify_strict_profile_dispatches_to_append_strict.rs` | **DEF-004** | YES |
| Kani | KANI-DISPATCH-002 | `verify_journaled_profile_dispatches_to_append_journaled.rs` | **DEF-004** | YES |
| Kani | KANI-CODEC-001 | `verify_record_kind_codec.rs` | **DEF-012** | Moderate |
| Kani | KANI-HYDRATE-001 | `verify_hydrate_run_frame_digest_matches.rs` | **DEF-005** | YES |
| Kani | KANI-REPLAY-001 | `verify_replay_divergence_detected.rs` | **DEF-006** | YES |
| Loom | LOOM-QUEUE-001 | `queue_concurrency.rs` | **DEF-008** | YES |
| Loom | LOOM-QUEUE-002 | `queue_concurrency.rs` | **DEF-008** | YES |
| Loom | LOOM-QUEUE-003 | `queue_concurrency.rs` | **DEF-008** | YES |
| Loom | LOOM-QUEUE-004 | `queue_concurrency.rs` | **DEF-008** | YES |
| Miri | MIRI-CODEC-001 | `record_kind_roundtrip.rs` | Adequate (DEF-012 concern) | Moderate |
| Proptest | PROPTEST-EVENTSEQ-001 | `event_seq_ordering.rs` | **DEF-009** | Moderate |
| Integration | INTEGRATION-ACK-001 | `submit_direct_durability_test.rs` | **DEF-007** | YES |
| Integration | INTEGRATION-ACK-002 | `recovery_digest_match_test.rs` | **DEF-007** | YES |
| Integration | INTEGRATION-ACK-003 | `action_completion_ack_test.rs` | **DEF-007** | YES |
| Integration | INTEGRATION-ACK-004 | `ask_completion_ack_test.rs` | **DEF-007** | YES |
| Static | STATIC-SCAN-001 | (clippy command) | Adequate | No |
| Static | STATIC-SCAN-002 | (clippy command) | Adequate | No |

---

## Open Questions (OQ) Contract Coverage

| OQ | Question | Coverage Assessment |
|----|----------|---------------------|
| OQ-1 | Does `flush_batch` guarantee same barrier as `append_strict`? | **INADEQUATE** — TLA-QUEUE-001 has DEF-003; Loom tests have DEF-008. Neither provides reliable evidence. |
| OQ-2 | Is `BeforeJournalAppend` reachable through public API? | **ADEQUATE** — KANI-ACK-001 harness is structurally sound (noting DEF-012 about hardcoded list). |
| OQ-3 | Are all 11 `test_evidence` paths real tests? | **INADEQUATE** — Integration tests are stubs (DEF-007). Cannot confirm. |

---

## Verdict

**REJECTED — 6 fatal defects, 5 moderate defects**

The proof artifact set cannot produce reliable verification evidence for the central claim (ACK-ORDER-001: strict persistence-before-acknowledgement). The Verus lane is completely dead (all specs commented). Two TLA+ invariants are unreliable due to modeling gaps. The Kani recovery harnesses are placeholders. The integration tests are stubs.

The contract `ACK-ORDER-001/002` is not adequately proven by these artifacts.

---

*Proof reviewer for vb-core-strict-ack-ordering. State 6.*
