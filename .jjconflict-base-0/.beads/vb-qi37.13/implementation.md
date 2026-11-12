# vb-qi37.13 implementation evidence

## Status

STATUS: PASS_WITH_WAIVER_CANDIDATE

Public CLI exit code 9 was removed from `CliExitCode`. The Verus diagnostic model now proves only public variants in `0..=8`. The postcard fuzz harness builds and runs with `--target x86_64-unknown-linux-gnu`; cargo-fuzz default `x86_64-unknown-linux-musl` remains blocked by sanitizer/static-libc incompatibility.

## Files changed

- `crates/velvet_ballistics/src/exit_code.rs`
  - Removed `CliExitCode::DomainError = 9`.
  - Updated exit-code tests to cover exactly the nine public variants `0..=8`.
- `verification/verus/diagnostic_envelope_verus.rs`
  - Removed `DomainError` from the Verus public exit-code model.
  - Replaced `0..=9` range proof with `0..=8` proof.
- `fuzz/src/lib.rs`
  - Removed a useless unsigned lower-bound assertion that produced fuzz build warnings.
  - Replaced `unwrap()` and unchecked subtraction in the step-budget fuzz helper with explicit `match` and `checked_sub` so strict clippy on the fuzz package passes.

## Command evidence

All commands used `TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER=` unless noted.

PASS:

```text
rustfmt --check crates/velvet_ballistics/src/exit_code.rs verification/verus/diagnostic_envelope_verus.rs fuzz/src/lib.rs fuzz/src/bin/vb_ui_model_postcard_decode.rs
```

PASS:

```text
cargo test -p velvet_ballistics exit_code --all-features
```

Observed final targeted count: 9 exit-code tests passed plus related filtered integration checks passed.

PASS:

```text
verus verification/verus/diagnostic_envelope_verus.rs
```

Observed: `verification results:: 4 verified, 0 errors`.

PASS:

```text
cargo test -p vb_ui_model --all-features postcard
```

Observed: 8 postcard tests passed.

PASS:

```text
cargo run --manifest-path fuzz/Cargo.toml --features fuzz --bin vb_ui_model_postcard_decode -- < /dev/null
```

PASS with explicit target override:

```text
cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

Observed: fuzz binary built and launched with no sanitizer/static-libc error.

PASS:

```text
cargo clippy -p velvet_ballistics --lib --bin velvet-ballistics --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
```

PASS:

```text
cargo clippy --manifest-path fuzz/Cargo.toml --features fuzz --lib --bin vb_ui_model_postcard_decode -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
```

BLOCKER / waiver candidate:

```text
cargo fuzz run vb_ui_model_postcard_decode -- -runs=1
```

Observed blocker:

```text
error: sanitizer is incompatible with statically linked libc, disable it using `-C target-feature=-crt-static`
Error: failed to build fuzz script ... --target x86_64-unknown-linux-musl ... -Zsanitizer=address ...
```

Kani attempted:

```text
cargo kani -p vb_ui_model
```

Observed blocker outside this bead's touched files:

```text
error[E0583]: file not found for module `kani`
  --> crates/vb_core/src/lib.rs:41:1
```

## Remaining blockers

- `cargo fuzz run vb_ui_model_postcard_decode -- -runs=1` still defaults to `x86_64-unknown-linux-musl`, which conflicts with AddressSanitizer/static libc. Use `--target x86_64-unknown-linux-gnu` until cargo-fuzz target policy is changed.
- `cargo kani -p vb_ui_model` is blocked by pre-existing `vb_core` Kani module wiring: missing `crates/vb_core/src/kani.rs` or `crates/vb_core/src/kani/mod.rs`.

## rerun_from

```bash
cd /home/lewis/src/vb-qi37-13-r2
mkdir -p target/tmp
TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo test -p velvet_ballistics exit_code --all-features
TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= verus verification/verus/diagnostic_envelope_verus.rs
TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo test -p vb_ui_model --all-features postcard
TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo run --manifest-path fuzz/Cargo.toml --features fuzz --bin vb_ui_model_postcard_decode -- < /dev/null
TMPDIR=$PWD/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

## State 10 implementation evidence

STATUS: PASS

State 10 implemented structured validation diagnostics for red CLI argument-parse cases while preserving stdout/stderr separation and public exit code `ValidationFailed = 1`.

### Files changed

- `crates/velvet_ballistics/src/main.rs`
  - Captures requested output format before argument parsing so parse failures can honor `--json` and `--jsonl`.
  - Emits `DiagnosticReport` stderr envelopes with `schema_version`, `kind`, `code`, `exit_code`, and exact parse-error `message` for structured parse failures.
  - Keeps text-mode parse failures on the existing help-bearing stderr path.
- `.beads/vb-qi37.13/implementation.md`
  - Appended State 10 implementation and command evidence.

### Command evidence

All commands used `/home/lewis/src/vb-qi37-13-r2` and `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER=`.

PASS:

```text
cargo test -p velvet_ballistics --test vb_qi37_13_structured_reconciliation --all-features
```

Observed: 6 passed, 0 failed.

PASS:

```text
cargo test -p vb_ui_model --all-features postcard
```

Observed: 12 passed, 0 failed.

PASS:

```text
cargo test -p velvet_ballistics exit_code --all-features
```

Observed: scoped exit-code tests passed; public exit matrix remains `0..=8` with no `9`.

PASS:

```text
verus verification/verus/diagnostic_envelope_verus.rs
```

Observed: `verification results:: 4 verified, 0 errors`.

PASS:

```text
cargo clippy -p velvet_ballistics --lib --bin velvet-ballistics --all-features -- -D warnings
```

PASS:

```text
cargo fmt --check -p velvet_ballistics
rustfmt --edition 2024 --check crates/velvet_ballistics/src/main.rs
```

BLOCKED / superseded by edition-aware command:

```text
rustfmt --check crates/velvet_ballistics/src/main.rs
```

Observed: direct rustfmt without edition failed while resolving `args.rs` because let-chains require Rust 2024. The edition-aware rustfmt command and `cargo fmt --check -p velvet_ballistics` passed.

### Residual risks

- No benchmark/profiler evidence was collected because this was correctness-only CLI diagnostic work and no performance claim is made.
- Existing JSON helper functions still use their prior serialization fallback shape; State 10 touched only parse-error diagnostics needed by the approved red tests.

## State 12 black-hat defect repair evidence

STATUS: PASS_READY_FOR_STATE_11_RERUN

Implemented repairs only in `/home/lewis/src/vb-qi37-13-r2`; did not use the broken `/home/lewis/src/vb-qi37-13` checkout.

### Files changed in this repair

- `crates/velvet_ballistics/Cargo.toml`
  - Added `blake3.workspace = true` for deterministic 32-byte CLI postcard payload digests.
- `Cargo.lock`
  - Updated by Cargo for the `velvet_ballistics` `blake3` dependency edge.
- `crates/velvet_ballistics/src/cli_postcard.rs`
  - Added contracted schema version and kind constants.
  - Added fail-closed validation for old/future version, wrong kind, payload bound, header CRC, and payload digest before returning payload bytes.
  - Replaced the previous partial `DefaultHasher` payload hash with BLAKE3 32-byte digest bytes.
  - Added tests for corrupted CRC, corrupted digest, old/future version, wrong kind, max+1 payload, truncated header, and successful round trip.
- `crates/velvet_ballistics/src/main.rs`
  - Routed `validate` and `explain` output format through command handlers instead of discarding it.
  - Added `DiagnosticReport` helpers emitting `schema_version`, `kind`, stable `code`, `exit_code`, and `message` to stderr for structured failures.
  - Updated missing-file read paths, validation YAML/compile failures, compile failures, representative runtime input mapping failures, and representative storage open/read failures to keep stdout empty and stderr structured for `--json`/`--jsonl`.
  - Added legacy JSON error inference so older public structured error call sites are wrapped as `DiagnosticReport` with validation/compile/runtime/storage codes instead of ad-hoc `{success:false,error}` payloads.
  - Preserved text-mode error output.
- `crates/velvet_ballistics/tests/vb_qi37_13_structured_reconciliation.rs`
  - Expanded diagnostic matrix coverage for missing-file validation, malformed YAML validation, missing-file compile, runtime input decode failure, and storage open failure.
- `.beads/vb-qi37.13/implementation.md`
  - Appended this State 12 evidence.

### Command evidence

All commands used `/home/lewis/src/vb-qi37-13-r2` with `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER=`.

PASS:

```text
cargo test -p velvet_ballistics --test vb_qi37_13_structured_reconciliation --all-features
```

Observed: 11 passed, 0 failed.

PASS:

```text
cargo test -p velvet_ballistics cli_postcard --all-features
```

Observed: 17 `cli_postcard` unit tests passed.

PASS:

```text
cargo test -p velvet_ballistics postcard --all-features
```

Observed: all filtered velvet postcard tests passed, including `cli_postcard`, `cli_run_maps_postcard_slot_values_from_input_bin`, output envelope postcard serialization, and non-postcard IR rejection.

PASS:

```text
cargo test -p velvet_ballistics exit_code --all-features
```

Observed: 9 exit-code tests plus filtered integration checks passed.

PASS:

```text
verus verification/verus/diagnostic_envelope_verus.rs
```

Observed: `verification results:: 4 verified, 0 errors`.

PASS:

```text
cargo clippy -p velvet_ballistics --lib --bin velvet-ballistics --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
```

PASS:

```text
rustfmt --edition 2024 --check crates/velvet_ballistics/src/main.rs crates/velvet_ballistics/src/cli_postcard.rs crates/velvet_ballistics/tests/vb_qi37_13_structured_reconciliation.rs
```

### State 11 routing

- Ready for State 11 rerun. State 11 ledger was not edited per instruction; formal-verifier owns that rerun.
- No route to State 3 is required: the contracted `cli_postcard::decode_postcard(data: &[u8])` was implemented rather than narrowed.

### Residual risks

- Existing broad CLI code still contains older ad-hoc `json_error` call sites outside the newly asserted matrix; `json_error` now wraps those as `DiagnosticReport` with a validation default, while representative compile/runtime/storage paths have explicit codes. A full per-command audit remains advisable before release hardening.
- No benchmark/profiler evidence was collected because this was correctness/security diagnostic repair only and no performance claim is made.

## State 12 remaining black-hat defect repair evidence (r2)

STATUS: PASS_READY_FOR_STATE_11_RERUN

Implemented repairs only in `/home/lewis/src/vb-qi37-13-r2`; did not use the broken checkout or source workspace.

### Files changed in this repair

- `crates/velvet_ballistics/src/main.rs`
  - Changed `verify <invalid-utf8-file> --json|--jsonl` to emit the stable `DiagnosticReport` envelope on stderr via `write_failure_message`, with stdout empty and public exit code `ValidationFailed = 1`.
  - Changed run-id parsing to accept `OutputFormat`, so `inspect not-a-run --db <tmp>/db --json` and sibling run-id routes emit structured validation diagnostics on stderr instead of plain text.
- `crates/velvet_ballistics/tests/vb_qi37_13_structured_reconciliation.rs`
  - Added black-box tests for `verify` invalid UTF-8 in `--json` and `--jsonl` modes.
  - Added black-box test for `inspect not-a-run --db <tmp>/db --json`.
- `.beads/vb-qi37.13/implementation.md`
  - Appended this r2 evidence.

### Command evidence

All commands used `/home/lewis/src/vb-qi37-13-r2` with `TMPDIR=$PWD/target/tmp RUSTC_WRAPPER=`.

PASS:

```text
cargo test -p velvet_ballistics --test vb_qi37_13_structured_reconciliation --all-features
```

Observed: 14 passed, 0 failed.

PASS:

```text
cargo test -p velvet_ballistics exit_code --all-features
```

Observed: 9 focused exit-code/unit tests passed, plus filtered integration checks passed.

PASS:

```text
verus verification/verus/diagnostic_envelope_verus.rs
```

Observed: `verification results:: 4 verified, 0 errors`.

PASS:

```text
cargo clippy -p velvet_ballistics --lib --bin velvet-ballistics --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
```

PASS:

```text
rustfmt --edition 2024 --check crates/velvet_ballistics/src/main.rs crates/velvet_ballistics/tests/vb_qi37_13_structured_reconciliation.rs
```

### State 11 routing

- Ready for State 11 rerun.
- Public CLI exit-code range remains `0..=8`; no new public exit code was added.

### Residual risks

- No benchmark/profiler evidence was collected because this was correctness-only CLI diagnostic repair and no performance claim is made.
- Remaining broad CLI audit risk from earlier State 12 still applies for unasserted command-specific `json_error` paths outside the focused black-hat cases.
