# vb-vs630 Test Review v38 — Corrected

**Status:** REJECTED — genuine assertion coverage gaps remain

**Prior review:** v35 was rejected for conflating unreachable error variants with genuine coverage gaps, and for claiming full-record preservation coverage that the tests do not actually assert. This review corrects those errors.

## Contract Requirements

From `.beads/vb-vs630/contract.md`, the required behavior evidence includes:

1. All three policies (Relaxed, Journaled, Strict)
2. Mixed default/catalog/CLI global ordering
3. Typed retained inputs plus all four rebound digest fields
4. Per-run event0/1 semantics; workflow header unchanged
5. Malformed 7/9 byte counter probes
6. u64::MAX overflow
7. Duplicate-run rejection before counter publication
8. Prevalidation/fault failures with exact unchanged state before retry
9. Same-path close/reopen state preservation
10. Bounded concurrent submissions
11. Pure preparation remains zero
12. Existing default and registered helper durability unchanged

## Coverage Assessment

### Fully Covered

- **All three policies:** `issuance_tests.rs` has explicit tests for Strict, Journaled, and Relaxed.
- **Per-run event0/1:** `cli_journaled_events_have_correct_sequences` asserts RunAccepted at seq 0, RunAdmission at seq 1.
- **Relaxed writes no events:** `cli_relaxed_writes_no_events` asserts empty events.
- **Malformed 7/9 byte counters:** `issuance_boundary_tests.rs` has explicit 7-byte and 9-byte probes with exact error variant assertions.
- **u64::MAX overflow:** `cli_counter_at_max_yields_sequence_overflow` and `cli_counter_at_max_minus_one_allows_one_more` cover the boundary.
- **Pure preparation zero:** `cli_pure_prepare_does_not_advance_counter` asserts counter is None and calls `verify_empty_journal` which checks all 7 keyspaces.
- **Sequential CLI submissions:** `cli_sequential_submissions_increment_counter` verifies monotonic counter.
- **Fjall IO error propagation:** `cli_injected_batch_commit_failure_*` tests verify `JournalError::Fjall(...)` with IoOther kind.

### Partially Covered — Assertion Gaps

These tests exist but do not fully assert what the contract requires:

**F1: Duplicate rejection — incomplete record preservation check**

`issuance_failure_tests.rs::cli_duplicate_run_rejected_before_counter_publication` only asserts that `run_inputs` and `events_for_run` are unchanged after the duplicate submission. It does not verify that compiled IR, source, header, or index entries are preserved. A bug that overwrote only some records would pass this test.

Contract requires "exact unchanged state before retry" — this test asserts counter (1), run_inputs, and events. It omits compiled_ir, workflow_source, run_header, index_workflow, and index_status.

**F2: Close/reopen — not full record equality**

`issuance_boundary_tests.rs::cli_same_path_reopen_preserves_state_and_advances` verifies input digests, typed inputs, compiled IR bytes, source bytes, header compiled_digest, and events count. It does not assert full record object equality. Specifically:

- Events: only asserts `len()`, not full event record content (only one event's artifact_digest is spot-checked)
- Header: only asserts `compiled_digest`, not `run`, `workflow_id`, `status`, `accepted_at_ms`
- No index entry verification (workflow index, status index)

A serialization bug that corrupted only some fields of header or events could pass.

**F3: Concurrency — no per-run typed data verification**

`issuance_boundary_tests.rs::cli_bounded_concurrent_submissions` asserts the counter is 4 and that the four runs received sequences [1,2,3,4]. It does not verify that each run retained its correct typed inputs, source, header, or index entries. A cross-run contamination bug (e.g., run 100 getting run 101's inputs) would not be detected.

**F4: Mixed submission — catalog not included in any test**

Contract requires "Mixed default/catalog/CLI global ordering" as evidence. The test suite has:

- `issuance_tests.rs::mixed_default_and_cli_submissions_share_counter` — default + CLI only
- `executor/issuance_tests.rs::mixed_default_and_catalog_submissions` — default + catalog only

No single test exercises all three submission methods in one journal with a shared counter. The pairwise tests do not prove the three-way interaction.

### Error Variant Reachability Analysis

The contract lists these as "Exact Actual Errors":

| Variant | Reachable? | Tested? | Notes |
|---------|------------|---------|-------|
| `MalformedKeyspaceRow` | Yes | Yes | 7/9 byte probes |
| `SequenceOverflow` | Yes | Yes | u64::MAX |
| `WriteLockPoisoned` | Technically yes, practically no | No | Requires panic while holding lock; no `panic!`/`expect!`/`unwrap!` permitted in production or tests. Preexisting infrastructure guard, not contract-required test target. |
| `RunAlreadyExists` | Yes | Yes | Duplicate submission |
| `Fjall(...)` | Yes | Yes | Injected batch failure |
| `PostcardEncodeFailed` | No (for closed types) | No | `to_allocvec` failure requires OOM or custom fallible serializer. `AcceptedArtifact` is closed derived Serialize (tuples, scalars, Vec<u8>, closed enums). OOM is panic, not Result. |
| bind_artifact_digest errors | No (distinct) | No | `bind_artifact_digest` calls `accepted_artifact_digest` which calls `postcard::to_allocvec`. A bind failure IS a `PostcardEncodeFailed`. No distinct bind error variant exists. |

## True Gaps (REJECTION BASIS)

Four genuine coverage gaps remain:

1. **F1:** Duplicate test omits compiled IR, source, header, index preservation assertions
2. **F2:** Close/reopen test omits full header field, full events content, index verification
3. **F3:** Concurrency test omits per-run typed data integrity verification
4. **F4:** No mixed default+catalog+CLI single-journal test

These are not error-variant reachability claims. They are assertions about what the tests actually verify versus what the contract requires. The tests exist but assert fewer invariants than the contract demands.

## Not Gaps

- **WriteLockPoisoned:** Preexisting guard; unreachable without panic macros; not contract-required test target.
- **PostcardEncodeFailed:** Unreachable for closed derived Serialize types at reasonable sizes.
- **Malformed counter at all lengths:** Contract explicitly requests only 7/9 byte probes; no authority to expand to infinite enumeration.
- **Pure preparation zero:** Correctly asserts counter is None AND all keyspaces empty via `verify_empty_journal`.

## Required Fixes

| Fix | File | Test |
|-----|------|------|
| F1 | `issuance_failure_tests.rs` | Extend `cli_duplicate_run_rejected_before_counter_publication` to snapshot and reassert compiled IR, source, header, workflow index, status index after duplicate attempt |
| F2 | `issuance_boundary_tests.rs` | Extend `cli_same_path_reopen_preserves_state_and_advances` to assert full header field equality, full events content equality, and index entry existence after reopen |
| F3 | `issuance_boundary_tests.rs` | Extend `cli_bounded_concurrent_submissions` to assert per-run typed input values, source digests, and index entries match expected values for each run |
| F4 | New test | Create mixed default + registered catalog + CLI test that submits one of each in sequence and asserts the shared counter reaches 3 with correct per-artifact seq values |
