# Concurrency Test Repair v12 (local3090)

Bead: vb-qz2yn

## Status: PENDING

Compiler and test execution by coordinator.

## Changes

**File:** `crates/vb_storage/src/admission/acceptance_sequence_concurrency_tests.rs`

### Problem
- `WorkflowDigest` does not implement `Ord` (compiler error E0277 at line 74)
- External `Arc<Mutex<TestJournal>>` wrapped the entire journal, serializing all calls before they reached production's internal `write_lock` - defeating the concurrency test purpose

### Fix
1. **Removed external Arc/Mutex wrapper entirely.** Threads now share a direct `&FjallJournal` reference through `std::thread::scope`. Production's internal write_lock provides serialization; no outer mutex hides it.

2. **Fixed Ord dependency.** Replaced `digests.sort()` + `dedup()` with `HashSet` insertion. `WorkflowDigest` derives `Hash` and `Eq` but not `Ord` - confirmed from source at `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:268`.

3. **Added full readback with postcard decode.** For each of the 8 artifacts:
   - Retrieve `CompiledIrRecord` via `journal.compiled_ir(artifact.digest)`
   - Decode with `postcard::from_bytes(&record.ir)` into `AcceptedArtifact`
   - Assert `decoded == *artifact` (full identity check)
   - Assert `record.digest == artifact.digest`

4. **Preserved all canonical assertions:**
   - `accepted_artifact_digest(artifact)` matches `artifact.digest`
   - `artifact.verification.digest` matches computed digest
   - `verify_accepted_artifact_for_policy(artifact, RuntimePolicy::Relaxed)`
   - Sequence numbers form exact set {1, 2, 3, 4, 5, 6, 7, 8}
   - 9th submission receives seq 9

5. **Thread structure:**
   - `std::thread::scope` with 8 spawned threads
   - Each thread clones workflow, calls `submit_artifact(&journal, &wf, RuntimePolicy::Relaxed)` directly
   - Returns `Result<AcceptedArtifact, String>`
   - All 8 joins checked with error propagation
   - No sleeps, try-locks, or artificial synchronization

### Sync Assumption
Test relies on `FjallJournal` implementing `Sync`. If the compiler reports a missing `Sync` bound, that is the exact error to report to coordinator - do not add fake outer serialization. The test is designed to validate production's internal write_lock, not work around its absence.

## Verification
- File: 108 lines (well under 300 limit)
- No `unsafe`, `unwrap`, `expect`, `panic`, unchecked indexing, arithmetic, or casts
- No lint waivers
- No production code changes
- No test hook or module hook changes
