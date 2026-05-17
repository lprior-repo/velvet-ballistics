# Test Plan Review — LETHAL-8 (Mode 1: Plan Inquisition)

## VERDICT: REJECTED

---

## Executive Summary

The plan identifies the correct behaviors and has good structural coverage, but has **2 LETHAL findings** and **6 MAJOR findings** that must be resolved before approval. The Kani harness claim is mathematically false, and critical boundary cases and error variants are missing coverage.

---

## Axis 1 — Contract Parity

**Functions identified in plan:**
- `drive_deterministic_full` (pub fn, drive.rs:47) — covered by B-1 scenarios ✓
- `replay_events` (pub fn, replay/core.rs:34) — covered by B-2 scenarios ✓
- `recover_full_journal` (pub fn, replay/core.rs:109) — covered by B-2 scenarios ✓
- `recover_snapshot_plus_tail` (pub fn, replay/core.rs:139) — covered by B-3 scenarios ✓
- `mark_step_after_signal` (pub fn, helpers.rs:12) — referenced in harness description, not in scenarios ✗
- `emit_slot_evidence` (private fn, drive.rs:130) — implicitly tested via drive_deterministic_full ✓

**Error variants in `RecoveryError`:**
- `RecoveryError::ReplayDivergence` — tested exactly in `replay_detects_slot_written_after_step_start_violation` ✓
- `RecoveryError::NoRecoveryData` — NOT explicitly tested in this plan's scenarios (only via existing tests in recovery_bdd_tests.rs)
- `RecoveryError::CorruptSnapshot` — NOT explicitly tested in this plan's scenarios (only via existing tests)
- `RecoveryError::NonIdempotentActionBlocked` — NOT covered
- `RecoveryError::Journal` — NOT covered
- `RecoveryError::FrameDimensionOverflow` — NOT covered
- `RecoveryError::CompiledIrDigestMismatch` — NOT covered

**LETHAL-1: Missing Error Variant Coverage**
- `recover_full_journal` has 5+ error return paths but plan only tests `ReplayDivergence`
- `NoRecoveryData`, `CorruptSnapshot`, `Journal` error variants have no explicit scenario asserting exact variant
- This violates Axis 1: "Every Error variant must have a scenario asserting the exact variant"

**LETHAL-2: Kani Harness Claim is Mathematically False**
- Plan states (line 174): "Kani can formally verify the call order: `mark_step_after_signal` must be called (and return) before `emit_slot_evidence` pushes the slot write event"
- The harness skeleton (lines 177-188) does NOT verify this ordering
- The harness calls `drive_deterministic_full` for one step and asserts evidence contains `SlotWritten` and PC advanced — it does NOT assert ordering between `mark_step_after_signal` and `emit_slot_evidence`
- Kani cannot verify Rust runtime call ordering without embedding formal annotations in the source or modeling the function internals
- The harness proves only that evidence is emitted after one step completes — NOT that `mark_step_after_signal` returns before `emit_slot_evidence`
- This is a false proof claim: the harness does not prove what the plan says it proves

**MAJOR-1: `mark_step_after_signal` Has No Direct Scenario**
- Only referenced in the Kani harness description, not in any BDD scenario
- B-1 behavior says "SlotWritten BEFORE PC advance" but `mark_step_after_signal` is the PC-advance operation
- No scenario directly tests the relationship between `mark_step_after_signal` return and evidence emission

---

## Axis 2 — Assertion Sharpness

**Scenario-by-scenario Then clauses:**

| Scenario | Assertion | Verdict |
|----------|-----------|---------|
| `slot_written_appears_before_next_step_started_in_evidence_stream` | "index of SlotWritten(0) < index of StepStarted(1)" | EXACT ✓ |
| `evidence_collector_emits_slot_before_mark_step_after_signal_returns` | "EvidenceCollector drain contains SlotWritten(slot=0) event" | WEAK — no ordering asserted |
| `multi_slot_node_emit_order_preserved` | "All SlotWritten events appear before any evidence from next step" | EXACT ✓ |
| `no_slot_written_node_omits_slot_event` | "no SlotWritten" | EXACT ✓ |
| `replay_restores_slot_values_in_correct_sequence_order` | "slot 0 = value_from_SlotWrittenEvent_0, slot 1 = value_from_SlotWrittenEvent_1" | EXACT ✓ |
| `snapshot_plus_tail_replays_tail_slot_writes_after_snapshot` | "slot value from SlotWrittenEvent(seq=S+1) is present" | EXACT ✓ |
| `replay_detects_slot_written_after_step_start_violation` | "RecoveryError::ReplayDivergence with detail" | EXACT ✓ |
| `replay_preserves_slot_value_on_recovery` | "slot 42 contains I64(99)" | EXACT ✓ |
| `snapshot_captures_all_preceding_slot_writes` | "values for slot 0, slot 1, slot 2" | EXACT ✓ |
| `tail_events_after_snapshot_preserve_order` | "Each tail event's seq > snapshot seq" | EXACT ✓ |
| `corrupt_snapshot_seq_fails_gracefully` | "RecoveryError::ReplayDivergence" | EXACT ✓ |

**MAJOR-2: `evidence_collector_emits_slot_before_mark_step_after_signal_returns` — Missing Ordering Assertion**
- The scenario name implies an ordering relationship
- The Then clause only asserts: "EvidenceCollector drain contains SlotWritten(slot=0) event"
- This proves evidence is emitted eventually, not that it happens before `mark_step_after_signal` returns
- The behavior B-1 says "SlotWritten events BEFORE the program counter advances" — but this scenario doesn't assert the ordering
- A test passing this scenario would NOT catch if `emit_slot_evidence` were called BEFORE `mark_step_after_signal`

---

## Axis 3 — Trophy Allocation

**Test counts stated:**
- 2 integration / 1 unit / 0 e2e / 0 static
- 1 proptest invariant
- 1 Kani harness (but see LETHAL-2)

**Function count for trophy ratio:**
- `drive_deterministic_full` — 1 pub fn
- `replay_events` — 1 pub fn
- `recover_full_journal` — 1 pub fn
- `recover_snapshot_plus_tail` — 1 pub fn
- Plus private helpers implicitly tested

**Ratio: 3 tests / 4 pub fn = 0.75x — FAILS target of 5x**

However, this is a focused behavioral gap plan, not a full coverage plan. The plan correctly identifies 11 scenarios across 3 behaviors. The ratio concern is mitigated by the targeted nature.

**MAJOR-3: `recover_full_journal` Missing Unit Test Coverage**
- Only has integration test via `replay_restores_slot_values_in_correct_sequence_order`
- No unit test for `NoRecoveryData` (empty journal) variant
- No unit test for `Journal` error wrapping
- This is a pure function with multiple error paths requiring explicit variant testing

**MAJOR-4: `replay_events` Proptest Invariant Missing**
- `journal_seq_ordering_invariant` covers `replay_events` (line 155-159)
- Strategy: "Generate random event sequences with monotonically increasing seq values"
- This tests the happy path ordering, NOT the ANTI-invariance (decreasing/duplicate seq)
- The invariant states `E1.seq() <= E2.seq()` but the anti-invariant description only mentions "decreasing or duplicate seq values" — the proptest should generate violating cases to ensure the function rejects them
- The invariant as stated is a TOO-WEAK characterization — it only covers monotonic seq, not the full rejection property

---

## Axis 4 — Boundary Completeness

For `recover_full_journal`:
- Min (empty journal): implicitly via `NoRecoveryData` but NOT explicitly tested
- Max (huge journal): NOT specified
- One-above-max (journal with corrupted size): NOT specified
- Empty / zero: NOT explicitly tested

For `replay_events`:
- Min (empty slice): NOT explicitly tested
- Max (huge slice with 100K events): NOT specified
- Empty seq (all seq=0): NOT specified
- Overflow (seq wraps at max): NOT specified

For `recover_snapshot_plus_tail`:
- Snapshot seq boundary (snapshot seq = tail seq exactly, not greater): NOT explicitly tested
- Snapshot seq < tail seq boundary: tested via `corrupt_snapshot_seq_fails_gracefully` ✓

For `drive_deterministic_full`:
- 0 steps (empty workflow): NOT specified
- 1 step: tested ✓
- Many steps (10+): tested via proptest strategy ✓
- Suspending step (Ask without answer): tested ✓

**MINOR-1: Missing Empty Journal Boundary for `recover_full_journal`**
- `replay_events` with empty input is unverified
- Existing tests use non-empty journals

**MINOR-2: Missing Seq Overflow Boundary**
- No test with `SeqNo::MAX` or wrapping arithmetic
- This is a numeric boundary, not a behavioral one — lower priority

---

## Axis 5 — Mutation Survivability

**Mutations identified in plan:**
| Mutation | Location | Test | Adequacy |
|----------|----------|------|----------|
| Swap `mark_step_after_signal` and `emit_slot_evidence` | `finish_drive_step` | `slot_written_appears_before_next_step_started_in_evidence_stream` | Partial — scenario asserts ordering but `evidence_collector_emits_slot_before_mark_step_after_signal_returns` does NOT |
| Remove `emit_slot_evidence` | `finish_drive_step` | `no_slot_written_event_emitted_when_step_has_output` | Not in plan — only `no_slot_written_node_omits_slot_event` exists |
| Change SlotWrittenEvent seq after PC advance | journal emission | `replay_restores_slot_values_in_correct_sequence_order` | Indirect — doesn't directly assert seq ordering |
| Reorder `replay_events` StepStarted before SlotWritten | `replay/core.rs` | `replay_detects_slot_written_after_step_start_violation` | ✓ |

**MAJOR-5: Missing `emit_slot_evidence` Removal Mutation**
- Plan lists "Remove `emit_slot_evidence` call entirely" as a critical mutation
- But no scenario tests the case where a step that SHOULD write a slot produces NO SlotWritten event
- `no_slot_written_node_omits_slot_event` tests Nop (no output) — not a SetConst that should emit but doesn't

**MAJOR-6: `replay_events` Swap Mutation Not Directly Tested**
- The plan says: "Reorder `replay_events` to process `StepStarted` before `SlotWrittenEvent` for same step"
- `replay_detects_slot_written_after_step_start_violation` tests StepStarted(1) before SlotWrittenEvent(0) — which is a cross-step violation
- Does NOT test StepStarted(N) before SlotWrittenEvent(N) for the SAME step
- Missing: SlotWrittenEvent(N) arrives after StepStarted(N) for same step — should be rejected

---

## Axis 6 — Evidence Plan Audit

**Given/When/Then structure:** All scenarios have explicit Given/When/Then ✓

**Preconditions stated explicitly:** Partial
- Most scenarios state the input setup explicitly
- `evidence_collector_emits_slot_before_mark_step_after_signal_returns`: Given is explicit, When is explicit, Then is WEAK (see MAJOR-2)

**Generated coverage bounding:**
- Proptest strategy (1-10 consecutive SetConst nodes) is bounded ✓
- But "1-10" is arbitrary — no justification for this range

**Test file locations specified:**
- Integration: `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` — new file, not yet created
- Unit: `crates/vb_storage/src/recovery/replay/ordering_tests.rs` — marked as new file
- Kani: `crates/vb_runtime/src/kani_slot_written_ordering.rs` — marked as new file

**MAJOR-7: Integration Test References Non-Existent File**
- The file `slot_written_ordering_integration_tests.rs` does not exist
- Plan says tests will be written there, but no existing tests are referenced
- This is expected for Mode 1 (pre-implementation) but worth noting

**Combinatorial matrix has orphaned entries:**
- `replay_skips_older_attempt_slot_writes` — referenced in matrix but not in BDD scenarios
- `snapshot_plus_tail_with_empty_tail_succeeds` — referenced in matrix but not in BDD scenarios
- These appear to be planned tests that were never described in the BDD section

---

## Summary of Findings

### LETHAL FINDINGS (must fix before resubmission)

1. **[LETHAL-1] Missing Error Variant Coverage for `recover_full_journal`**
   - `NoRecoveryData`, `CorruptSnapshot`, `Journal` error variants have no scenario asserting exact variant
   - Violates Axis 1: "Every Error variant must have a scenario asserting the exact variant"

2. **[LETHAL-2] Kani Harness Claim is Mathematically False**
   - Plan states Kani "can formally verify the call order: `mark_step_after_signal` must be called (and return) before `emit_slot_evidence`"
   - The harness skeleton does NOT verify this ordering — it only asserts evidence contains SlotWritten
   - Kani cannot verify runtime call ordering without formal annotations in source
   - The harness proves presence of evidence after step completion, NOT ordering between the two specific functions

### MAJOR FINDINGS (≥3 = automatic rejection)

1. **[MAJOR-1]** `mark_step_after_signal` has no direct BDD scenario — only mentioned in harness description
2. **[MAJOR-2]** `evidence_collector_emits_slot_before_mark_step_after_signal_returns` has no ordering assertion — only presence assertion
3. **[MAJOR-3]** `recover_full_journal` missing unit test coverage for `NoRecoveryData` and `Journal` error paths
4. **[MAJOR-4]** `replay_events` proptest invariant only covers monotonic seq, not anti-invariant (decreasing/duplicate) rejection
5. **[MAJOR-5]** Missing mutation test for "Remove `emit_slot_evidence` entirely" — no scenario covers a step that SHOULD write but DOESN'T
6. **[MAJOR-6]** `replay_events` swap mutation not directly tested — `replay_detects_slot_written_after_step_start_violation` tests cross-step only, not same-step ordering violation
7. **[MAJOR-7]** Combinatorial matrix has orphaned entries (`replay_skips_older_attempt_slot_writes`, `snapshot_plus_tail_with_empty_tail_succeeds`) not defined in BDD section

### MINOR FINDINGS

1. **[MINOR-1]** `replay_events` with empty input slice not explicitly tested
2. **[MINOR-2]** SeqNo overflow boundary not tested
3. **[MINOR-3]** Proptest strategy "1-10" has no justification for range bounds

---

## MANDATE

Before resubmission for Mode 1 re-review:

1. **Fix LETHAL-1**: Add explicit scenarios for `NoRecoveryData`, `CorruptSnapshot`, and `Journal` error variants from `recover_full_journal`

2. **Fix LETHAL-2**: Either:
   - Remove the false claim that Kani verifies call ordering, OR
   - Rewrite the harness description to accurately state what Kani verifies (evidence presence after step completion), AND add source-level formal annotations that enable Kani to verify the ordering claim

3. **Fix MAJOR-1**: Add direct scenario for `mark_step_after_signal` relationship to evidence emission

4. **Fix MAJOR-2**: Add explicit ordering assertion to `evidence_collector_emits_slot_before_mark_step_after_signal_returns`

5. **Fix MAJOR-3**: Add unit test scenarios for `recover_full_journal` error paths

6. **Fix MAJOR-4**: Enhance `journal_seq_ordering_invariant` proptest to generate ANTI-invariant cases (decreasing seq, duplicate seq) and verify `replay_events` returns error

7. **Fix MAJOR-5**: Add scenario for "emit_slot_evidence removed" mutation — step that should write slot but produces no SlotWritten

8. **Fix MAJOR-6**: Add same-step ordering violation test for `replay_events` — SlotWrittenEvent(N) arriving after StepStarted(N) for same step

9. **Fix MAJOR-7**: Either define the orphaned matrix entries in BDD section, or remove from matrix

**Total: 2 LETHAL + 7 MAJOR = REJECTED**
