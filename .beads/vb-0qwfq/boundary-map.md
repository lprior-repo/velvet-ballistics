# Boundary Map: Artifact Acceptance Sequence

## Pure Core
- `accepted_artifact_digest()` — computes BLAKE3 hash of artifact envelope (including seq).
- `bind_artifact_digest()` — sets artifact and proof digest fields.
- `compute_policy_digest()` — hashes resource contract.

## Imperative Shell
- `submit_artifact()` / `submit_artifact_with_contracts()` — orchestrate allocation, preparation, persistence.
- `verify_artifact_persisted()` — readback verification.

## Storage Boundary
- `FjallJournal::accept_seq_allocate()` — atomic counter operation on `system` keyspace.
- `FjallJournal::put_compiled_ir()` — artifact persistence.
- `FjallJournal::persist_strict()` — durability barrier (Strict policy only).

## Concurrency Boundary
- `FjallJournal.write_lock` — `Mutex<()>` serializing all write operations including sequence allocation.
- Process lock ensures single writer per journal path.

## Policy Boundary
- Relaxed: no SyncAll, no gate validation, but still gets sequence.
- Journaled: gate validation, no SyncAll.
- Strict: gate validation + SyncAll + readback.

## Call Site Boundaries
- **vb_cli/src/run.rs**: `submit_cli_compiled_artifact()` → `submit_artifact()`.
- **vb_runtime/src/admission/**: `admit_artifact_run()` loads stored artifact, checks seq freshness.
- **Tests**: Various integration and unit tests exercise all paths.
