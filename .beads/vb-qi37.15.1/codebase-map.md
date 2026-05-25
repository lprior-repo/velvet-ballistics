bead_id: vb-qi37.15.1
bead_title: cli: Add simulate command
phase: State 2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

- CLI parser: `crates/velvet_ballistics/src/args.rs` already has `Command::Simulate { workflow, output }`.
- CLI dispatcher: `crates/velvet_ballistics/src/main.rs` calls `cmd_simulate(&workflow, output)`.
- Current `cmd_simulate` compiles workflow bytes, calls `commands_workflow::simulate_workflow`, prints text or JSON/JSONL; no durable DB path is accepted.
- Pure workflow dry-run logic: `crates/velvet_ballistics/src/commands_workflow.rs::simulate_workflow` enumerates compiled nodes and returns step descriptions plus counts for actions and branches.
- Existing black-box test home: `crates/velvet_ballistics/tests/cli_integration.rs` with helpers for temp workflows and CLI subprocess execution.
- Important acceptance gap to lock: dry-run must not create durable external effects; invalid artifacts must fail with diagnostics; structured output must be deterministic and bounded.
