# Test Writer Report: vb-m5gp

## Scope

- State: 8 test repair only.
- Repair attempt: 3 after State 9 rejection.
- Production behavior edited: no.
- Test/gate artifacts edited:
  - `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs`
  - `scripts/check-source-length.sh`

## Repair 3 Change

- Added `vb_compile_production_sources_remain_under_agreed_line_limit` to enumerate every top-level `crates/vb_compile/src/*.rs` production source and reject any file with `>=300` physical lines.
- Retained prior split ownership checks: required private module declarations, `lib.rs <300`, non-doc-only split modules, no `include!`, and no returned `compile_core_impl.rs` hidden body.
- Strengthened `scripts/check-source-length.sh` with the same top-level `crates/vb_compile/src/*.rs` physical line limit so oversized split modules cannot pass the shell gate.

## Command Evidence

- `cargo +nightly fmt --all --check`: PASS.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract --no-run`: PASS.
- `cargo +nightly test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract`: expected RED; 6 passed, 1 failed.
  - Failing test: `vb_compile_production_sources_remain_under_agreed_line_limit`.
  - Oversized top-level sources: `expression.rs=881`, `expression_bytecode.rs=2242`, `mod_compile_errors.rs=848`, `mod_compile_lowering.rs=2539`, `mod_compile_validation.rs=1447`, `references.rs=342`, `schema.rs=729`, `type_taint.rs=511`.
- `bash scripts/check-source-length.sh`: expected RED with the same top-level `crates/vb_compile/src/*.rs` oversized production files.

## Red Status

Red is intentional and contract-aligned. The implementation has split the facade, but several top-level production files, including three `mod_compile_*` split modules, still exceed the approved `<300` line threshold. State 10 must decompose these modules further or provide an approved bead-linked waiver/follow-up before this gate can turn green.

## Follow-up For Test Review

- Verify the State 9 rejection blocker is closed: oversized split modules now fail both the Rust split contract test and `scripts/check-source-length.sh`.
- Do not weaken the `<300` threshold without contract-review evidence.
