bead_id: vb-qi37.16.5
bead_title: cli/runtime: Add lifecycle integration evidence
phase: state-2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

Exploration evidence:
- Lifecycle command surfaces are declared in `crates/velvet_ballastics/src/args.rs`: cancel, resume, retry, answer.
- CLI command implementations and storage-backed runtime operations are under `crates/velvet_ballastics/src/storage.rs` and related command modules.
- Runtime lifecycle transitions are under `crates/vb_runtime/src/shard/lifecycle.rs` and `types.rs`.
- Durable evidence/recovery spans `crates/vb_runtime/src/journal.rs`, `crates/vb_storage/src/journal.rs`, and recovery/replay modules.
- Integration tests belong under `crates/velvet_ballastics/tests/cli_integration.rs` or a focused lifecycle integration test file.
