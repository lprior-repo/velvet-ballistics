bead_id: vb-qi37.4
bead_title: runtime: Accepted artifact admission and durability
phase: State 2 - codebase map
updated_at: 2026-05-15T19:48:24Z

# Codebase Map

- `crates/vb_storage/src/admission.rs`: builds and persists the `AcceptedArtifact` envelope through `submit_artifact` and `submit_artifact_with_contracts`; records digest, postcard IR bytes, verification proof, accepted sequence, required capabilities, and strict durability via `persist_strict()`.
- `crates/vb_runtime/src/admission.rs`: runtime admission gate. `AcceptedArtifactStore` loads stored envelopes, `StorageArtifactStore` reads `compiled_ir`, `admit_artifact_run` validates gate count/proof flags/capabilities before run creation, and maps malformed/missing envelopes into typed `AdmissionError` values.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs`: submit lifecycle calls `build_admission` before allocating/inserting `RunState`; appends `RunSubmitted` then `RunAdmission`; maps admission failures to `RuntimeError` without creating a live run.
- `crates/vb_runtime/src/journal/chunk_001.rs` and `crates/vb_runtime/src/journal/chunk_002.rs`: define `RuntimeJournalEvent::RunSubmitted`/`RunAdmission` and convert them to storage `JournalEvent` values; strict storage adapter uses `FjallJournal::append_strict`.
- `crates/vb_storage/src/events.rs`: durable `JournalEvent::RunAdmission` carries run id, sequence, artifact digest, granted capabilities, and runtime policy.
- `crates/vb_storage/src/records.rs`: `CompiledIrRecord` stores accepted artifact bytes by digest; `RunHeaderRecord` binds run id, workflow id, compiled digest, status, and accepted timestamp.
- `crates/vb_storage/src/journal/append.rs`: durability boundary; `append_strict` appends then calls `persist_strict`, which uses `fjall::PersistMode::SyncAll`.
- `crates/vb_runtime/src/error/mod.rs`, `display.rs`, `diagnostics.rs`, `equality.rs`, and `conversions.rs`: typed runtime admission/durability errors and stable diagnostic/runtime-code mapping.
- `crates/vb_runtime/src/recovery.rs`: hydrates `RunAdmission` from durable storage events for recovery-facing runtime metadata.
- `crates/velvet_ballistics/tests/admission_evidence_integration.rs` and included chunks: cross-crate evidence for submit-artifact-then-run, relaxed policy behavior, storage failure before header acknowledgement, restart header lookup, and capability rejection.
- `crates/vb_storage/tests/accepted_artifact_red_phase.rs`: storage-level accepted artifact tests for envelope proof fields, gate count, postcard encoding, round trip, required capabilities, digest binding, and raw workflow rejection.
- `crates/velvet_ballistics/tests/admission_durability_code.rs`: API envelope preservation of admission durability diagnostic codes.

## Scope Notes

- Parent bead `vb-qi37.4` is the aggregate admission/durability stream. Closed child slices already cover envelope definition (`vb-qi37.4.1`), header-before-ack (`vb-qi37.4.3`), error taxonomy (`vb-qi37.4.4`), and evidence tests (`vb-qi37.4.5`); open child `vb-qi37.4.2` remains the primary run-creation gate enforcement surface.
- State 2 is artifact repair only. No production code, tests, proofs, or source checkout files were modified.

## Command Evidence

- `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.4 --json` exited 0 and showed `vb-qi37.4` depends on child admission/durability beads, including open `vb-qi37.4.2` and closed `vb-qi37.4.1/.3/.4/.5`.
- `grep`/`read` in the isolated workspace found `REQUIRED_GATE_COUNT`, `AcceptedArtifactStore`, `StorageArtifactStore`, `admit_artifact_run`, `RunAdmission`, `RuntimeError::AdmissionArtifact*`, `JournalEvent::RunAdmission`, and strict `PersistMode::SyncAll` durability boundaries in the files listed above.
