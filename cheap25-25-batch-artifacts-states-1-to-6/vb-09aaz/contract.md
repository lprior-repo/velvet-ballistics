# Contract: vb-09aaz — Abort Batch on All Index Key Construction Failures

## Acceptance Contract for Downstream Lanes

The storage layer must abort `JournalWriteBatch` on any fallible
step's `Err`, including the G8 IndexKeyConstruction step currently
missing from `append_event`. The fix restores parity with the
existing abort-on-fallible-step contract already implemented in
the `put_*` methods (28 occurrences in `putters.rs`).

### C1 Abort-on-Fallible-Step Invariant (Cross-Method)

Every fallible step in `JournalWriteBatch::append_event` and
`JournalWriteBatch::put_*` MUST set `self.aborted = true` BEFORE
propagating a typed error, with the explicit exceptions:

- **G1 KeyConstruction** (`run_event_key`): fires BEFORE any state
  mutation. The abort flag stays `false` because there is no
  partial state to abort.
- **G2 SameBatchDuplicate** (`DuplicateStagedKey`): fires BEFORE
  any state mutation.
- **G4 BatchCount** (`QueueFull`): fires BEFORE any state mutation.
- **G5 PerRecordEncoding** (`Encode`, `PayloadTooLarge`): fires
  BEFORE any state mutation (the encoding is a transient
  computation).
- **G6 AccumulatedByteAdmission** (`JournalBatchBytesExceeded`,
  `SequenceOverflow`): fires BEFORE any state mutation.

The remaining fallible step in `append_event` is G8, which fires
AFTER the journal event is staged into `inner` at G7. G8 MUST set
`aborted = true` on Err. This is the missing piece addressed by
vb-09aaz.

### C2 G8 Guard Precedence (C6 Update)

The doc-comment at append_event.rs:18-26 must enumerate the
8-guard order as:

```text
1. Key construction            (G1)
2. Semantic event validation
3. Same-batch duplicate        (G2)
4. Durable duplicate           (G3, aborts)
5. Count capacity              (G4)
6. Per-record encoding         (G5)
7. Accumulated byte admission  (G6)
8. Insert into inner OwnedWriteBatch (G7)
9. Pending-action-index key construction (G8, aborts) [NEW]
```

G8 is the final fallible step before the infallible
`staged_event_keys.insert(key)` and `Ok(())` return. The doc-comment
currently stops at step 8 (which is G7 in this canonical numbering);
the fix MUST add G9 (which corresponds to the G8 step in the
Verus spec's numbering) to the enumeration.

### C3 Typed Error Propagation

On `Err(JournalError::KeyCapacity)` from G8, the typed error
propagates to the caller unchanged. The fix does not introduce a
new error variant; it reuses the existing `JournalError::KeyCapacity`
(error/mod.rs:28-29, diagnostic code `KEY_CAPACITY_EXCEEDED`).

### C4 Post-Condition: Aborted State on G8 Err

After `append_event` returns `Err(JournalError::KeyCapacity)` from
G8, the batch is in the aborted state:

- `batch.is_aborted() == true`.
- `batch.commit()` returns `Err(JournalError::BatchAborted)`
  (commit.rs:20-23).
- No partial persistence: the journal event for this batch is not
  committed; the index_action mutation is not committed; the Fjall
  database state is unchanged.
- No events are durable for the run.

The Postconditions (ensures) doc-comment at append_event.rs:33-41
MUST add a new bullet:

```text
- On `KeyCapacity` (G8, index-key construction failure):
  batch is aborted; no partial persistence; commit() returns Err(BatchAborted).
```

### C5 No Partial Persistence (Master §49 Compliance)

The fix enforces master §49 Crash-Consistency Rule by ensuring
that any index-key failure aborts the batch and surfaces
`Err(JournalError::BatchAborted)` on commit. The journal event and
the pending-action index mutation are durable together or not at all.

This is the SAME invariant as the existing G3 DurableDuplicate
post-condition, extended to the G8 IndexKeyConstruction post-condition.

### C6 Public API Stability

The public API surface is unchanged:

- `pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>` — signature unchanged.
- `pub fn is_aborted(&self) -> bool` — accessor unchanged.
- `pub fn commit(self) -> Result<(), JournalError>` — signature unchanged; short-circuit behavior unchanged.

The post-condition of `append_event` gains a new clause (C4 above).
The signature, error variants, and accessor surface are unchanged.

### C7 Verus Spec Extension (PS-008, PS-009)

The Verus spec files at `verification/verus/vb-vzcuf-PS-008.rs`
and `verification/verus/vb-vzcuf-PS-009.rs` MUST be extended to
model the G8 guard:

1. Add a new mirror input (e.g., `index_key_ok: bool`) to the
   `SpecJournalWriteBatch::append_event` exec fn.
2. Add a new G8 step to the mirror body that mirrors the
   abort-on-error pattern:
   ```rust
   // Guard G8: index-key construction.
   if !index_key_ok {
       self.aborted = true;
       return Err(SpecJournalError::KeyCapacity);
   }
   ```
3. Update the `assume_specification` contract to add a new match
   arm for `Err(KeyCapacity)` requiring
   `spec_state_preserved_except_aborted(*old(batch), *final(batch))`
   with witness `!index_key_ok`.
4. Update the production mirrors at
   `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:78-95`
   and `_PS_009_production.rs:67-93` to enumerate G8 in the guard
   order comment block.
5. Add a new exec wrapper `wrapper_append_event_index_key_error`
   to exercise G8 from `verus!` context.

The fix MUST pass `bash scripts/verify-verus.sh` and
`bash scripts/check-verus-production-binding.sh` per AGENTS.md
mandatory gates.

### C8 Test Coverage

A new regression test MUST be added at
`crates/vb_storage/src/batch/t_append_event.rs` mirroring the
existing `batch_index_key_error_aborts_commit` at
`batch/t_putters_b.rs:177-209`. The new test
(`batch_append_event_index_key_error_aborts_commit`) MUST assert:

1. `append_event` returns `Err(JournalError::KeyCapacity)`.
2. `batch.is_aborted() == true`.
3. `batch.commit()` returns `Err(JournalError::BatchAborted)`.
4. No events are durable for the run (`events_for_run(run).is_empty()`).

A proptest variant (`proptest_vb_hyog0_PS_010.rs` or extension to
`proptest_vb_vzcuf_PS_004.rs`) is RECOMMENDED to hammer the G8
path with arbitrary `ActionId/RunId/StepIdx` triples and assert
the abort invariant under all inputs.

### C9 Doc-Comment Update

The fix MUST update the Guard Precedence (C6) doc-comment at
append_event.rs:18-26 to enumerate G8 alongside G1..G7, and the
Postconditions (ensures) doc-comment at L33-41 to document the
new abort invariant for KeyCapacity on G8.

## Non-Goals

- Do NOT introduce a new `JournalError` variant; reuse `KeyCapacity`.
- Do NOT modify `putters.rs`; it already follows the abort-on-fallible-step contract.
- Do NOT modify `commit.rs`; the short-circuit at L20-23 is the existing mechanism.
- Do NOT modify `stage_pending_action_index_op`; its post-condition is correct.
- Do NOT modify the queued-writer path (`queue/writer/stage.rs`); its single-shot batch is not vulnerable to partial-write hazards.
- Do NOT modify the direct path (`journal/internal.rs::append_unfsynced`); its fresh-batch construction is not vulnerable to partial-write hazards.
- Do NOT change the public API signature of `append_event`, `is_aborted`, or `commit`.
- Do NOT add new fields to `JournalWriteBatch`.
- Do NOT implement production Rust, write tests, or write verifier artifacts in this contract state.

## Open Domain Questions

1. **Staged-event-keys insertion order**: append_event.rs:119 inserts the key AFTER the G8 `?`. If G8 fires, the key is not in the set. The contract RECOMMENDS moving the insert to before G8 to guarantee same-batch rejection across G8-failed batches. This is flagged for the downstream contract owner; not addressed in vb-09aaz.
2. **KeyCapacity reachability in spec**: the Verus production mirror currently declares `KeyCapacity` unreachable (PS-008 L174, PS-009 L168-171). The contract recommends adding a new mirror input `index_key_ok: bool` for G8 and keeping `KeyCapacity` unreachable for the G1 run_event_key path. This decision belongs to the proof-writer; this contract only flags the requirement.
3. **Test trigger feasibility**: under production `index_action_key`, KeyCapacity is unreachable for nominal inputs. The test-planner must choose between (a) constructing a degenerate event, (b) using a proptest with arbitrary triples, or (c) a unit test that exercises the abort-on-error pattern directly via state manipulation. See `boundary-map.md` Test Boundary section for details.

## Cross-References

- **Codebase map**: `.beads/vb-09aaz/codebase-map.md` — primary defect site, secondary sites, reference patterns, and existing test coverage.
- **Delivery scope**: `.beads/vb-09aaz/delivery-scope.jsonl` — 27 file entries with risk tags, contract parity, and verifier lanes.
- **Verus spec**: `verification/verus/vb-vzcuf-PS-008.rs`, `verification/verus/vb-vzcuf-PS-009.rs` — spec files to be extended with G8.
- **Production mirror**: `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`, `verification/verus/production_inner/vb_vzcuf_PS_009_production.rs` — drift-gated mirrors to be regenerated.
- **Reference test**: `crates/vb_storage/src/batch/t_putters_b.rs:177-209` (`batch_index_key_error_aborts_commit`) — pattern to mirror in `batch/t_append_event.rs`.
- **Master plan**: `velvet-ballistics-MASTER.md` §49 — Crash-Consistency Rule.
- **Domain model**: `.beads/vb-09aaz/domain-model.md` — ubiquitous language, aggregate, policies, invariants.
- **Type contracts**: `.beads/vb-09aaz/type-contracts.md` — required types, API contract, illegal states.
- **Workflow model**: `.beads/vb-09aaz/workflow-model.md` — typestates, decision table, terminal outcomes.
- **Error taxonomy**: `.beads/vb-09aaz/error-taxonomy.md` — error families and G8 details.
- **Boundary map**: `.beads/vb-09aaz/boundary-map.md` — pure core, imperative shell, persistence boundary, test boundary.
- **Hazard analysis**: `.beads/vb-09aaz/hazard-analysis.md` — H1..H12 hazards.
- **Proof seeds**: `.beads/vb-09aaz/proof-seeds.jsonl` — domain-level proof hints.
- **Traceability**: `.beads/vb-09aaz/traceability-matrix.jsonl` — requirement-to-artifact-to-proof-seed mapping.