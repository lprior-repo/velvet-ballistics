# vb-qi37.6 Contract Repair Report

STATUS: REPAIRED

## Startup citations

- Read `/home/lewis/.claude/skills/rust-contract/SKILL.md`: lines 12-26 require contract-first verification planning, TLA+ for temporal behavior, Verus-first Rust core obligations, machine-readable proof obligations, review, and no implementation/proof/test code.
- Read `/home/lewis/.agents/skills/rust-contract/SKILL.md`: same version 2.6.0 rules; per instruction, `.agents` wins if conflicts exist. No conflict found.

## Scope

- Workspace used: `/home/lewis/src/vb-qi37-6` only.
- Forbidden checkout `/home/lewis/src/Velvet-ballistics` was not used.
- Production source, tests, TLA model code, Verus code, Kani harnesses, and fuzz harnesses were not edited.

## Repairs made

- Repaired State 6 contract-verification rejection by replacing non-executable `BLOCKED_SETUP owner_state 8: ...` placeholder commands in canonical `.beads/vb-qi37.6/proof-obligations.jsonl` with executable State 8 setup-check commands.
- Repaired exact rows:
  - `PRE-003-FUZZ-SCHEMA`: command now checks `fuzz/Cargo.toml` exists and contains `capability_name_schema` and `capability_contract_schema` bin registrations; State 11 fuzz runs remain in `after_setup_commands`.
  - `INV-001-KANI-EXACT-SETUP`: command now checks `crates/vb_core/src/kani.rs` or `crates/vb_core/src/kani/mod.rs` exists; State 11 Kani harness execution remains in `after_setup_commands`.
  - `INV-002-KANI-CARDINALITY-SETUP`: command now checks upstream `vb_core` Kani module wiring exists; State 11 runtime Kani harness execution remains in `after_setup_commands`.
- Rebuilt canonical `.beads/vb-qi37.6/proof-obligations.jsonl` with mandatory `layer` and `checker` fields on every row.
- Synced `.beads/vb-qi37.6/proof-obligations.planned.jsonl` to the repaired canonical obligation ledger so downstream readers cannot consume stale Kani/fuzz routing.
- Added required TLA scope fields on every `layer: "tla-plus"` row: `tla_module`, `model`, `config`, `variables`, `actions`, `invariants`, `temporal_properties`, `fairness`, `state_constraints`, and `refinement`.
- Rebuilt `.beads/vb-qi37.6/traceability-matrix.jsonl` to include every `PRE-*`, `POST-*`, and `INV-*` clause from `contract.md`.
- Preserved State 8 setup / State 11 execution routing for Kani and fuzz:
  - `INV-001-KANI-EXACT-SETUP`: owner_state 8 setup, after_setup_owner_state 11 execution.
  - `INV-002-KANI-CARDINALITY-SETUP`: owner_state 8 setup, after_setup_owner_state 11 execution.
  - `PRE-003-FUZZ-SCHEMA`: owner_state 8 setup, after_setup_owner_state 11 execution.

## Coverage summary

- Preconditions covered: PRE-001, PRE-002, PRE-003, PRE-004, PRE-005, PRE-006.
- Postconditions covered: POST-001, POST-002, POST-003, POST-004, POST-005, POST-006, POST-007, POST-008, POST-009.
- Invariants covered: INV-001, INV-002, INV-003, INV-004, INV-005, INV-006, INV-007, INV-008.
- Release gate retained: `GAUNTLET-010`.

## No-PASS guarantee

- All obligation rows use `"status":"planned"`.
- No canonical row records verifier success.
- Kani/fuzz rows are setup-routed blockers until State 8 and execution-routed to State 11.

## Notes for State 11 verifier

- TLA rows reference existing `verification/tla/CapabilityLifecycle.tla` focused cfgs and retain safety-only expected evidence; no temporal/liveness success is claimed here.
- Kani commands are exact after setup, but setup is blocked by missing `vb_core` Kani module wiring.
- Fuzz commands are exact after setup, but setup is blocked by missing fuzz bin registrations under `fuzz/Cargo.toml` with `autobins = false`.
