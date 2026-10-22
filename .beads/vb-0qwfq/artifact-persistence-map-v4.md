# vb-0qwfq artifact persistence callgraph (production only)

Actual production paths that construct, verify and persist AcceptedArtifact.
Read-only investigation; no code, tests, or contract changes.

## Artifact identity chain

AcceptedArtifact is constructed in one place:
`crates/vb_storage/src/admission/verification.rs:prepare_compiled_artifact()`
→ calls `prepare_with_profile()` which builds the envelope and sets
`accepted_at_seq: EventSeq::new(0)` (line 45). This field is never updated
after construction — it remains zero for all persisted artifacts regardless
of the policy tier or the journal's actual event sequence.

## Producer paths

### Path 1: CLI `vb submit` (RunSubmission path)
```
velvet_ballistics/src/submit.rs:prepare_records()
  → vb_storage::admission::prepare_compiled_artifact() [admission.rs:80]
  → constructs RunSubmission with AcceptedArtifact
  → velvet_ballistics/src/submit.rs:commit_submission() [line 128]
  → FjallJournal::commit_run_submission() [submission.rs:137]
  → acquires self.write_lock [submission.rs:145]
  → RunSubmission::commit() [submission.rs:77]
  → JournalWriteBatch staging [submission.rs:83]
  → batch.put_compiled_ir(CompiledIrRecord{digest, ir}) [submission.rs:88]
  → batch.append_event(RunAccepted{seq: EventSeq::new(0)}) [submission.rs:115]
  → batch.append_event(RunAdmission{seq: EventSeq::new(1)}) [submission.rs:125]
  → batch.commit() [submission.rs:79]
```
Lock: `write_lock` acquired in `commit_run_submission` before staging.
Events carry hardcoded seq 0/1, not journal-allocated.

### Path 2: CLI `vb run` (submit_artifact path)
```
velvet_ballistics/src/run.rs:109
  → vb_storage::submit_artifact(journal, compiled, policy) [admission.rs:341]
  → submit_artifact_with_contracts(journal, workflow, policy, &[]) [admission.rs:346]
  → prepare_compiled_artifact(workflow, policy, &[]) [admission.rs:357]
  → postcard::to_allocvec(&artifact) [admission.rs:358]
  → journal.put_compiled_ir(CompiledIrRecord{digest: artifact.digest, ir}) [admission.rs:359]
  → journal.persist_strict() if Strict policy [admission.rs:363]
  → verify_artifact_persisted(journal, artifact.digest) [admission.rs:365]
```
Lock: none explicit at this level; `put_compiled_ir` uses journal's internal mutex.
No events appended for this path — artifact-only persistence.

### Path 3: JournalWriteBatch.put_compiled_ir (batch path)
```
crates/vb_storage/src/batch/putters.rs:71
  → verify_compiled_ir_record_digest(record)
  → stages key in batch (no immediate I/O)
  → committed via JournalWriteBatch::commit()
```
Used by Path 1. Not a standalone producer.

### Path 4: FjallJournal.put_compiled_ir (direct path)
```
crates/vb_storage/src/journal/source.rs:55
  → verify_compiled_ir_record_digest(record)
  → compiled_ir_key(record.digest)
  → encode_record(..., RecordKind::CompiledIr, 0, record, ...) [line 58]
  → self.compiled_ir.insert(key, value) [line 63]
```
The `0` at line 58 is the envelope's seq field — hardcoded, not journal-allocated.
Used by Path 2.

### Path 5: Executor packed path (registered executor profile)
```
crates/vb_storage/src/executor/packed.rs:57
  → self.prepare_artifact() → verification::prepare_with_profile()
  → journal.put_compiled_ir(CompiledIrRecord{digest, ir})
  → postcard::to_allocvec(&artifact)
```
Lock: none explicit; delegates to journal internals.

## Verification path (re-check on runtime use)
```
crates/vb_storage/src/admission/verification.rs:61
verify_accepted_artifact(artifact)
  → envelope digest check
  → decode IR
  → prepare_compiled_artifact(&workflow, policy, contracts)
  → verify_expected_bindings(artifact, &expected)
```
Cold-path re-verification. Does not construct new AcceptedArtifact; re-derives
expected artifact and compares bindings.

## RunInputs path
```
crates/vb_runtime/src/admission/parts/chunk_003_stores.rs:67
accepted_artifact_from_record()
  → accepted_at_seq: EventSeq::new(0) [line 67]
```
Reconstructs AcceptedArtifact from journal records. Always sets seq 0.

## Key observations

1. **`accepted_at_seq` is always zero** in all production paths. The field exists
   in AcceptedArtifact but is never set to a nonzero value. Line 45 of
   `verification.rs` is the sole assignment.

2. **No journal-wide artifact acceptance counter exists.** There is no counter
   that increments across artifacts globally. Event sequences are per-run, not
   per-artifact.

3. **RunSubmission events use hardcoded seq 0/1** (submission.rs:115,125). These
   are not actual journal event sequences; they're static values assigned when
   constructing the events before the batch is committed.

4. **Runtime shard path uses dynamic seq allocation.** In `vb_runtime`, the
   shard lifecycle (e.g., `shard/lifecycle/chunk_005.rs:84`) allocates real
   sequences via `journal_sequence_for(run)` + `checked_add(offset)`. However,
   this happens for run events, not for artifact acceptance.

5. **Two separate concerns conflated:**
   - Artifact acceptance (what this bead addresses): setting `accepted_at_seq`
     to a meaningful value
   - Run event sequencing (already working): per-run event sequences in the
     runtime shard

## Where a nonzero journal-owned seq could be assigned safely

The only location where a journal-owned artifact acceptance seq could be
stamped is **`crates/vb_storage/src/admission/verification.rs:prepare_compiled_artifact()`**,
specifically line 45 where `accepted_at_seq: EventSeq::new(0)` is set.

A safe approach:
1. Add a journal-wide `artifact_seq` counter (u64) stored in the journal's internal state.
2. Expose an atomic `alloc_artifact_seq() -> u64` on FjallJournal.
3. Call it from `submit_artifact_with_contracts()` (admission.rs:357) BEFORE
   `prepare_compiled_artifact()` is called.
4. Pass the allocated seq to a new `prepare_compiled_artifact_with_seq()` variant.
5. The seq is assigned exactly once per artifact, at submission time.

Alternative: stamp after `put_compiled_ir` returns but before `verify_artifact_persisted`.
This would give the actual persisted sequence, but requires rebinding the
artifact envelope (digest would change), which is complex and risky.

Prefer assigning before persistence so the seq becomes part of the artifact
envelope's canonical content.

## Risks and constraints

- **Key space collision:** The seq must not collide with the compiled IR key
  space. Using a u64 counter is safe as long as it's stored under a distinct
  key prefix (e.g., `artifact_seq`).
- **Type confusion:** `accepted_at_seq` is `EventSeq` (u64 wrapper). The journal
  counter must be the same type.
- **Hash integrity:** The artifact digest includes all fields. Changing
  `accepted_at_seq` after the artifact envelope is formed would break the
  digest binding. Assign seq during preparation, not after.
- **Write lock:** If the counter is stored in the journal and read/written
  atomically, it must be done under the `write_lock` (acquired in
  `commit_run_submission` at submission.rs:145) to avoid races with
  concurrent artifact submissions.
- **Recursion:** Do not call a journal method that re-acquires `write_lock`
  while already holding it. The counter allocation must be lock-free or use
  a separate mutex.

## Paths that are unpersisted checks only

- `vb_runtime/src/admission/admit_artifact_run()`: checks artifact for runtime
  admission but does not persist anything.
- `verify_accepted_artifact()`: re-verifies, does not persist.

## Paths that have existing nonzero authority

- Run events in the runtime shard path (`vb_runtime/src/shard/`) have real
  nonzero sequences allocated dynamically.
- These are run-scoped, not artifact-scoped.

## Legacy zero envelope reads

All existing persisted artifacts have `accepted_at_seq = 0`. Any new
nonzero-assignment scheme must remain readable by code that expects zero
(fallback). The floor is 0.
