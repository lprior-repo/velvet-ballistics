# IR Primitive Coverage Matrix

This matrix enumerates every `CompiledNodeKind` variant defined in
`crates/vb_core/src/workflow/node.rs` and lists the test, proof, and benchmark
artifacts that exercise it. Closes the master line-1668 gap
("Full final primitive semantics still require end-to-end proof").

## Conventions

- **Test** — unit or integration test exercising the variant.
- **Proof** — Kani / Flux / Verus / Loom model proving an invariant
  about the variant.
- **Bench** — Criterion / iai-callgrind benchmark for the variant.
- **Source** — the lowering site where the variant is emitted by the
  compiler.

## Coverage Matrix

| Variant                     | Source (compiler)                                       | Test                                                                                            | Proof                                                       | Bench                                      |
|-----------------------------|---------------------------------------------------------|-------------------------------------------------------------------------------------------------|-------------------------------------------------------------|--------------------------------------------|
| `Nop`                       | (pass-through)                                          | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `SetConst`                  | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | `crates/vb_core/.../kani_digest_step_primitive_no_panic.rs` | —                                          |
| `Copy`                      | `crates/vb_compile/src/mod_compile_lowering/part_01.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `EvalExpr`                  | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `BuildObject`               | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `BuildList`                 | `crates/vb_compile/src/mod_compile_lowering/part_05.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `Do`                        | `crates/vb_compile/src/mod_compile_lowering/part_03.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `Choose`                    | `crates/vb_compile/src/mod_compile_lowering/part_07.rs` | `crates/vb_compile/tests/proptest/proptest_choose_width.rs`                                     | —                                                           | —                                          |
| `ChooseSlot`                | `crates/vb_compile/src/mod_compile_lowering/part_07.rs` | `crates/vb_compile/tests/proptest/proptest_choose_*.rs`                                        | —                                                           | —                                          |
| `ForEachStart`              | `crates/vb_compile/src/mod_compile_lowering/part_02.rs` | `crates/vb_compile/tests/foreach_at_once_tests.rs`                                             | `crates/vb_compile/src/kani_foreach_parity.rs`              | —                                          |
| `ForEachNext`               | `crates/vb_compile/src/mod_compile_lowering/part_02.rs` | `crates/vb_compile/tests/foreach_at_once_tests.rs`                                             | `crates/vb_compile/src/kani_foreach_parity.rs`              | —                                          |
| `ForEachJoin`               | `crates/vb_compile/src/mod_compile_lowering/part_02.rs` | `crates/vb_compile/tests/foreach_at_once_tests.rs`                                             | `crates/vb_compile/src/kani_foreach_parity.rs`              | —                                          |
| `TogetherStart`             | `crates/vb_compile/src/mod_compile_lowering/part_10.rs` | `crates/vb_compile/src/mod_compile_lowering/together_e2e_tests.rs`                              | `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`   | —                                          |
| `TogetherBranch`            | `crates/vb_compile/src/mod_compile_lowering/part_10.rs` | `crates/vb_compile/src/mod_compile_lowering/together_integration_tests.rs`                      | `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`   | —                                          |
| `TogetherJoin`              | `crates/vb_compile/src/mod_compile_lowering/part_10.rs` | `crates/vb_compile/src/mod_compile_lowering/together_integration_tests.rs`                      | `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`   | —                                          |
| `CollectStart`              | `crates/vb_compile/src/mod_compile_lowering/part_08.rs` | `crates/vb_compile/src/workflow/proptest_collect_traversal.rs`                                 | —                                                           | —                                          |
| `CollectPage`               | `crates/vb_compile/src/mod_compile_lowering/part_08.rs` | `crates/vb_compile/src/workflow/proptest_collect_traversal.rs`                                 | —                                                           | —                                          |
| `CollectNext`               | `crates/vb_compile/src/mod_compile_lowering/part_08.rs` | `crates/vb_compile/src/workflow/proptest_collect_traversal.rs`                                 | —                                                           | —                                          |
| `CollectFinish`             | `crates/vb_compile/src/mod_compile_lowering/part_08.rs` | `crates/vb_compile/src/workflow/proptest_collect_traversal.rs`                                 | —                                                           | —                                          |
| `ReduceStart`               | `crates/vb_compile/src/mod_compile_lowering/part_09.rs` | `crates/vb_compile/tests/integration_reduce_tests.rs`                                          | —                                                           | —                                          |
| `ReduceNext`                | `crates/vb_compile/src/mod_compile_lowering/part_09.rs` | `crates/vb_compile/tests/integration_reduce_tests.rs`                                          | —                                                           | —                                          |
| `ReduceFinish`              | `crates/vb_compile/src/mod_compile_lowering/part_09.rs` | `crates/vb_compile/tests/integration_reduce_tests.rs`                                          | —                                                           | —                                          |
| `RepeatStart`               | `crates/vb_compile/src/mod_compile_lowering/part_11.rs` | `crates/vb_compile/tests/repeat_digest_integration.rs`                                         | `crates/vb_compile/src/kani_digest_repeat.rs`               | —                                          |
| `RepeatAttempt`             | `crates/vb_compile/src/mod_compile_lowering/part_11.rs` | `crates/vb_compile/tests/repeat_digest_integration.rs`                                         | `crates/vb_compile/src/kani_digest_repeat.rs`               | —                                          |
| `RepeatCheck`               | `crates/vb_compile/src/mod_compile_lowering/part_11.rs` | `crates/vb_compile/tests/repeat_digest_integration.rs`                                         | `crates/vb_compile/src/kani_digest_repeat.rs`               | —                                          |
| `RepeatFinish`              | `crates/vb_compile/src/mod_compile_lowering/part_11.rs` | `crates/vb_compile/tests/repeat_digest_integration.rs`                                         | `crates/vb_compile/src/kani_digest_repeat.rs`               | —                                          |
| `WaitUntil`                 | `crates/vb_compile/src/mod_compile_lowering/part_12.rs` | `crates/vb_compile/tests/digest_ask_timeout_sensitivity.rs`                                    | `crates/vb_compile/src/kani_digest_ask_timeout_sensitivity.rs` | —                                          |
| `WaitEvent`                 | `crates/vb_compile/src/mod_compile_lowering/part_12.rs` | `crates/vb_compile/tests/digest_ask_determinism.rs`                                            | `crates/vb_compile/src/kani_digest_ask_field_ordering.rs`  | —                                          |
| `Ask`                       | `crates/vb_compile/src/mod_compile_lowering/part_13.rs` | `crates/vb_compile/tests/digest_ask_determinism.rs`                                            | `crates/vb_compile/src/kani_digest_ask_field_ordering.rs`  | —                                          |
| `AskResume`                 | `crates/vb_compile/src/mod_compile_lowering/part_13.rs` | `crates/vb_compile/tests/digest_ask_determinism.rs`                                            | `crates/vb_compile/src/kani_digest_ask_field_ordering.rs`  | —                                          |
| `RetryCheck`                | `crates/vb_compile/src/mod_compile_lowering/part_14.rs` | `crates/vb_compile/tests/digest_ask_determinism.rs`                                            | `crates/vb_compile/src/kani_digest_ask_field_ordering.rs`  | —                                          |
| `ErrorHandler`              | `crates/vb_compile/src/mod_compile_lowering/part_15.rs` | `crates/vb_compile/tests/finish_digest_integration.rs`                                         | —                                                           | —                                          |
| `Jump`                      | `crates/vb_compile/src/mod_compile_lowering/part_06.rs` | `crates/vb_compile/tests/v1_primitive_lowering.rs`                                             | —                                                           | —                                          |
| `Finish`                    | `crates/vb_compile/src/mod_compile_lowering/part_16.rs` | `crates/vb_compile/tests/finish_digest_integration.rs`                                         | `crates/vb_compile/src/kani_finish_digest.rs`               | —                                          |

## Gap Inventory

The following variants do **not** have a dedicated Kani/Flux proof target
above. Each is exercised by at least one integration test but lacks a
bounded-model-checker invariant:

- `Nop`, `SetConst`, `Copy`, `EvalExpr`, `BuildObject`, `BuildList`,
  `Do`, `Choose`, `ChooseSlot`, `Jump`.

The following variant families are missing dedicated **benchmark** entries:

- All `CompiledNodeKind` variants. The `crates/vb_benchmark/` crate does
  not yet contain per-variant micro-benchmarks.

## Evidence Compilation

This matrix is the source of truth for tier-a-3-009 and is referenced by
the tier-a-3-008 roundtrip proptest (`crates/vb_compile/tests/proptest_compile_ir_roundtrip.rs`).

## Update Cadence

Re-generate this matrix when:

- A new `CompiledNodeKind` variant is added.
- A new Kani harness or Flux proof lands.
- A new benchmark is added to `crates/vb_benchmark/`.
- A lowering site is moved between `mod_compile_lowering/part_*.rs` files.
