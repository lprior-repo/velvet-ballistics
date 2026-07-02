# Contract — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**owner_stage:** rust-contract
**upstream_artifacts:**
`domain-model.md`, `type-contracts.md`, `workflow-model.md`,
`error-taxonomy.md`, `boundary-map.md`, `hazard-analysis.md`

This contract is the normative binding between the domain model and the
implementable proof, test, and code stages that follow. Every clause is
identifiable by `C-*` ID and traceable in `traceability-matrix.jsonl`.

---

## C-1. Domain Model

### C-1.1 Ubiquitous Language (MUST)

The terms defined in `domain-model.md` §1 are normative throughout the
fix. Anyone reviewing production code, tests, or proof artifacts MUST
use these terms exactly. Paraphrases (`"out-of-order error"`,
`"missing-seq error"`) are non-compliant.

### C-1.2 Forbidden State FS-1

The state `events_for_run(run)` returning a non-contiguous prefix where
the gap was created during this process's lifetime is **unreachable**
via any in-process append path. Under the fix, no append path may
durably commit an event whose `seq` does not equal
`next_sequence_at_write(run)` at the moment of write.

### C-1.3 Forbidden State FS-3

A silent rewrite of `event.seq()` to match `expected` is forbidden.
The fix MUST reject, not correct.

## C-2. New Method: `FjallJournal::next_sequence_at_write`

### C-2.1 Signature (MUST)

```rust
pub fn next_sequence_at_write(
    &self,
    run: RunId,
) -> Result<EventSeq, JournalError>;
```

Implementation MUST match this signature exactly; downstream crates rely
on it for re-export and re-binding.

### C-2.2 Return Value for Fresh Run (MUST)

When the `events` keyspace contains zero entries whose key starts with
`run_prefix_key(run)`, the function returns `Ok(EventSeq::ZERO)`.

### C-2.3 Return Value for Non-Empty Run (MUST)

When the keyspace contains at least one entry, let `seq_max` be the
largest seq among them. The function returns
`Ok(codec::next_seq(seq_max)?)`.

### C-2.4 Lookup Discipline (MUST)

The lookup MUST be key-only. Specifically:

- Use `self.events.prefix(run_prefix_key(run)?).next_back()`.
- Decode only the matching key (17 bytes) into `StorageKey::RunEvent`.
- Do NOT decode the matching value.
- Do NOT iterate the whole prefix range.

### C-2.5 Overflow (MUST)

If `seq_max == EventSeq::MAX`, the function returns
`Err(JournalError::SequenceOverflow)`.

### C-2.6 Identifier Hygiene (MUST)

For `run == RunId::ZERO`, the function returns
`Err(JournalError::InvalidRunId { run })` (existing variant) rather
than a seq value. This keeps invalid-identifier semantics consistent
across all storage paths.

### C-2.7 Locking (MUST NOT)

The function MUST NOT acquire `self.write_lock`. The function is
documented as lock-free and concurrent-read-safe.

### C-2.8 Public Wrapper (MUST)

`crates/vb_storage/src/public_api.rs` MUST expose:

```rust
pub fn next_sequence_at_write(
    journal: &FjallJournal,
    run: RunId,
) -> Result<EventSeq, JournalError> {
    journal.next_sequence_at_write(run)
}
```

### C-2.9 No Panic (MUST NOT)

The function MUST NOT panic, `unwrap`, `expect`, `todo`,
`unimplemented`, or `dbg`. `EventSeq::MAX` overflow is reported as
`Err(SequenceOverflow)`.

## C-3. New Variant: `JournalError::SequenceMismatch`

### C-3.1 Variant Declaration (MUST)

```rust
#[error(
    "journal append sequence mismatch for run {run:?}: \
     expected {expected:?}, actual {actual:?}"
)]
SequenceMismatch {
    run: RunId,
    expected: EventSeq,
    actual: EventSeq,
},
```

placed in `crates/vb_storage/src/error/mod.rs` alongside the existing
variants.

### C-3.2 Field Pre-Condition (MUST)

The constructor pre-condition `expected != actual` MUST hold for any
constructed value. A test in `error_tests.rs`
(`sequence_mismatch_constructor_fields`) verifies non-equality on
constructed values.

### C-3.3 Diagnostic Code (MUST)

```rust
pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode =
    DiagnosticCode::new(0x4042);
```

The arm in `diagnostic_code()` MUST be:

```rust
Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE,
```

The arm in `symbolic_code()` MUST be:

```rust
Self::SequenceMismatch { .. } => "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE",
```

### C-3.4 Code-Registry Status (PREFERRED)

`"JOURNAL_SEQUENCE_MISMATCH_AT_WRITE"` SHOULD be registered in
`SymbolicCode::CODE_REGISTRY`. If registration is not feasible for v1,
the existing fallback to `SymbolicCode::INTERNAL_INVARIANT` is
acceptable for v1 and matches the historic convention for
unregistered `0x40xx` codes.

### C-3.5 Coexistence with `SequenceGap` (MUST)

`SequenceGap { expected, actual }` continues to exist as the read-time
diagnostic. The two variants are distinct: `SequenceGap` is emitted by
`events_for_run`; `SequenceMismatch` is emitted by an append path.

## C-4. Append Path Contract

### C-4.1 Affected Methods (MUST)

The five append paths inherit a uniform new post-condition:

- `FjallJournal::append_journaled`
- `FjallJournal::append_strict`
- `FjallJournal::append_strict_batch`
- `FjallJournal::append_unfsynced` (`pub(crate)`)
- `JournalWriteBatch::append_event`

For each, on `Ok(())`:

- `event.seq() == next_sequence_at_write(event.run_id()) pre-call`.
- The durable log advances the tail by exactly one event at the matching `(run, seq)`.

On `Err(JournalError::SequenceMismatch { run, expected, actual })`:

- The durable log is unchanged.
- `expected == next_sequence_at_write(event.run_id()) observed`.
- `actual == event.seq()`.

### C-4.2 Guard Precedence (MUST)

The C6 guard precedence in `JournalWriteBatch::append_event`'s
doc-comment is updated so that the new guard sits at slot 3 (between
`event.is_valid()` and the same-batch duplicate check). The order is:

1. Key construction.
2. `event.is_valid()`.
3. **`next_sequence_at_write` guard** (NEW).
4. Same-batch duplicate guard.
5. Durable duplicate guard.
6. Count capacity.
7. Per-record encoding.
8. Byte admission.
9. Insert.

### C-4.3 Doc-Comments (MUST)

Each of the five append methods' doc-comments MUST grow to mention the
new guard and the `SequenceMismatch` outcome. No method's public
post-condition changes; only the failure surface widens by one variant.

### C-4.4 Batch Atomicity (MUST)

`append_strict_batch` rejects the entire batch on the first
`SequenceMismatch`. No partial durable commit. The
`JournalWriteBatch` reaches the same property via `self.aborted = true`
on the offending `append_event` call.

## C-5. No Silent Rewrite (MUST NOT)

The fix MUST NOT silently rewrite `event.seq()` to match
`expected`. The post-condition is `actual == event.seq()` of the
**originating call**; the implementation may compare against
`expected` but MUST NOT mutate `event`.

## C-6. Existing Tests Must Update

### C-6.1 `crates/vb_storage/src/tests.rs:1737`

The test `append_strict_rejects_out_of_order_sequence` MUST be updated
so that the FIRST `append_strict(&event2)` call returns
`Err(JournalError::SequenceMismatch { expected: EventSeq::new(1), actual: EventSeq::new(2), .. })`.
A SECOND `events_for_run` call afterwards MUST observe only the seq=0
event (no seq=2 event was committed).

### C-6.2 `crates/vb_storage/src/tests.rs:4612`

The test `adversarial_read_events_with_sequence_gap_returns_exact_gap`
MUST be updated so that the `append_journaled(seq=5)` call returns
`Err(JournalError::SequenceMismatch { expected: EventSeq::new(1), actual: EventSeq::new(5), .. })`.
The test name or comment MUST be revised, because the gap-detection
behaviour moves from `events_for_run` to the append path.

### C-6.3 Tests Not Updated

The following tests continue to assert the existing behaviour and MUST NOT be modified except for variant arm additions:

- `append_strict_rejects_duplicate_event` at `journal/tests.rs:409`.
- `adversarial_append_duplicate_sequence_rejected_with_exact_fields` at `tests.rs:4585` (subject to reclassification; see §6.4).

### C-6.4 Test at `tests.rs:4585` Reclassification

The test at line 4585 currently asserts `DuplicateEvent` for a
same-seq retry after a successful commit. Under the fix, the
retry's seq (0) does not match the expected (1), and the second
append returns `SequenceMismatch` — NOT `DuplicateEvent`. The test
MUST be reclassified:

- Either rename the test to assert `SequenceMismatch`.
- Or add a new test that asserts `DuplicateEvent` for the genuine retry
  case (which only fires inside a single batch where the durable tail
  has not advanced).

The implementer MUST choose one of these two paths and document the
choice in the test-planner artifact.

## C-7. New Behavior Tests (Seeds Only)

`test-planner` owns the final layout; this contract emits the
must-have test seeds:

- `append_strict_rejects_sequence_skipped_with_typed_error`.
- `append_strict_rejects_sequence_at_zero_for_run_with_history`.
- `append_strict_accepts_first_seq_for_fresh_run`.
- `next_sequence_at_write_returns_zero_for_fresh_run`.
- `next_sequence_at_write_returns_last_plus_one_after_writes`.
- `append_strict_batch_rejects_on_first_mismatch_atomically`.
- `append_unfsynced_uses_next_sequence_at_write_guard`.

## C-8. Proof / Test Surface (Lane Hint, NOT a Lane Decision)

| Layer | Suggested scope | Owning stage |
|---|---|---|
| Kani | `kani-sequence-at-write` feature-gated harness group; small (n=0..4) tree enumerations. | `proof-writer`. |
| Verus | None new. Existing `recovery_types_spec.rs` unaffected. | n/a |
| Flux | None new. No new refinement boundary. | n/a |
| proptest | `proptest_journal_sequence_at_write` covering random valid/invalid append sequences. | `test-planner` / `proof-writer`. |
| fuzz | Update four fuzz harnesses' arm lists for the new variant. | `proof-writer`. |

These are seed-only hints. `proof-planner` decides final lanes and
`proof-plan-reviewer` accepts them.

## C-9. Kani Harness Isolation (MUST)

The new Kani harness group MUST be gated behind the new Cargo feature
`kani-sequence-at-write` AND the existing `#[cfg(kani)]` attribute.
`crates/vb_storage/src/lib.rs` MUST register the module under
`#[cfg(all(kani, feature = "kani-sequence-at-write"))]` to comply
with AGENTS.md kani-harness-isolation rule.

## C-10. Downstream Caller Audit (Gating Condition)

This contract assumes (per the bead description) that no downstream
caller legitimately writes a non-contiguous `seq`. The implementer
MUST audit `crates/vb_runtime` and `crates/vb_storage::recovery` for
such callers before closing the bead. If a caller exists, the
contract widens (see `domain-model.md` ODQ-1).

## C-11. Acceptance Gate (Final Gate)

A complete closure MUST include raw evidence for:

```
cargo test -p vb_storage --lib append_strict_rejects_sequence_skipped
cargo test -p vb_storage --lib next_sequence_at_write
cargo test -p vb_storage --lib --features kani-sequence-at-write
cargo test -p vb_storage --test proptest_journal_error_codes -- --nocapture
moon run :nightly-feature-gate
moon ci
```

(The Kani command is informational; actual Kani runs are owned by the
formal-verifier stage.)

## C-12. Cross-Stage Hand-Off

- **rust-contract (this stage):** produces the 9 artifacts under `.beads/vb-r8oso/`.
- **test-planner:** plans behaviour tests per C-7.
- **holzman-rust:** implements per C-2..C-5; updates tests per C-6.
- **proof-planner:** plans proof coverage per C-8 lane hints.
- **proof-writer:** writes Kani harness group behind the new feature; fuzz updates.
- **black-hat-reviewer:** verifies no-silent-rewrite invariant C-5; verifies C-6 test rewrites; verifies diagnostic code C-3.3.

The contract closes when `traceability-matrix.jsonl` covers every C-* clause with an owner and a target artifact, and every proof seed has a target verifier lane (yet undecided by planner).
