# Regression Diff — State 11 rerun

STATUS: PASS

Workspace: `/home/lewis/src/vb-qi37-13-r2` only. Broken `/home/lewis/src/vb-qi37-13` and source checkout `/home/lewis/src/Velvet-ballistics` were not used.

## Result

- Required local/regression obligations: 9 PASS, 0 FAIL_LOCAL, 0 FAIL_REGRESSION, 0 WAIVED, 0 DEFERRED_GLOBAL.
- Additional requested State 11 gates: PASS.
- No new regression observed in scoped gates.
- Coverage overclaim correction: ledger rows remain limited to approved `proof-obligations.jsonl`; extra diagnostic/clippy/fmt gates are reported separately as State 11 rerun evidence, not as proof-obligation rows.

## Required ledger commands

- `verus verification/verus/diagnostic_envelope_verus.rs` -> PASS, `4 verified, 0 errors`.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics exit_code --all-features` -> PASS.
- `rg -n "DomainError\s*=\s*9|ExitCode::from\(9u8\)|0_to_9|<= 9" crates/velvet_ballastics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs` -> PASS as no-match scan, exit 1/no output.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics parse_error_unknown_command_exit_code_is_1 --all-features` -> PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics bdd_format_parity_exit_code_identical_across_formats --all-features` -> PASS.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard` -> PASS, 12/12.
- `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1` -> PASS.
- `RECON-CHILD-001` exact Python command -> PASS, no output.
- `MATRIX-COMMAND-001` exact Python command -> PASS, no output.

## Additional machine gates

- `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballastics --test vb_qi37_13_structured_reconciliation --all-features` -> PASS, 11 tests passed.
- `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo clippy -p velvet_ballastics --lib --bin velvet-ballastics --all-features -- -D warnings` -> PASS.
- `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo fmt --check -p velvet_ballastics && rustfmt --edition 2024 --check crates/velvet_ballastics/src/main.rs` -> PASS.
- `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo clippy --manifest-path fuzz/Cargo.toml --features fuzz --lib --bin vb_ui_model_postcard_decode -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` -> PASS.
- `rustfmt --edition 2024 --check crates/velvet_ballastics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs fuzz/src/lib.rs fuzz/src/bin/vb_ui_model_postcard_decode.rs` -> PASS.

## Blockers

None.
