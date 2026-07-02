bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 2
updated_at: 2026-05-14T22:15:30Z
attempt: 1-of-7

# Codebase Map

## Scope summary

Target behavior: stable CLI/operator structured output parent reconciliation for `--emit text|yaml|postcard`, `schema_version`, `kind`, stable diagnostics, and public exit codes exactly `0..=8`.

Replacement isolated workspace: `/home/lewis/src/vb-qi37-13-r2`.

## Located production files

- `crates/velvet_ballistics/src/args.rs`
  - `OutputFormat` currently has `Text`, `Json`, and `Jsonl` (lines 7-17); master behavior mentions text/yaml/postcard.
  - `EmitTarget` currently has `Ir`, `Rust`, `Yaml`, `Postcard` (lines 203-209) for compile artifact emission.
  - Many command variants carry `output: OutputFormat`; scope includes validate/verify/compile/run/simulate/trace/diff/explain/status and related operator commands.
- `crates/velvet_ballistics/src/exit_code.rs`
  - `CliExitCode` currently includes ten variants: `Success = 0` through `ReplayDivergence = 8`, plus `DomainError = 9`.
  - Existing unit tests assert `DomainError as u8 == 9` and include ten variants. This contradicts the required public range `0..=8`.
- `crates/velvet_ballistics/src/main.rs`
  - Help text still advertises many `[--json|--jsonl]` surfaces while compile advertises `--emit <ir|rust|yaml|postcard>`.
  - Compile path handles `EmitTarget::{Ir,Rust,Yaml,Postcard}` around lines 913-1110.
  - Error exits use `CliExitCode::{ValidationFailed,CompileFailed,RuntimeFailed,StorageError,...}` throughout command paths.
- `crates/velvet_ballistics/src/cli_postcard.rs`
  - Defines `CLI_MAGIC`, `MAX_PAYLOAD`, `HEADER_SIZE`, `PostcardHeader`, `PostcardError`, `decode_postcard`, and `encode_postcard`.
  - Header validation covers magic, header length, and bounded payload length before payload slicing; digest field uses `DefaultHasher`-derived bytes rather than cryptographic digest despite comments saying SHA-256.

## Located test files

- `crates/velvet_ballistics/src/mode_activation_tests.rs`
  - Contains `cli_exit_code_all_9_variants_distinct`, suggesting some test coverage already excludes `DomainError`, but `exit_code.rs` unit tests still include it.
- `crates/velvet_ballistics/src/main_tests.rs`
  - Contains parse tests for unknown emit target and runtime input postcard decode paths.
- `crates/velvet_ballistics/tests/*`
  - Integration tests include mode activation and structured CLI surfaces; only one direct `CliExitCode` grep hit under `tests/` was in `mode_activation_integration_tests.rs`.

## Located proof/formal artifacts

- `verification/verus/diagnostic_envelope_verus.rs`
  - Mirrors `CliExitCode` with `DomainError` and proves range `0..=9`, not `0..=8`.
  - Proof writer must repair this to prove the required public range exactly `0..=8` or produce a valid waiver for removed/internal-only variants.
- No active proof approval has been verified in this replacement workspace; downstream states must rebuild or re-approve artifacts here.

## Located fuzz/tooling artifacts

- `fuzz/Cargo.toml`
  - Registers many `[[bin]]` fuzz targets but no `vb_ui_model_postcard_decode` bin was found.
- `fuzz/fuzz_targets.rs`
  - Callable harness module includes targets such as `compiled_ir`, `expr_eval`, `slot_value_roundtrip`, and `admission_fuzz`; no `vb_ui_model_postcard_decode` function was found.
- `fuzz/src/lib.rs`
  - Contains existing postcard decode harness bodies for storage/events/core workflow parts and slot values; a UI model postcard decode route may be missing or named differently.

## Risk tags

- `public-api`: exit-code values are an operator-facing compatibility contract.
- `codec`: postcard output/decode route and fuzz coverage are in scope.
- `parser`: CLI argument format/output format reconciliation can alter error behavior.
- `user-visible-behavior`: text/yaml/postcard output and diagnostics are external CLI behavior.
- `formal`: Verus proof currently proves wrong range (`0..=9`).
- `release-critical`: parent release-plan bead and engine acceptance dependents consume this behavior.

## Downstream recommendations

- State 3 contract must specify whether `DomainError` is removed, mapped to an existing code, or made internal without public exit status `9`.
- State 4 proof plan must include Verus range proof `0..=8` and an integrated postcard proof route: Kani/proptest/fuzz or explicit approved waiver.
- State 5 proof writer must repair `verification/verus/diagnostic_envelope_verus.rs` and either register/fix `vb_ui_model_postcard_decode` fuzz target or supply an approved waiver path.
- State 10 implementation owner must repair `crates/velvet_ballistics/src/exit_code.rs` and tests only after proof/test gates approve.

## Open questions

- UNKNOWN: Whether `DomainError` is used by any current production path; no focused production use was found in the first `crates/velvet_ballistics/src` grep output beyond its definition/tests, but a full exact-symbol count should be part of State 3/10.
- UNKNOWN: The canonical command for Verus in this repository; State 4/5/11 must identify executable verifier command.
- UNKNOWN: Whether `vb_ui_model_postcard_decode` existed in the damaged partial workspace; this replacement workspace does not contain it by glob/grep.
