bead_id: vb-qi37.16.5
phase: state-8
classification: PASS_AFTER_REPAIR

# State 8 Regression Classification

## Initial failure

- Category: `FORMAT`
- Classification: `BLOCK_LOCAL`
- Owner state: 8 repair / holzman-rust formatting fallout
- Rerun from: 8 after repair

## Repair result

`state-8-format-repair.md` records successful rerun evidence:

- `rtk cargo fmt -- --check`: PASS
- `rtk cargo test --package velvet_ballistics --test lifecycle_integration -- --test-threads=1`: `43 passed`
- `moon run :quick`: PASS
- `moon run :test`: `9894 passed`

## Decision

State 8 is unblocked after targeted repair. `vb-qi37.16.5` may advance to State 9.

## State 15 preflight regression diff — 2026-05-12

Initial landing preflight blockers after State 14:

- `fmt`: `crates/velvet_ballistics/tests/lifecycle_integration.rs` drift.
- `doc-test`: `crates/vb_storage/src/journal.rs` `FjallJournal::inject_seq_gap` example missed `journal` and `run` setup.
- `lint-src`: `vb_proof_kernels/src/envelope_header.rs` missing `Default` on old base; after rebase, main already supplied `Default` and local duplicate was removed.

Additional rebase-exposed local blockers repaired in this workspace:

- Rebase conflicts in `Cargo.lock`, `vb_core::errors`, `velvet_ballistics::lib`, `fuzz decode_record`, and `xtask main`.
- `JournalEvent::attempt` missing lifecycle variants.
- `RunCancelled` construction missing `attempt` and `reason` after main schema changes.
- Persistent `/tmp/velvet_test_readonly_journal` test pollution replaced with a per-test tempdir.
- Source lint forbade ignored fallible writes and panic-prone test-helper locking in production lib target.

Classification: `PASS_AFTER_REPAIR`, `BLOCK_LOCAL` cleared. No remaining preflight blockers observed under `moon ci`.
