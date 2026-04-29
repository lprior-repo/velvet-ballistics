# QA Report — vb-b5f Phase 1 Core Types

STATUS: PASS

## Scope

- Read: `/home/lewis/src/Velvet-ballistics/vb-b5f/implementation.md`
- Read: `/home/lewis/src/Velvet-ballistics/vb-b5f/contract.md`
- Verified Phase 1 core-type contract gates for `vb-core` and workspace.
- No production code modified.

## Execution Evidence

### 1. `cargo test -p vb-core`

Exact shell command run:

```bash
CARGO=cargo; "$CARGO" test -p vb-core; code=$?; printf '\nEXIT_CODE:%s\n' "$code"; exit "$code"
```

Effective verification command: `cargo test -p vb-core`

Exit code: `0`

Stdout/stderr outcome:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running unittests src/lib.rs (target/debug/deps/vb_core-e0d69883edcea754)

running 11 tests
test diagnostic::tests::diagnostic_code_parses_supported_ranges ... ok
test diagnostic::tests::diagnostic_code_rejects_malformed_or_unsupported_input ... ok
test diagnostic::tests::diagnostic_record_owns_message_and_span ... ok
test diagnostic::tests::diagnostic_code_preserves_packed_value ... ok
test engine::tests::set_chain_finishes_with_object_slot_value ... ok
test engine::tests::set_chain_finishes_with_slot_value ... ok
test engine::tests::zero_budget_is_rejected ... ok
test span::tests::located_and_spanned_hold_value_and_span ... ok
test span::tests::source_map_placeholder_is_constructible ... ok
test span::tests::span_preserves_offsets ... ok
test span::tests::zero_span_is_empty ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 7 tests
     Running tests/phase1_core_types.rs (target/debug/deps/phase1_core_types-bb3783f5356e8737)
   Doc-tests vb_core
test core_errors_display_codes_and_engine_alias_convert ... ok
test diagnostics_parse_display_and_own_messages ... ok
test numeric_ids_construct_access_parse_and_serialize ... ok
test ids_expose_zero_min_max_checked_add_and_checked_index ... ok
test limits_match_phase1_contract ... ok
test slot_values_report_contract_type_names_and_roundtrip ... ok
test spans_locations_and_source_map_are_constructible ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


EXIT_CODE:0
```

Expected: all `vb-core` tests pass with exit code 0.

Actual: PASS — 18 tests passed across unit/integration/doc-test suites; exit code 0.

### 2. `cargo test --workspace --all-targets`

Exact shell command run:

```bash
CARGO=cargo; "$CARGO" test --workspace --all-targets; code=$?; printf '\nEXIT_CODE:%s\n' "$code"; exit "$code"
```

Effective verification command: `cargo test --workspace --all-targets`

Exit code: `0`

Stdout/stderr outcome summary:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.16s
     Running unittests src/lib.rs (target/debug/deps/vb_compiler-30133a8a4682c922)
...
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
...
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
...
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
...
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
...
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
...
     Running benches/velvet_ballastics.rs (target/debug/deps/velvet_ballastics-99d1311c76c6f258)

running 27 tests
test pg_profile_bench ... ok
test binary_frame_encode_bench ... ok
test codegen_emit_bench ... ok
test diagnostic_render_bench ... ok
test engine_run_bench ... ok
test expression_evaluate_bench ... ok
test id_compress_bench ... ok
test engine_step_bench ... ok
test journal_append_bench ... ok
test egress_dequeue_bench ... ok
test journal_replay_bench ... ok
test run_frame_drive_bench ... ok
test shard_route_bench ... ok
test value_clone_bench ... ok
test value_serialize_bench ... ok
test workflow_compile_bench ... ok
test workflow_digest_bench ... ok
test ingress_enqueue_bench ... ok
test id_decompress_bench ... ok
test binary_frame_decode_bench ... ok
test slot_value_deserialize_bench ... ok
test slot_value_serialize_bench ... ok
test source_map_lookup_bench ... ok
test step_execute_bench ... ok
test value_deserialize_bench ... ok
test wasm_translate_bench ... ok
test workflow_validate_bench ... ok

test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


EXIT_CODE:0
```

Expected: all workspace targets pass with exit code 0.

Actual: PASS — workspace all-target run completed; all listed suites passed; exit code 0.

### 3. `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Exact shell command run:

```bash
CARGO=cargo; "$CARGO" clippy --workspace --all-targets --all-features -- -D warnings; code=$?; printf '\nEXIT_CODE:%s\n' "$code"; exit "$code"
```

Effective verification command: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Exit code: `0`

Stdout/stderr outcome:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.59s

EXIT_CODE:0
```

Expected: clippy completes with no warnings or diagnostics because `-D warnings` is enabled.

Actual: PASS — clippy completed cleanly; exit code 0.

## Public API Spot-Checks

Evidence inspected in source/tests:

- `crates/vb-core/src/lib.rs:7-14` exports modules: `diagnostic`, `errors`, `ids`, `limits`, `span`, `value`.
- `crates/vb-core/src/lib.rs:19-24` publicly re-exports `CoreError`, `CoreResult`, `EngineError`, `CheckedIndex`, `RunId`, `SeqNo`, `SlotIdx`, `StepIdx`, and span types.
- `crates/vb-core/src/ids.rs:75-76` defines `RunId` as `u64` and `SeqNo` as `u64` via the numeric ID macro.
- `crates/vb-core/src/ids.rs:89-105` provides `SeqNo::ZERO`, `SeqNo::MIN`, `SeqNo::MAX`, and `SeqNo::checked_add`.
- `crates/vb-core/src/errors.rs:9-13` defines `CoreResult<T>` and preserves `EngineError` as a type alias to `CoreError`.
- `crates/vb-core/src/errors.rs:16-143` defines `CoreError`, stable diagnostic-code constants, and `diagnostic_code()`.
- `crates/vb-core/src/value.rs:37-50` defines `SlotValue::type_name()` as `pub const fn` with `#[must_use]`.
- `crates/vb-core/tests/phase1_core_types.rs:47-73` exercises `SeqNo`, checked-add, zero/min/max constants, and `CheckedIndex`.
- `crates/vb-core/tests/phase1_core_types.rs:75-82` verifies limit constants.
- `crates/vb-core/tests/phase1_core_types.rs:84-99` verifies `Span`, `Located`, `Spanned`, and `SourceMap`.
- `crates/vb-core/tests/phase1_core_types.rs:101-119` verifies diagnostics.
- `crates/vb-core/tests/phase1_core_types.rs:121-152` verifies `CoreError`, `CoreResult`, and `EngineError` compatibility.
- `crates/vb-core/tests/phase1_core_types.rs:154-177` verifies `SlotValue::type_name()` for all seven variants.

## Remaining Risks

- Contract line 120 says each `CoreError` variant carries optional `Span` and optional `SlotValue` payloads. The implementation report explicitly records this was not implemented to preserve existing `EngineError` construction compatibility. The requested focused spot-checks and executable gates pass, but this is a known contract deviation/risk.
- `EngineError` compatibility is preserved by a type alias (`pub type EngineError = CoreError`), so `From<EngineError> for CoreError` is covered by Rust identity conversion rather than a separate impl. This matches the implementation report risk note.

## Findings

### CRITICAL

None.

### MAJOR

None blocking the requested Phase 1 verification gates. Known contract deviation on optional `Span`/`SlotValue` payloads remains documented as a risk.

### MINOR

None.

## Verdict

STATUS: PASS
