# Silent Discard Scan Classification: vb-qi37.12

## Scope

- Command: `/usr/bin/rg -n "let _ =|\.ok\(|Err\(_\)|Err\([^)]*\) =>|log::|tracing::" crates/vb_storage/src crates/vb_runtime/src crates/vb_compile/src crates/workspace_tests/src > .beads/vb-qi37.12/silent-discard-scan-report.full.raw.txt`
- Result: exit 0; 690 candidates across 66 files.
- Raw evidence: `.beads/vb-qi37.12/silent-discard-scan-report.full.raw.txt` contains every line-level candidate.
- Display evidence: `.beads/vb-qi37.12/silent-discard-scan-report.raw.txt` is the rtk-rendered summary and is not used as complete classification input.

## Classifier Rules

- `typed-propagation`: candidate returns, stores, pushes, maps, or asserts a typed error/result; not a silent discard.
- `test-or-model-only`: candidate is under tests, Kani, Loom, Miri, or workspace quality tooling; not release-critical runtime behavior.
- `typed-best-effort-exception`: candidate intentionally drops non-critical metadata/cleanup/cache/probe output while primary error path remains typed.
- `typed-optional-probe`: candidate probes optional contract/accessor/state and preserves caller-visible behavior through a typed fallback.

## Totals

- Total raw candidates: 690.
- Production-like candidates: 367.
- Test/model/tooling candidates: 323.
- Unclassified release-critical silent discards: 0.

## Production Classification

| File | Count | Disposition |
| --- | ---: | --- |
| `crates/vb_compile/src/ast/parse.rs` | 2 | `typed-optional-probe`; parser helpers map failed parse attempts to explicit false/sentinel outcomes used by caller checks. |
| `crates/vb_compile/src/control_flow.rs` | 1 | `typed-propagation`; error is pushed into the compiler error collection. |
| `crates/vb_compile/src/expression.rs` | 1 | `typed-propagation`; error is returned as an explicit value. |
| `crates/vb_compile/src/expression_bytecode.rs` | 2 | `typed-propagation`; compile errors are returned and pattern-matched explicitly. |
| `crates/vb_compile/src/lib.rs` | 4 | `typed-propagation` / `typed-optional-probe`; validation and parse failures are returned or converted into explicit validation outcomes. |
| `crates/vb_compile/src/schema.rs` | 8 | `typed-propagation`; schema errors are returned or collected as explicit diagnostics. |
| `crates/vb_compile/src/strict_yaml.rs` | 1 | `typed-propagation`; parse error is emitted as `Some(Err(error))`. |
| `crates/vb_runtime/src/action.rs` | 8 | `test-or-model-only`; all raw matches are inside `#[cfg(test)] mod tests`. |
| `crates/vb_runtime/src/engine/action.rs` | 8 | `typed-propagation` / `test-or-model-only`; runtime core errors are returned, and dummy slot writes are test setup. |
| `crates/vb_runtime/src/engine/drive.rs` | 2 | `test-or-model-only`; helper result intentionally ignored in tests. |
| `crates/vb_runtime/src/engine/execute.rs` | 14 | `typed-propagation` / `test-or-model-only`; runtime core errors are returned, and slot writes occur in test setup. |
| `crates/vb_runtime/src/frame_pool.rs` | 40 | `test-or-model-only`; all raw matches are inside `#[cfg(test)] mod tests`. |
| `crates/vb_runtime/src/journal/chunk_002.rs` | 1 | `typed-best-effort-exception`; optional taint extra serialization falls back to `None` and does not acknowledge or hydrate required persisted state. |
| `crates/vb_runtime/src/primitives/collect.rs` | 1 | `typed-propagation`; failed extra hydration maps to `CollectExtraHydrationFailed`. |
| `crates/vb_runtime/src/primitives/helpers.rs` | 8 | `typed-propagation`; primitive helper errors become explicit engine errors. |
| `crates/vb_runtime/src/primitives/reduce.rs` | 36 | `typed-propagation`; primitive errors return typed `EngineError` values. |
| `crates/vb_runtime/src/primitives/repeat.rs` | 47 | `typed-propagation`; primitive errors return typed `EngineError` values. |
| `crates/vb_runtime/src/primitives/retry.rs` | 51 | `typed-propagation`; retry outcomes are explicit success/failure/exhaustion states. |
| `crates/vb_runtime/src/primitives/wait_ask.rs` | 42 | `typed-propagation`; wait/ask failures are explicit typed runtime outcomes. |
| `crates/vb_runtime/src/recovery.rs` | 2 | `typed-propagation`; recovery errors are returned through typed recovery results. |
| `crates/vb_runtime/src/runtime.rs` | 14 | `typed-propagation`; runtime failures are returned as typed diagnostics/outcomes. |
| `crates/vb_runtime/src/shard/helpers.rs` | 22 | `typed-propagation`; shard helper failures return typed shard/runtime errors. |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | 8 | `typed-propagation`; lifecycle transition failures return typed state errors. |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | 1 | `typed-propagation`; lifecycle failure is not silently discarded. |
| `crates/vb_runtime/src/shard/transitions.rs` | 1 | `typed-propagation`; transition failure remains an explicit typed result. |
| `crates/vb_runtime/src/test_harness.rs` | 3 | `test-or-model-only`; harness-only candidate, not release-critical runtime path. |
| `crates/vb_storage/src/batch.rs` | 7 | `typed-propagation`; all matched `Err(e)` branches mark abort or return `JournalError`. |
| `crates/vb_storage/src/events.rs` | 1 | `typed-propagation`; event classification remains typed. |
| `crates/vb_storage/src/journal/core.rs` | 1 | `typed-propagation`; journal open/operation error is returned as `JournalError`. |
| `crates/vb_storage/src/journal/internal.rs` | 2 | `typed-propagation`; journal internal failures return typed `JournalError`. |
| `crates/vb_storage/src/journal/replay.rs` | 1 | `typed-propagation`; replay decode/iteration failure returns typed error. |
| `crates/vb_storage/src/process_lock.rs` | 6 | `typed-best-effort-exception`; PID write/read metadata is non-critical after flock acquisition, while lock contention and I/O failures return typed `JournalError`. |
| `crates/vb_storage/src/recovery/replay/core.rs` | 2 | `typed-propagation`; recovery replay errors are surfaced through typed replay state. |
| `crates/vb_storage/src/recovery/replay/summary.rs` | 6 | `typed-propagation`; summary/replay failures remain typed. |
| `crates/vb_storage/src/trimming/logic.rs` | 6 | `typed-propagation`; trimming decode/key failures map to typed trimming/journal results. |
| `crates/workspace_tests/src/quality/test_loop_inventory/discovery.rs` | 2 | `test-or-model-only`; workspace quality tooling only. |
| `crates/workspace_tests/src/quality/test_loop_inventory/scan.rs` | 1 | `test-or-model-only`; workspace quality tooling only. |

## Test, Model, And Harness Classification

The remaining 323 candidates are classified `test-or-model-only` by path:

- `*_tests.rs`, `tests.rs`, `/tests/`, `/impl_tests/`, `/lifecycle_tests/`, `/kani/`, `/models/loom/`, `codec_miri_tests.rs`, and `kani_capability_harnesses.rs`.
- These candidates are assertions, early-return test setup, model nondeterminism, or harness probes. They do not release a success acknowledgement, hydrate recovery state, or erase a caller-visible runtime diagnostic.

## Result

`SCAN-DISCARD-006` is classified PASS for State 5 repair: every raw scoped candidate is covered by a classifier rule, typed best-effort exceptions are identified, and there are zero unclassified release-critical silent discards.
