bead_id: vb-qi37.16.2
bead_title: cli/runtime: Implement durable resume transition
phase: state-2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

Exploration evidence:
- `crates/velvet_ballistics/src/args.rs` declares `Command::Resume { run_id, db, output }` and command parsing surface.
- `crates/vb_runtime/src/shard/types.rs` declares `ShardCommand::Resume { run }`.
- `crates/vb_runtime/src/shard/lifecycle.rs` currently implements `handle_resume` as `self.drive_run(run)`.
- The runtime journal evidence path is in `crates/vb_runtime/src/journal.rs` and `RuntimeJournalEvent` usage in lifecycle handlers.
- CLI storage/runtime lifecycle command routing is expected under `crates/velvet_ballistics/src/storage.rs` plus integration tests under `crates/velvet_ballistics/tests/cli_integration.rs`.
