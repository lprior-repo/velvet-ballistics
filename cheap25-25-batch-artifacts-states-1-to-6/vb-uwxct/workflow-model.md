# Workflow Model — vb-uwxct

This bead is TEST-ONLY. There is no production workflow to model. The
"workflow" this bead models is the **specimen-repair lifecycle**:

```
              ┌──────────────────────────────┐
              │  Specimen drafted (current)  │
              │   full u64 + .expect() /     │
              │   Err(_) => assert!(false)   │
              └─────────────────┬────────────┘
                                │
                                ▼
                ┌────────────────────────────────┐
                │ Choose repair form              │
                │ (canonical: 0u64..u64::MAX;     │
                │  sentinel: kani::assume;        │
                │  heterogeneous: match arm)      │
                └─────────────────┬───────────────┘
                                  │
                                  ▼
                ┌────────────────────────────────┐
                │ Tighten the specimen            │
                │ (Holzman Rust implementation)   │
                └─────────────────┬───────────────┘
                                  │
                                  ▼
                ┌────────────────────────────────┐
                │ Behavior test pass              │
                │ cargo test -p workspace_tests   │
                │ --test restate_journal_tail_…   │
                │ (no panics; properties hold on  │
                │  contract-encodable range)      │
                └─────────────────┬───────────────┘
                                  │
                                  ▼
                ┌────────────────────────────────┐
                │ Kani re-prove (if harness      │
                │ touched) — accept the typed    │
                │ SequenceOverflow rejection     │
                │ as contract-conformant          │
                └─────────────────┬───────────────┘
                                  │
                                  ▼
                ┌────────────────────────────────┐
                │ Closure evidence:              │
                │ - targeted cargo test green    │
                │ - (if Kani touched)            │
                │   bash scripts/kani-list.sh    │
                │   vb_storage plus harness-only │
                │   probe                        │
                │ - black-hat-reviewer accepts   │
                └────────────────────────────────┘
```

The remainder of this document names the **legal states**, **transitions**,
**guards**, and **outcomes** of this workflow model.

## Legal States (Specimen Lifecycle)

| State | Definition | Acceptable? |
|---|---|---|
| `SpecimenDrafted` | Original shape: full u64 range + `.expect()` / `Err(_) => assert!(false)` | YES (pre-bead) |
| `SpecimenTightened` | Repair form applied; one of the four acceptable forms from `type-contracts.md` §6 | YES (post-bead) |
| `SpecimenContractHold` | All `seq ∈ 0..u64::MAX` inputs continue to satisfy the property | YES (post-bead) |
| `SpecimenSentinelHonored` | `seq == u64::MAX` input either skips (`assume!`) or `match`-treats as `Err(SequenceOverflow)` | YES (post-bead) |
| `SpecimenRunning` | Specimen compiles and runs without panic for all sampled inputs | YES (post-bead) |
| `SpecimenClosed` | Targeted `cargo test` green; (if Kani touched) Kani harness probes green | YES (closure) |

## Forbidden Specimen States

| State | Why forbidden |
|---|---|
| `SpecimenSilentlyAccepted` | Specimen accepts an input that should be rejected (e.g., `Err(_) => prop_assert!(true)` without a sentinel check) |
| `SpecimenStrictlyRejected` | Specimen rejects an input that should succeed (e.g., wrong `prop_assume!` makes `seq ∈ valid_range` impossible) |
| `SpecimenPropertyWeakened` | Specimen's property under test has changed shape (e.g., removed an assertion to dodge a sentinel) |
| `SpecimenVariantRenamed` | Specimen re-binds the rejection to a different `JournalError` variant |

## Transitions and Guards

### T1 — `SpecimenDrafted → SpecimenTightened`

- **Command**: implementation agent edits the seven specimen source spans.
- **Guard**: at least one of the four acceptable forms chosen per specimen.
- **Precondition**: production code unchanged.
- **Postcondition**: each specimen source span has the documented shape.

### T2 — `SpecimenTightened → SpecimenContractHold`

- **Command**: targeted `cargo test -p workspace_tests --test restate_journal_tail_scan_fallback_tests -- --nocapture`.
- **Guard**: zero proptest panics, zero `.expect()` failures on `u64::MAX`
  inputs, properties asserted on `seq ∈ 0..u64::MAX` continue to hold.
- **Postcondition**: local green.

### T3 — `SpecimenTightened → SpecimenSentinelHonored` (Kani only)

- **Command**: harness updated to `match`-bind the typed error OR
  `kani::assume(seq_value != u64::MAX)` added.
- **Guard**: harness closure considers `seq_value == u64::MAX` an explicitly
  accepted sentinel rejection (not a vacuous counterexample).
- **Postcondition**: `bash scripts/kani-list.sh vb_storage` plus harness-only
  probe complete.

### T4 — `SpecimenContractHold + SpecimenSentinelHonored → SpecimenRunning`

- **Command**: full proptest runs (default 256 cases) succeed without panic;
  (Kani) `cargo kani -p vb_storage --harness vb_eepg_typed_partitioned_ids`
  returns PASS.
- **Postcondition**: no `cargo kani` counterexamples surface for
  `seq_value == u64::MAX`.

### T5 — `SpecimenRunning → SpecimenClosed`

- **Command**: black-hat review; evidence packaging.
- **Postcondition**: bead vb-uwxct closed.

## Commands (legal transitions only)

| # | Legal transition | Lane | Required |
|---|---|---|---|
| C1 | T1 (any specimen) | holzman-rust-implementation | yes |
| C2 | T2 (cargo test, no overflow panic) | behavior-test-acceptance | yes |
| C3 | T3 (Kani harness) | proof-writer-kani | only if harness byte-rewrite path chosen; alternative `kani::assume(seq_value != u64::MAX)` path requires no harness body rewrite past the assumption |
| C4 | T4 (Kani run + targeted cargo test full pass) | proof-verifier | yes if T3 fired |
| C5 | T5 (black-hat) | black-hat-reviewer | yes (acceptance) |

## Idempotence

A re-run of any of C2/C4 on already-closed specimens must continue to pass
without re-tightening. The repair is structurally idempotent because the
canonical pattern `s in 0u64..u64::MAX` and `kani::assume(seq != u64::MAX)`
produce stable input spaces.

## Cancellation / Drain

If the implementation agent must abandon the repair mid-stream, the
**forbidden state** to avoid is "SpecimenHalf-Tightened": some proptests
repaired and others left in the original over-rejecting shape. The repair
must be all-or-nothing — either all six proptests and the harness updated,
or none updated and the bead reverted.

## Terminal Outcomes

- **Pass (preferred)**: all five transitions complete; closure evidence in
  `.beads/vb-uwxct/evidence/`.
- **Blocked tooling**: `cargo kani` or `cargo test` environment not present
  on the worker's machine. The agent must report the block; this bead does
  not advance past T3 in that case.
- **Rejection**: a downstream reviewer (proof-reviewer or black-hat) finds
  a flaw in the choice of repair form (e.g., Kani `kani::assume(seq_value != u64::MAX)`
  chosen but the harness body still has `Err(_) => assert!(false)`); the
  bead loops back to T1 with a reviewer finding.

## Out of Workflow Scope

- Production behavior of `run_event_key` — NOT in workflow.
- `JournalError::SequenceOverflow` semantics — NOT in workflow (already correct).
- Snapshot key (`run_snapshot_key`) specimen at lines 1195-1215
  (`sequence_overflow_must_be_distinct_from_sequence_gap`) — referenced but
  not modified.
- Proptests in `fjall_keyspace_manifest_tests.rs` — referenced as canonical
  pattern but not modified.
