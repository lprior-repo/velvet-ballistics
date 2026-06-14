P2-15r index-status-workflow: Audit whether index_status and index_workflow Fjall keyspaces are populated during submit/recover (NOT a pending_actions gate)

# Verification excerpts (read-before-write)

## crates/vb_storage/src/indexes.rs (68 lines)
- Line 15-24: `pub fn put_status_index(&self, state: IndexStatusState, timestamp: u64, run: RunId) -> Result<(), JournalError>` — method on FjallJournal.
- Line 27-35: `pub fn put_workflow_index(&self, workflow: WorkflowId, run: RunId) -> Result<(), JournalError>`.
- Line 22: `self.index_status.insert(key.to_vec(), Vec::<u8>::new())` — `index_status` is a Fjall keyspace (field on FjallJournal).
- Line 33: `self.index_workflow.insert(key.to_vec(), Vec::<u8>::new())` — `index_workflow` is a Fjall keyspace.

## crates/vb_storage/src/tests/chunk_032.rs (213 lines)
- Line 67-91: Test `status_index_multiple_runs_same_state` accesses `journal.index_status.get(key.as_slice())`.
- Line 24-30: Test `workflow_index_stores_and_queries_by_workflow_id` accesses `journal.index_workflow.get(key.as_slice())`.
- Both tests verify the keyspaces are populated.

## crates/vb_storage/src/recovery/types.rs (657 lines)
- Line 300-310: `pub struct UnsupportedRecoveryState { pub slot_values: bool, pub slot_taint: bool, pub action_payloads: bool, pub pending_actions: bool }` — `pending_actions` is a BOOL FLAG indicating "pending actions cannot be projected into the runtime frame yet", NOT a method or gate.
- Line 339-346: `pub const fn pending_actions_unsupported() -> Self` — factory for the flag, not a gate.

## Master doc §44.15 — DOES NOT EXIST
- §44 is "Backend / IR Interpreter Definition of Done" with 24 numbered points (line 2039-2064). No §44.15 (the master has only 24 points; "15" is within those 24). The "operational affordances" framing is INVENTED.

# Round-2 corrections applied (from black-hat review)

The round-2 bead's "pending_actions gate" framing is wrong. There is no `pending_actions` API or gate — only `UnsupportedRecoveryState::pending_actions: bool` field. The actual audit is about whether `index_status` and `index_workflow` Fjall keyspaces are populated during submit/recover.

# Scope (verified, no fabrication)

Audit (read-only) the `submit_artifact` and `recover_runtime_frame_seed` code paths to determine:
1. Does `submit_artifact` call `put_status_index` and `put_workflow_index` for the new run?
2. Does `recover_runtime_frame_seed` rebuild the index entries from the journal?
3. If neither, this is a real gap (master §63 gate 14 "results" / gate 15 "evidence" require index observability).

# Implementation

Read `crates/vb_storage/src/admission.rs:230-310` (`submit_artifact` and helpers) and grep for `put_status_index` / `put_workflow_index` calls. Also read `crates/vb_storage/src/recovery/recover.rs:251-296` and grep the same.

If the keyspaces are NOT populated:
- Add a one-line call in `submit_artifact_for_policy` after `persist_accepted_artifact_ir` (admission.rs:304): `journal.put_status_index(...)?; journal.put_workflow_index(...)?;`
- Add a corresponding call in `recover_all_incomplete_runs` (recover.rs:289) per-run.

If the keyspaces ARE populated (likely, given chunk_032 tests pass):
- Close this bead with a `bd remember` note documenting the audit and the result.

# Acceptance test

The audit IS the acceptance. If a gap is found, the unit test is:
```rust
#[test]
fn submit_artifact_populates_status_and_workflow_indexes() {
    // Open a test journal.
    // Call submit_artifact.
    // Assert: journal.index_status has at least 1 entry for the run.
    // Assert: journal.index_workflow has at least 1 entry for the run.
}
```

# Anti-hallucination guards

- DO NOT cite "master §44.15" or "operational affordances" — these do not exist in master doc.
- DO NOT add a `pending_actions` gate or method — only the bool flag exists.
- DO NOT remove the dead code without audit — the round-2 "wire vs remove" decision is binary, but the actual scope is "audit the indexes".

# Kani harness (skipped — index operations are bounded inserts; no hot-path arithmetic)

# Dependency

This bead has NO dependencies. (Round-2 had vb-v1jiq (P0-5b) depending on this — that was a P0→P2 inversion, now removed.)
