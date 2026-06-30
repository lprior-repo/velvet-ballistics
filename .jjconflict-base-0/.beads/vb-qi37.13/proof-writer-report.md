bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 5
updated_at: 2026-05-14T23:06:53Z
attempt: 3-of-7

# Proof Writer Report

STATUS: REPAIRED

## Scope

- Worktree used: `/home/lewis/src/vb-qi37-13-r2` only.
- Forbidden source checkouts were not used.
- Production source and proof plan files were not edited.
- Updated artifacts: `.beads/vb-qi37.13/proof-evidence.md` and `.beads/vb-qi37.13/proof-writer-report.md`.

## Obligation Accounting

- VERUS-EXIT-001: PASS, fresh `verus verification/verus/diagnostic_envelope_verus.rs` evidence recorded.
- TEST-EXIT-001: PASS, fresh `cargo test -p velvet_ballistics exit_code --all-features` evidence recorded.
- STATIC-EXIT-001: PASS, fresh no-match static scan evidence recorded.
- TEST-DIAGNOSTICS-001: PASS, fresh `parse_error_unknown_command_exit_code_is_1` evidence recorded.
- TEST-STRUCTURED-001: PASS, fresh `bdd_format_parity_exit_code_identical_across_formats` evidence recorded.
- TEST-POSTCARD-001: PASS, fresh postcard cargo-test evidence recorded.
- FUZZ-POSTCARD-001: PASS, fresh stdin harness and explicit GNU cargo-fuzz evidence recorded.
- RECON-CHILD-001: PASS, child evidence marker validation exited 0.
- MATRIX-COMMAND-001: PASS, command matrix validation exited 0.

## Ledger Validation

- `.beads/vb-qi37.13/proof-obligations.jsonl`: 9 IDs.
- `.beads/vb-qi37.13/proof-obligations.planned.jsonl`: 9 IDs.
- Exact matching ID set and order: `VERUS-EXIT-001`, `TEST-EXIT-001`, `STATIC-EXIT-001`, `TEST-DIAGNOSTICS-001`, `TEST-STRUCTURED-001`, `TEST-POSTCARD-001`, `FUZZ-POSTCARD-001`, `RECON-CHILD-001`, `MATRIX-COMMAND-001`.

## Commands Run

- `python3 -c "...ledger id equality..."`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics exit_code --all-features`: PASS.
- `verus verification/verus/diagnostic_envelope_verus.rs`: PASS, `verification results:: 4 verified, 0 errors`.
- `rg -n "DomainError\s*=\s*9|ExitCode::from\(9u8\)|0_to_9|<= 9" crates/velvet_ballistics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs`: PASS by expected no-match exit status 1.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics parse_error_unknown_command_exit_code_is_1 --all-features`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics bdd_format_parity_exit_code_identical_across_formats --all-features`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard`: PASS, 8 postcard tests passed.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo run --manifest-path fuzz/Cargo.toml --features fuzz --bin vb_ui_model_postcard_decode -- < /dev/null`: PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1`: PASS.
- `python3 -c "...child evidence marker check..."`: PASS.
- `python3 -c "...command matrix check..."`: PASS.

## Decision

- State 5 evidence is aligned to the repaired State 3/4 ledgers.
- Every current planned obligation is accounted by ID.
- No current State 5 planned obligation is blocked.
- `STATUS: REPAIRED`.
