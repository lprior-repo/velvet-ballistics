# vb-0qwfq Producer Audit v1 — Parent Acceptance Review

**Scope:** Persisted production artifact producers only. Not all v1/branches/proof closure.
**Candidate:** 4df9e4f196e3047a280cb2d1755488afd1bfd60b

## Findings

### Verified Producer Entrypoints (All Stamp Sequence)

| # | Entrypoint | Path | Issuance Mechanism |
|---|------------|------|-------------------|
| 1 | `admission::submit_artifact(journal, workflow, policy)` | `crates/vb_storage/src/admission.rs:341` | Delegates to #2 |
| 2 | `admission::submit_artifact_with_contracts(journal, workflow, policy, contracts)` | `crates/vb_storage/src/admission.rs:351` | `issue_next_artifact_sequence(journal, artifact, policy)` |
| 3 | `ExecutorCatalog::submit_artifact(journal, workflow, policy, contracts, packs)` | `crates/vb_storage/src/executor/packed.rs:48` | `issue_next_artifact_sequence(journal, artifact, RuntimePolicy::Strict)` |
| 4 | `FjallJournal::commit_run_submission(submission, policy)` | `crates/vb_storage/src/inputs/submission.rs:180` | `prepare_artifact_with_sequence(journal, artifact)` → `batch.put_artifact_sequence(seq)` |

All four public producer entrypoints acquire `write_lock`, read counter, increment, stamp `accepted_at_seq`, rebind artifact digest, and commit counter+artifact writes atomically in a single `fjall::OwnedWriteBatch`.

### Non-Producers (Correctly Do Not Stamp)

| Entrypoint | Role | Sequence Behavior |
|------------|------|-------------------|
| `admission::prepare_compiled_artifact(...)` | Pure preparation | seq=0, no persistence |
| `admission::verify_accepted_artifact(artifact)` | Verification only | N/A |
| `admission::verify_accepted_artifact_for_policy(artifact, policy)` | Verification only | N/A |
| `ExecutorCatalog::prepare_artifact(...)` | Pure preparation | seq=0, no persistence |
| `ExecutorCatalog::verify_artifact(artifact, policy)` | Verification only | N/A |
| `vb_storage::put_compiled_ir(journal, record)` | Low-level raw record storage | No sequence issuance |
| `JournalWriteBatch::put_compiled_ir(record)` | Batch raw record storage | No sequence issuance |
| `FjallJournal::put_compiled_ir(record)` (internal) | Raw record storage | No sequence issuance |

### Synthetic/Compatibility Store

`AlwaysPresentArtifactStore` (`crates/vb_runtime/src/admission/parts/chunk_003_stores.rs`) produces artifacts with `accepted_at_seq: EventSeq::new(0)` sentinel. This is by-design for the relaxed/synthetic compatibility path that does not interact with the journal counter. No journal issuance occurs.

### Private Counter Authority and Locking

- **Single counter:** `artifact_acceptance` keyspace, key `last_issued` (big-endian u64)
- **Locking:** `FjallJournal.write_lock` (Mutex) acquired by all producer paths before counter read
- **Atomicity:** Counter update and artifact write staged together in one `fjall::OwnedWriteBatch`, committed together
- **Overflow:** `checked_add(1)` on counter; `NonZeroU64` wrapper enforces non-zero issued sequences

### Zero Sentinel Distinction

- seq=0: Pure prepare artifacts, synthetic `AlwaysPresentArtifactStore` artifacts, test fixtures
- seq≥1: Issued artifacts from all four public producer entrypoints

### Policy Durability

- Strict: `PersistMode::SyncAll` (fsync) before return
- Journaled: Batch commit without fsync (data-loss window)
- Relaxed: Batch commit without fsync (data-loss window)

## Test Evidence

From `crates/vb_storage/src/executor/issuance_tests.rs` and `crates/vb_storage/src/inputs/issuance_tests.rs`:

- `prepare_produces_no_sequence`: Pure prepare leaves counter None, seq=0
- `strict_submit_sequence_one`: First submit returns seq=1
- `sequential_submissions_increment`: Three policies increment counter 1,2,3
- `mixed_default_and_catalog_submissions`: Default and catalog share counter
- `same_journal_four_way_sequence`: default(1) → catalog(2) → CLI(3) → default(4) proves shared counter across all three producer paths
- `journal_reopen_preserves_sequence`: Persistence across reopen
- `cli_pure_prepare_does_not_advance_counter`: Preparation without commit does not advance counter

## Missing Producer Analysis

RG inventory search across the repository for `put_compiled_ir` and `AcceptedArtifact` construction patterns found:

- All production persisted producers call through the four verified entrypoints above
- No additional producer entrypoints found that bypass `issue_next_artifact_sequence` or `prepare_artifact_with_sequence`
- Test fixtures use seq=0 sentinel appropriately
- No low-level `put_compiled_ir` calls in production producer paths

## Remaining Concrete Issues

NONE — all verified persisted production artifact producers stamp `accepted_at_seq` via the private single-counter issuance path.

## Coverage Conclusion

All four public persisted artifact producer entrypoints (default admission, default with contracts, registered executor catalog, CLI run submission) correctly acquire the journal write lock, atomically increment the private `artifact_acceptance` counter within a single batch with the artifact write, stamp the issued sequence on the artifact, and rebind the artifact digest. The zero sentinel is correctly reserved for pure prepare and synthetic compatibility artifacts. No additional persisted production producer entrypoints bypass the issuance mechanism.