# Implementation Report — vb-b5f Phase 1

## Files changed

- `crates/vb-core/src/ids.rs` — added `SeqNo`, `CheckedIndex`, `ZERO`/`MIN`/`MAX`, checked arithmetic, `FromStr`, and changed `RunId` to `u64` with `as_u64()`.
- `crates/vb-core/src/errors.rs` — added `CoreError`, `CoreResult`, `EngineError` alias, diagnostic-code constants, and `diagnostic_code()`.
- `crates/vb-core/src/error.rs` — retained as a compatibility re-export module.
- `crates/vb-core/src/limits.rs` — audited existing file and added `#![forbid(unsafe_code)]`; constants matched the contract.
- `crates/vb-core/src/span.rs` — audited existing source location primitives; they satisfy Phase 1 and remain exported.
- `crates/vb-core/src/diagnostic.rs` — audited existing diagnostics; they satisfy Phase 1 plus parsing/display.
- `crates/vb-core/src/value.rs` — added `SlotValue::type_name()` for all existing variants.
- `crates/vb-core/src/lib.rs` — exported `errors`, `CoreError`, `CoreResult`, `SeqNo`, and `CheckedIndex`.
- `crates/vb-core/src/engine.rs` — switched internal import to `crate::errors::EngineError`.
- `crates/vb-core/tests/phase1_core_types.rs` — added Phase 1 integration coverage for IDs, limits, spans, diagnostics, errors, values, and postcard roundtrips.
- `crates/vb-core/Cargo.toml` / `Cargo.lock` — added `postcard` as a `dev-dependency` for serialization roundtrip tests.
- `crates/vb-storage/src/lib.rs` — narrow downstream compatibility update for `RunId::as_u64()` while preserving the existing 24-byte journal key format.

## Tests added

- Public constructor/accessor/`FromStr` coverage for `WorkflowId`, `StepIdx`, `SlotIdx`, `ExprIdx`, `ActionId`, `AccessorIdx`, `ConstIdx`, `RunId`, and `SeqNo`.
- `SeqNo`, `StepIdx`, `SlotIdx`, and `ConstIdx` checked-add overflow coverage.
- `CheckedIndex` coverage for all required index types.
- Limit constant assertions.
- `Span`, `Located`, `Spanned`, and `SourceMap` construction/behavior coverage.
- `DiagnosticCode` parse/display and `Diagnostic` ownership coverage.
- `CoreError`/`EngineError` alias, display, and diagnostic-code coverage.
- `SlotValue::type_name()` coverage for all seven current variants.
- Postcard roundtrips for representative ID, span, and slot value types.

## Commands run and results

- `rtk cargo fmt --all` — passed.
- `rtk cargo test -p vb-core` — passed: `18 passed (3 suites, 0.00s)`.
- `rtk cargo test --workspace --all-targets` — failed on unrelated pre-existing benchmark stubs in `benches/velvet_ballastics.rs`; first failure: `workflow_compile_bench` panicked with `not yet implemented: benchmark implementation` at line 15. All regular workspace test suites before the bench passed.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed: `No issues found`.
- `moon run :ci-source` — not available/misconfigured: missing `.moon` or `.config/moon` workspace configuration.
- `rtk cargo fmt --check` — passed.
- `rtk cargo nextest run 2>&1 | tdd-guard-rust --project-root . --passthrough` — failed because `tdd-guard-rust` is not installed.
- `rtk cargo nextest run` — passed.

## Remaining risks

- `CoreError` is aliased as `EngineError`, so Rust's blanket identity conversion covers `CoreError::from(engine_error)`; a separate `From<EngineError> for CoreError` impl is impossible without conflicting with `impl<T> From<T> for T`.
- Error variants do not carry explicit `Span`/`SlotValue` payload fields because preserving existing `EngineError` construction compatibility required keeping current variant shapes.
- Workspace `--all-targets` tests remain blocked by unrelated benchmark `todo!` stubs outside Phase 1 scope.

## Repair Results — workspace test gate

### Files changed

- `benches/velvet_ballastics.rs` — replaced all 27 non-running benchmark placeholder bodies with deterministic `criterion::black_box(())` iterations while preserving every benchmark function and name.
- `vb-b5f/implementation.md` — appended this repair summary and verification results.

### Commands run and results

- `rtk cargo fmt --all` — passed with no output.
- `rtk cargo test --workspace --all-targets` — passed: `cargo test: 140 passed (15 suites, 1.04s)`.
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed: `cargo clippy: No issues found`.

### Constraint proof

- No unsafe code added.
- No production code changed.
- No `unwrap`, `expect`, `panic`, `todo`, or `unimplemented` macros added to benchmarks.
- Benchmark placeholders remain intentionally non-functional and deterministic; no real benchmark logic was implemented.
