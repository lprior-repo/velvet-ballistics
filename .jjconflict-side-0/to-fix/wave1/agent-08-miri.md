# Wave 1 — Agent 08 (miri / UB-detector review)

Scope: 11 bugs (compiler/YAML/IR validation tier). Mode: read-only.
Source-tier policy: every production crate declares `#![forbid(unsafe_code)]`
at lib root (`vb_core/src/lib.rs:1`, `vb_runtime/src/runtime.rs:1`,
`vb_storage/src/keys.rs:1`, plus all submodules checked). No `unsafe`
blocks, raw pointers, `MaybeUninit`, `addr_of!`, `mem::transmute`, or
`repr(C)` / `repr(packed)` exist in the production source tree for these
fixes. Two `core::ptr::eq` callsites exist
(`vb_core/src/value_store.rs:423`, `vb_core/src/budget/tests_and_verification.rs:13`)
but they are pure pointer-equality on safe references and not in any of
the wave-1 fix paths.

Miri toolchain: `cargo-miri 0.1.0 (e0e95a7187 2026-04-04)` on
`rustup default nightly`. Strict-provenance flags accepted.

## Pre-existing baseline debt that gates vb_storage lib-test execution

`vb_storage/src/preview.rs:46-154` is a malformed production file
(`// TEST_MARKER_1`, duplicated test bodies, unbalanced braces — see
preview.rs:42 `// TEST_MARKER_1`, lines 106-153). `cargo test --lib -p
vb_storage` fails to compile with:

```
error: unexpected closing delimiter: `}`
   --> crates/vb_storage/src/preview.rs:154:1
```

This was already documented as BLOCK_GLOBAL baseline debt in wave-13/15
bead close reasons ("Pre-existing BLOCK_GLOBAL baseline debt (vb_storage
clippy, fmt drift, kind_edges/rs110 tests) unchanged and out of scope per
scoped per-crate gates" — vb-j4d19, vb-kqjo1, vb-kz475 close reasons).
It is not introduced by any wave-1 fix; it is upstream breakage.
Consequence: vb_storage and vb_runtime lib-test binaries do not compile,
so the regression tests for vb-j4d19, vb-j24jw, vb-k8eif, vb-keji6,
vb-kqjo1, vb-kz475, vb-kzpnj could not be executed via `cargo test`.
`vb_storage` `cargo check --lib` likewise fails for the same reason.
`cargo check` of `vb_core` and `velvet-ballistics-workspace-tests` passes
clean, and those test binaries do execute.

## Per-bug findings

| bug-id | pri | unsafe-touch | miri-needed | source-fix | test | miri-result | cargo-result | verdict | evidence |
|--------|-----|-------------|-------------|------------|------|-------------|--------------|---------|----------|
| vb-irenu | P2 | NO | NO | Duplicate of vb-ofk9m (RS-201 Arena::clear strands cleared slots). No production code change in this repo; closed by redirection to vb-ofk9m. No source path to inspect. | n/a | SKIPPED (no source path) | n/a | PATCHED (redirect) | `bd show vb-irenu` — close reason: "Duplicate of vb-ofk9m; same external_ref bug-hunt-2026-06-21:RS-201 remains tracked there." |
| vb-j24jw | P2 | NO | NO | `crates/vb_runtime/src/action.rs:196-206` — `validate_input_bytes` returns `ActionError::PayloadTooLarge { max_bytes, actual_bytes }` when `input.encoded_len() > contract.max_input_bytes`. Pure arithmetic compare. | `cargo test -p vb_runtime --lib action::` | SKIPPED (vb_runtime depends on vb_storage which fails to compile under `cargo test`) | BLOCKED (vb_storage lib build fails: preview.rs:154) | PATCHED (source-only, blocked from local regression by upstream baseline debt) | action.rs:197-206; preview.rs:42-154 |
| vb-j4d19 | P1 | NO | NO | `crates/vb_runtime/src/journal/chunk_001.rs:244-280` — `RuntimeJournal` trait + `NoopRuntimeJournal` default `append_sequenced_batch` contract. Pure enum + trait surface, no `unsafe`, no raw pointers. | n/a (default trait method, no test added in this fix's scope) | SKIPPED (lib build blocked by vb_storage preview.rs) | BLOCKED | PATCHED (source-only, blocked by upstream baseline debt) | chunk_001.rs:244-272 |
| vb-j83iq | P1 | NO | NO | `crates/vb_core/src/action.rs:711-720` — `check_output_slot_in_bounds` now rejects any ready output when `max_slots == 0` (returns `ActionError::OutputSlotOutOfBounds { slot, max_slots }` instead of silently passing). Pure integer comparison. | `vb_core::action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot` (also covered by `validate_action_dispatch_succeeds_with_zero_output_count`) | MIRI PASS (strict-provenance): `MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test -p vb_core --lib action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot` — `test result: ok. 1 passed; 0 failed` (24.48s) | PASS: `cargo test -p vb_core --lib action::tests --no-fail-fast` — 96 passed, 0 failed | PATCHED | action.rs:711-720; miri output; cargo test result |
| vb-k8eif | P3 | NO | NO | `crates/vb_storage/src/trimming/mod.rs:65-73` — `TrimError::diagnostic_code` for `Self::Journal(inner)` now delegates to `inner.diagnostic_code()` instead of `JournalError::FJALL_CODE`. Pure match expression. | `cargo test -p vb_storage --lib trimming` | SKIPPED (vb_storage lib build fails: preview.rs:154) | BLOCKED | PATCHED (source-only, blocked by upstream baseline debt) | trimming/mod.rs:65-73; preview.rs:42-154 |
| vb-keji6 | P2 | NO | NO | `crates/vb_storage/src/batch.rs:243-290` — `append_event` first checks staged-state duplicate key, then committed-state. The fix is a `BTreeMap`-style insert + `contains_key`; no raw pointer arithmetic. | `cargo test -p vb_storage --lib batch` | SKIPPED (vb_storage lib build fails: preview.rs:154) | BLOCKED | PATCHED (source-only, blocked by upstream baseline debt) | batch.rs:243-290; preview.rs:42-154 |
| vb-kqjo1 | P2 | NO | NO | `crates/vb_runtime/src/shard/types.rs:707-744` — `ShardConfig` public struct + `is_valid_trace_capacity` / `is_valid_step_budget_per_tick` predicates + `Default`. Pure data + const-fn predicates. | `cargo test -p vb_runtime --lib shard` | SKIPPED (vb_runtime depends on vb_storage which fails to compile) | BLOCKED | PATCHED (source-only, blocked by upstream baseline debt) | shard/types.rs:707-744 |
| vb-krus1 | P1 | NO | NO | `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs:101-112` — `ipc_decode_order_proptest` selector 3 now writes `1` to bytes[10..12] and asserts `IpcError::ReservedNonZero { actual: 1 }`. Pure byte-buffer mutation against safe-API decoder; `#![forbid(unsafe_code)]` on the test file. | `cargo test -p velvet-ballistics-workspace-tests --test restate_decode_error_taxonomy_tests` | SKIPPED (no unsafe touch — workspace_tests transitively uses unsafe in `bytes` / `fjall` deps, but no unsafe in the fix path) | PASS: 6/6 (incl. `ipc_decode_order_proptest`, `storage_decode_order_proptest`, `ipc_header_constants_are_current_public_contract`) | PATCHED | restate_decode_error_taxonomy_tests.rs:101-128; cargo test result |
| vb-kxf5z | P3 | NO | NO | `crates/vb_core/src/span.rs:9-31` — `Span::new(start, end)` accepts inverted byte ranges (start > end) as a pure data field; `is_empty()` test confirms equivalence. The fix is "documented-allowed" semantics — no panic, no UB. Const-fn only. | `vb_core::span::tests::*` | MIRI PASS (strict-provenance): `MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test -p vb_core --lib span::tests::span_new_at_max_offsets` — `test result: ok. 1 passed; 0 failed` (31.53s). Miri also passed for full `span::tests` module (21 tests, 15.66s). | PASS: `cargo test -p vb_core --lib span::tests --no-fail-fast` — 21 passed, 0 failed | PATCHED | span.rs:9-31; miri output; cargo test result |
| vb-kz475 | P1 | NO | NO | `crates/vb_storage/src/recovery/hydrate_support.rs:264-484` — `apply_tail_events` now resolves frame taint from `resolve_slot_taint_read(observe_slot_taint_read(frame.read_taint(*slot)))` per `SlotWrittenEvent`. Pure pattern matching on enum + safe frame mutators. | `cargo test -p vb_storage --lib recovery::tests::apply_tail_events_*` | SKIPPED (vb_storage lib build fails: preview.rs:154) | BLOCKED | PATCHED (source-only, blocked by upstream baseline debt) | hydrate_support.rs:264-484; preview.rs:42-154 |
| vb-kzpnj | P2 | NO | NO | `crates/vb_storage/src/hydrate_tests.rs` — replaces 3 `assert!(result.is_ok())` smoke tests (lines 281, 287, 374 cited in close reason) with `matches!(result, Ok(()))` shape matches. Test-only change, no production code. | `cargo test -p vb_storage --lib hydrate_tests` | SKIPPED (vb_storage lib build fails: preview.rs:154) | BLOCKED | PATCHED (test-only, blocked by upstream baseline debt) | hydrate_tests.rs:1-133; preview.rs:42-154 |

## Miri raw-output excerpts

### CV-103 (vb-j83iq) — strict-provenance miri
```
$ MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test \
    -p vb_core --lib action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot
   Finished `test` profile [unoptimized + debuginfo] target(s)
    Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/vb_core-14e15907cf26bccb)

running 1 test
test action::tests::validate_action_outcome_ready_rejects_out_of_bounds_slot ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2130 filtered out; finished in 24.48s
```

### CV-106 (vb-kxf5z) — strict-provenance miri
```
$ MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test \
    -p vb_core --lib span::tests::span_new_at_max_offsets
    Finished `test` profile [unoptimized + debuginfo] target(s)
    Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/vb_core-14e15907cf26bccb)

running 1 test
test span::tests::span_new_at_max_offsets ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2130 filtered out; finished in 31.53s
```

### vb-krus1 — cargo test
```
$ cargo test -p velvet-ballistics-workspace-tests --test restate_decode_error_taxonomy_tests
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.19s
    Running tests/restate_decode_error_taxonomy_tests.rs (target/debug/deps/restate_decode_error_taxonomy_tests-932e61bf67db98fe)

running 6 tests
test ipc_header_constants_are_current_public_contract ... ok
test ipc_payload_too_large_precedes_read_property ... ok
test ipc_decode_order_proptest ... ok
test storage_decode_order_proptest ... ok
test storage_payload_too_large_precedes_read_property ... ok
test storage_numeric_fields_are_observable ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### vb-core wide regression cargo test (covers CV-103 + CV-106 fix paths)
```
$ cargo test -p vb_core --lib --no-fail-fast
test result: ok. 2131 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Miri UB-relevant findings

No UB-relevant concerns in any of the 11 wave-1 fixes.

- No `unsafe` block, raw pointer, `MaybeUninit`, `addr_of!`,
  `mem::transmute`, `repr(C)`, `repr(packed)`, or strict-provenance-
  sensitive operation introduced or touched by any fix.
- The two safe `core::ptr::eq` callsites
  (`vb_core/src/value_store.rs:423`,
  `vb_core/src/budget/tests_and_verification.rs:13`) are pre-existing and
  not in any wave-1 fix path; they compare slice addresses derived from
  the same allocator, so strict-provenance is satisfied by construction.
- `vb_core` strict-provenance miri runs (CV-103, CV-106) complete without
  diagnostics.

## Caveats / follow-ups

1. `vb_storage/src/preview.rs:42-154` is a malformed production file
   unrelated to wave-1 fixes. It blocks `cargo test --lib -p vb_storage`
   and any dependent crate (`vb_runtime`, `velvet-ballistics-workspace-tests`
   lib). This was already documented as BLOCK_GLOBAL baseline debt in
   wave-13/15 close reasons and remains unowned. A targeted repair is
   needed before the blocked seven bugs can be regression-tested locally.
2. `vb-j4d19` (RE-016) was originally described as
   `RuntimeJournal::append_sequenced_batch`. The current
   `vb_runtime/src/journal/chunk_001.rs:244-272` shows the trait
   `RuntimeJournal` with `append_sequenced` (not `append_sequenced_batch`).
   The bug close reason (`Implemented and verified`) and the
   wave-15 commit `5b3273e9d implement ready bead batch` confirm the
   contract was reshaped during the ready-bead batch implementation.
   The shipped trait surface is safe Rust — no UB exposure — but the
   original test name referenced in the bead description no longer
   exists. Treat as PATCHED via API evolution rather than a pure in-place
   patch.

## File-path

This report: `/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-08-miri.md`