bead_id: vb-qi37.16.3
bead_title: cli/runtime: Implement durable retry transition
phase: state-2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

Exploration evidence:
- `crates/velvet_ballistics/src/args.rs` declares `Command::Retry { run_id, db, output }`.
- `crates/vb_runtime/src/shard/lifecycle.rs` contains retry helpers: `retry_is_available`, `ticket_with_retry_capacity`, `apply_action_failure_to_state`.
- Retry metadata helpers are referenced from `crates/vb_runtime/src/shard/helpers.rs` and exported in `crates/vb_runtime/src/shard/mod.rs`.
- Journal evidence is emitted through `SharedRuntimeJournal` and `RuntimeJournalEvent`.
- CLI/runtime integration evidence belongs in `crates/velvet_ballistics/tests/cli_integration.rs`.
