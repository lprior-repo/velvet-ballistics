bead_id: vb-qi37.4.3
bead_title: runtime/storage: Persist run header before acknowledgement
phase: State 2 - codebase map
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

- `crates/vb_runtime/src/shard/lifecycle.rs`: `Shard::handle_submit_with_inputs` performs duplicate/capacity checks, builds admission, appends `RunSubmitted`, appends `RunAdmission`, inserts active `RunState`, then drives the run.
- `crates/vb_runtime/src/journal.rs`: runtime journal event mapping to storage journal events; durability profile adapters live here.
- `crates/vb_runtime/src/lib.rs`: `RuntimeError` taxonomy and stable diagnostic/runtime codes.
- `crates/vb_runtime/src/recovery.rs`: runtime-facing hydration for admission metadata from durable storage events.
- `crates/vb_storage/src/**`: journal event persistence/recovery and accepted-artifact evidence storage.
- `crates/velvet_ballistics/tests/admission_evidence_integration.rs`: cross-crate admission evidence scenarios.

State 2 command evidence: grep/read found `handle_submit_with_inputs` lines 96-131 append admission events before state insertion/ack, and `RuntimeError` admission variants in `vb_runtime/src/lib.rs`.
