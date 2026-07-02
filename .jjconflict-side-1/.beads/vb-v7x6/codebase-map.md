bead_id: vb-v7x6
phase: 2
attempt: 1-of-7

# Codebase Map

- Release gate test: `xtask/tests/ui_release_gates.rs`.
- AI release command path: `xtask/src/main.rs`, `xtask/src/ai_profile.rs`, `xtask/src/evidence/*`.
- UI release contract/gate definitions: `xtask/src/evidence/release_contract.rs`, `release_validation.rs`, `release_validators.rs`.
- Dependent machine gate: `.moon/tasks/all.yml` `doc -> test -> check`.
- Incidental check blocker: `crates/vb_storage/src/recovery/recovery_unit_tests.rs` unused import/helper cleanup plus rustfmt-only files.
