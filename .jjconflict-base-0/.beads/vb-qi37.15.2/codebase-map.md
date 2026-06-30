bead_id: vb-qi37.15.2
bead_title: cli: Add submit command and job ledger
phase: State 2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

- CLI parser: `crates/velvet_ballistics/src/args.rs` already has `Command::Submit { workflow, input_bin, db, durability, output }`.
- CLI dispatcher: `crates/velvet_ballistics/src/main.rs` calls `cmd_submit(&workflow, &input_bin, &db, durability, output)`.
- Current `cmd_submit` reads input and workflow, compiles workflow, opens `vb_storage::FjallJournal`, writes workflow source, writes `RunHeaderRecord`, appends `RuntimeJournalEvent::RunSubmitted` for durable modes, and prints submitted identifiers.
- Existing runtime/journal commands for later inspection: `inspect`, `events`, `trace`, `doctor` in `main.rs` and `commands_journal`/storage APIs.
- Existing black-box test home: `crates/velvet_ballistics/tests/cli_integration.rs`.
- Important acceptance gap to lock: metadata must be persisted before success is reported; bad DB/profile/sink inputs must fail non-interactively; returned IDs must be structured and support later inspection.
