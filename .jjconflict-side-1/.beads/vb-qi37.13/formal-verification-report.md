# Formal Verification Report

STATUS: APPROVED

## Scope / startup

- Workspace used: `/home/lewis/src/vb-qi37-13-r2` only.
- Forbidden workspaces not used: `/home/lewis/src/vb-qi37-13`, `/home/lewis/src/Velvet-ballistics`.
- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md`; cited rules: lines 21-24 require approved plan, every obligation accounted, scope-before-status, and fail-closed missing tools.
- Read `/home/lewis/.agents/skills/formal-verifier/SKILL.md`; it is identical for the cited rules and wins on conflict. Lines 100-114 require exact obligation commands, recorded exit status/output, no all-target clippy style gate, and no silent waivers.

## Inputs

- `.beads/vb-qi37.13/proof-obligations.jsonl`: present, 9 JSONL rows, parsed by `jq`.
- `.beads/vb-qi37.13/traceability-matrix.jsonl`: present, 33 JSONL rows, parsed by `jq`.
- `.beads/vb-qi37.13/delivery-scope.jsonl`: present, parsed by `jq`.
- `.beads/vb-qi37.13/baseline-report.md`: present; isolated workspace is `/home/lewis/src/vb-qi37-13-r2`, forbidden partial workspace is `/home/lewis/src/vb-qi37-13`.
- `.beads/vb-qi37.13/tla-spec.md` and `.beads/vb-qi37.13/lean-contract.md`: present; non-applicability already approved upstream.
- `.beads/vb-qi37.13/contract-verification-review.md`: `STATUS: APPROVED` observed by mandatory gate.

Mandatory gate command completed successfully in `/home/lewis/src/vb-qi37-13-r2`:

```bash
test -s .beads/vb-qi37.13/proof-obligations.jsonl && test -s .beads/vb-qi37.13/traceability-matrix.jsonl && test -s .beads/vb-qi37.13/delivery-scope.jsonl && test -s .beads/vb-qi37.13/baseline-report.md && test -s .beads/vb-qi37.13/tla-spec.md && test -s .beads/vb-qi37.13/lean-contract.md && test -s .beads/vb-qi37.13/contract-verification-review.md && rg -n '^STATUS: APPROVED$' .beads/vb-qi37.13/contract-verification-review.md && jq -c . .beads/vb-qi37.13/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.13/traceability-matrix.jsonl >/dev/null && jq -c . .beads/vb-qi37.13/delivery-scope.jsonl >/dev/null
```

Observed output: `3:STATUS: APPROVED`.

## Tool / command evidence

- `jq`: available; parsed proof obligations, traceability matrix, and delivery scope.
- `rg`: available; no-public-code-9 scan produced no matches and exit status 1 as expected.
- `verus`: available; `verification results:: 4 verified, 0 errors`.
- `cargo test`: available; all scoped unit/integration/postcard tests passed.
- `cargo fuzz`: available; GNU `vb_ui_model_postcard_decode` target built and ran with `-runs=1`.
- `cargo clippy`: available; touched CLI package and fuzz target clippy gates passed.
- `cargo fmt` / `rustfmt`: available; focused fmt gates passed.

## Obligation Results

All 9 rows from `.beads/vb-qi37.13/proof-obligations.jsonl` are accounted in `.beads/vb-qi37.13/verification-ledger.jsonl`.

| id | result | classification | evidence summary |
|---|---:|---:|---|
| `VERUS-EXIT-001` | PASS | PASS | `verus verification/verus/diagnostic_envelope_verus.rs` -> exit 0, `4 verified, 0 errors`. |
| `TEST-EXIT-001` | PASS | PASS | exit-code filtered cargo test -> exit 0; public matrix test observes keys `0..8` and no `9`. |
| `STATIC-EXIT-001` | PASS | PASS | exact `rg` scan -> exit 1/no output; expected no-match result. |
| `TEST-DIAGNOSTICS-001` | PASS | PASS | parse diagnostic unit test -> exit 0. |
| `TEST-STRUCTURED-001` | PASS | PASS | format parity test -> exit 0; additional structured reconciliation target passed 11/11. |
| `TEST-POSTCARD-001` | PASS | PASS | `vb_ui_model` postcard tests -> 12 passed. |
| `FUZZ-POSTCARD-001` | PASS | PASS | GNU cargo-fuzz target ran with `-runs=1` -> exit 0. |
| `RECON-CHILD-001` | PASS | PASS | child evidence marker Python check -> exit 0/no output. |
| `MATRIX-COMMAND-001` | PASS | PASS | command matrix Python check -> exit 0/no output. |

Counts: PASS 9, FAIL_LOCAL 0, FAIL_REGRESSION 0, WAIVED 0, DEFERRED_GLOBAL 0.

## Additional State 11 coverage gates

These are not added as ledger obligations; they are explicit rerun coverage evidence requested for State 11 after black-hat repair.

- Structured diagnostic JSON/JSONL and stdout/stderr separation:
  - Command: `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics --test vb_qi37_13_structured_reconciliation --all-features`
  - Result: PASS, 11 passed.
  - Covered parse routes: unknown command JSON, unknown command JSONL one-line stderr, unsupported emit mode JSON, unsupported status emit mode JSON.
  - Covered non-parse routes: missing-file validate JSON, malformed YAML validate JSONL, missing-file compile JSON, runtime input decode JSON, storage open JSON.
  - Covered stdout/stderr separation: success payloads to stdout only; failure diagnostics to stderr only.
- Public exits `0..=8` / no `9`:
  - Covered by `VERUS-EXIT-001`, `TEST-EXIT-001`, `STATIC-EXIT-001`, and structured matrix test `cli_public_exit_code_matrix_is_exactly_zero_through_eight_in_agent_context`.
- Contracted `cli_postcard::decode_postcard` / `vb_ui_model` postcard validation:
  - Command: `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard`
  - Result: PASS, 12 passed.
  - Covered CRC, digest, old/future version, wrong kind, payload bound before exposure, empty input, truncated/header mismatch, bad magic, and roundtrip.
- Verus diagnostic:
  - Command: `verus verification/verus/diagnostic_envelope_verus.rs`
  - Result: PASS, 4 verified, 0 errors.
- GNU fuzz target:
  - Command: `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1`
  - Result: PASS.
- Command matrix / child reconciliation:
  - `RECON-CHILD-001` and `MATRIX-COMMAND-001` exact commands both passed.
- Clippy/fmt:
  - `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo clippy -p velvet_ballistics --lib --bin velvet-ballistics --all-features -- -D warnings` -> PASS.
  - `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo fmt --check -p velvet_ballistics && rustfmt --edition 2024 --check crates/velvet_ballistics/src/main.rs` -> PASS.
  - `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo clippy --manifest-path fuzz/Cargo.toml --features fuzz --lib --bin vb_ui_model_postcard_decode -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` -> PASS.
  - `rustfmt --edition 2024 --check crates/velvet_ballistics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs fuzz/src/lib.rs fuzz/src/bin/vb_ui_model_postcard_decode.rs` -> PASS.

## Waivers

No waiver was used to discharge any required State 11 obligation. TLA+/Lean non-applicability remains upstream-approved scope rationale only.

## Residual Risk

No State 11 blocker remains. The ledger does not claim full workspace release refresh; required local/regression rows and requested State 11 rerun gates passed.
