bead_id: vb-qi37.16.4
bead_title: cli/runtime: Implement durable answer command
phase: state-2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

Exploration evidence:
- `crates/velvet_ballastics/src/args.rs` declares `Command::Answer { run_id, step, value_file, db, output }`.
- `crates/vb_runtime/src/shard/types.rs` declares `AskTicket` and `AskAnswer`.
- `crates/vb_runtime/src/shard/lifecycle.rs` has `handle_ask_answer`, journal `AskAnswered`, and `SlotWritten` evidence.
- Runtime trace events include ask-answer diagnostics under `crates/vb_runtime/src/trace.rs`.
- CLI/runtime/storage answer routing likely belongs under `crates/velvet_ballastics/src/storage.rs` with integration coverage under `crates/velvet_ballastics/tests/cli_integration.rs`.
