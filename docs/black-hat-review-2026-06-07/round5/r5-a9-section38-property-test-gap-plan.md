# Section 38 Property-Test Gap Remediation — Round 5 Implementation Plan

**Date**: 2026-06-07
**Scope**: Fill 4 ship-blocker property-test gaps, strengthen 5 aliases, and replace the broken coverage/density/branch evidence surface.
**Total estimated effort**: 42 hours (5.25 engineer-days).
**Definition of done**: `moon run :coverage` produces a real `lcov.info` ≥ 100 KB; `moon run :test-density-gate` fails on density < 5x; all 4 new proptest files compile and pass; all 5 strengthened aliases have new test functions asserting exact behavior; branch coverage reports ≥1 branch per file in the changed crates.

---

## 1. Inventory of Round 4 Findings

| # | Item | Severity | Type | Current state |
|---|------|----------|------|---------------|
| 1 | `concurrency_safety` proptest missing | SHIP-BLOCKER | Property test | No proptest for production Mutex paths under concurrent fuzz |
| 2 | `bytecode_ast_parity` file absent | SHIP-BLOCKER | Property test | Comment in `crates/vb_compile/src/lib.rs:64` is a lie; no file exists |
| 3 | `taint_propagation` proptest missing | SHIP-BLOCKER | Property test | 2,578 lines of hardcoded `#[test]` in `integration_taint_propagation.rs`, zero proptests |
| 4 | `error_recovery` proptest missing | SHIP-BLOCKER | Property test | No proptest for recovery from fuzz-malformed journal records |
| 5 | `digest_stability` aliased | alias | Property test | Currently in `proptest_digest_determinism.rs` (Ask-only, narrow scope) |
| 6 | `for_each_ordering` aliased | alias | Property test | Kani only at `crates/vb_runtime/src/verification/kani/kani_for_each_ordering.rs:46` |
| 7 | `resource_budget` aliased | alias | Property test | `proptest_attempt_fence.rs` covers budget arithmetic but not retry-attempt/time-limit |
| 8 | `bound_enforcement` aliased | alias | Property test | `proptest_workflow.rs` tests validation-time only |
| 9 | `layout_stability` MISSING | alias | Property test | No file in tree |
| 10 | `tarpaulin-report.json` 3 bytes | EVIDENCE | Coverage | Contains `{}` + newline; no actual report |
| 11 | `coverage.log`/`llvm-cov.log` 1-line stubs | EVIDENCE | Coverage | "fixture-backed gate execution; no raw tool output" |
| 12 | 5x test density unenforced | EVIDENCE | CI gate | Mentioned in `to-fix/04-ci-formal-evidence-defects.md:48` |
| 13 | Branch coverage broken | EVIDENCE | Coverage | `vb_ipc/test-suite-review.md:9` reports 0 branches for all files |

---

## 2. Property Tests to Create / Strengthen

### 2.1 NEW: `concurrency_safety` property test (SHIP-BLOCKER #1)

**Defect**: Production Mutex paths exist in `vb_storage::queue::writer::JournalWriterQueue` (`crates/vb_storage/src/queue/writer.rs:18`), `vb_storage::journal::core::FjallJournal` (`crates/vb_storage/src/journal/core.rs:8`), and `vb_runtime::action_queue::queue::BoundedActionCompletionQueue` (`crates/vb_runtime/src/action_queue/queue.rs:19,38`). Existing concurrency tests in `crates/vb_storage/src/edge_case_tests.rs:85-300` are hand-coded `#[test]` functions with hardcoded thread counts and event counts. They do not exercise the property that **for any** thread count and any interleaving of enqueue/drain operations, the queue never loses, duplicates, or reorders accepted events.

**Fix**: Create a proptest that fuzzes (thread_count × operation_count × operation_kind) tuples and asserts the queue's loss/dup/reorder invariants.

**File**: `crates/vb_storage/tests/proptest_concurrency_safety.rs` (NEW, ~250 lines)
- `proptest!` block 1: `prop_journal_writer_queue_no_lose_under_concurrent_fuzz` (N=8 threads × 32 ops × 5 op kinds)
  - For any valid (thread_count, ops_per_thread, capacity) ≤ (8, 64, 256), spawn threads, allocate `Arc<JournalWriterQueue>`, drive a generator-selected mix of `enqueue_journaled` / `enqueue_strict` / `drain_all` ops, join, then assert: `pending.len() == 0` after all drains, and total accepted events == events successfully appended to journal.
  - Use `prop_assume!` to skip capacity-zero cases (constructor rejects with `JournalError::QueueCapacity`).
- `proptest!` block 2: `prop_bounded_action_queue_no_lose_under_concurrent_fuzz` (mirror the above for `vb_runtime::action_queue::queue::BoundedActionCompletionQueue` — same Mutex, same poison-recovery idiom).
  - Assert: every successfully enqueued `ActionTicket` (tickets that returned `Ok(())` on `enqueue`) is observable via `try_dequeue` and no ticket is duplicated.
- `proptest!` block 3: `prop_fjall_journal_no_lose_under_concurrent_fuzz` (per-run-key, since Fjall's whole-journal lock is per-`Arc<FjallJournal>`).
  - Assert: events appended from N threads for distinct `(run, seq)` pairs are all readable after a barrier-join.

**Acceptance**:
- File compiles under `cargo check -p vb_storage --tests --all-features` with no `#[allow(unsafe_code)]` and no forbidden patterns.
- `cargo test -p vb_storage --test proptest_concurrency_safety` passes ≥256 cases per proptest block.
- Each `prop_assert!` uses `prop_assert_eq!` or `prop_assert!` with a precise predicate (no `.is_ok()` / `.is_err()`).
- Proptest config: `ProptestConfig::with_cases(256)`, fork disabled (these touch the same `Arc`).
- A test that uses 0 threads or 0 ops is filtered via `prop_assume!` so the rejection is explicit.

**Test count delta**: +3 proptest functions (each ≥256 cases).
**Risk**: MEDIUM — proptest shared-state with threads can cause flakiness. Mitigate by joining all threads inside the test body before assertions. Use `Arc<JournalWriterQueue>` and `Arc<FjallJournal>` (the production types). The mutex poison-recovery in `action_queue/queue.rs:53-55` (`.into_inner()` on poison) is exercised by proptest's fork-disabled mode.
**Hours**: 4
**Bead ID**: `vb-cs38.1`

---

### 2.2 NEW: `bytecode_ast_parity` property test (SHIP-BLOCKER #2)

**Defect**: `crates/vb_compile/src/lib.rs:64` says `// TEMPORARILY DISABLED: pre-existing proptest macro compatibility issue in bytecode_ast_parity.rs` and the very next line is `// #[cfg(test)] mod property_tests;`. There is no `bytecode_ast_parity.rs` file in the repository. The comment is a stale lie from a prior round. The `expression_bytecode_tests.rs` file (`crates/vb_compile/src/expression_bytecode_tests.rs`, 65.7K) has 30+ hand-coded unit tests but no proptest that compares the bytecode-emitted `ExprProgram::ops` against a parallel AST interpreter. Section 38 mandates: "Compiled bytecode produces same result as AST interpretation."

**Fix**: Restore the proptest, bound it to the production `compile_expr_to_bytecode` function and a new `vb_expr::bytecode::interpret_ast` helper that walks the `ExprAst` directly and returns the same `ExprResult<SlotValue>`. The proptest compares both evaluators on a generator-driven `ExprAst` with a slot resolver that binds named slots to proptest-supplied `i64` values.

**File**: `crates/vb_compile/tests/proptest_bytecode_ast_parity.rs` (NEW, ~280 lines)
- `pub fn interpret_ast(expr: &ExprAst, slots: &HashMap<String, i64>) -> ExprResult<SlotValue>` (NEW, ~50 lines, inlined in the test file behind a `#[cfg(test)]` block) — recursive interpreter for `Literal(I64)`, `Literal(Bool)`, `Literal(Null)`, `Binary(Add, Sub, Mul, Div)`, `Unary(Neg, Not)`, `Reference(name)`, `Helper("empty", args)`. Every arm returns the same `SlotValue` shape that the bytecode evaluator returns, with the same error variants (`ExprError::DivisionByZero`, `ExprError::InvalidReference`).
- `proptest!` block 1: `prop_bytecode_evaluates_to_same_as_ast_for_arbitrary_i64_expr` — generate `ExprAst` up to depth 5, using `Add/Sub/Mul/Div` and bounded integer values, bind 0–4 named slots, run both evaluators, assert equality of `SlotValue::I64`.
- `proptest!` block 2: `prop_bytecode_division_by_zero_matches_ast` — generate `Div(NonZero, Zero)`; both must return `Err(ExprError::DivisionByZero)`.
- `proptest!` block 3: `prop_bytecode_unknown_reference_matches_ast` — generate `Reference("missing")`; both must return `Err(ExprError::InvalidReference)`.
- `proptest!` block 4: `prop_bytecode_constant_folding_parity` — generate expressions with literal sub-trees that constant-fold identically; bytecode version must yield the same `ops` length as the AST version would (after folding).

**Acceptance**:
- File compiles under `cargo check -p vb_compile --tests --all-features`.
- `cargo test -p vb_compile --test proptest_bytecode_ast_parity` passes ≥256 cases per block.
- The interpreter is bounded to a small AST surface (no `Helper` beyond `empty`/`contains`/`starts_with`/`ends_with`/`has`/`sum`) and refuses `F64` literals via `prop_assume!`.
- The test file is declared in `crates/vb_compile/src/lib.rs` by uncommenting the `mod property_tests;` line AND adding `mod bytecode_ast_parity_tests;` (or by registering the test file as a `#[path]`-attribute integration test, which the `tests/` directory auto-discovers — so the lib.rs comment line is harmless either way; delete it to remove the lie).
- Delete the lie at `crates/vb_compile/src/lib.rs:64-66` (3 lines, comment only).

**Test count delta**: +4 proptest functions (≥1024 cases total).
**Risk**: LOW — `compile_expr_to_bytecode` is a pure function with a clean error type; the AST interpreter is a small recursive function. The `#[path]`-attribute test discovery means no `mod` registration is strictly required.
**Hours**: 5
**Bead ID**: `vb-cs38.2`

---

### 2.3 NEW: `taint_propagation` proptest (SHIP-BLOCKER #3)

**Defect**: `crates/vb_core/src/engine/tests/integration_taint_propagation.rs` is **2,578 lines** with **101 `#[test]` functions** and **zero `proptest!` macros**. The `vb_validate/src/type_taint_tests.rs` (2,520 lines, 147 tests) has 8 occurrences of `proptest::proptest!(...)` but they all run the same hardcoded `Just(())` strategy — they are **strateless** (no `Strategy` combinator, no input generation). The Section 38 mandate "Taint safety: Secret taint never enters finish result (at compile time)" requires a property test that for any `(taint_lhs, taint_rhs, taint_rhs) ∈ Taint³` and any binary op, the join operation is monotone in the taint lattice. None of the existing 248 hand-coded tests cover this exhaustively.

**Fix**: Write a new file that uses `proptest!` with a `proptest::collection::vec` of proptest-generated `ExprAst` values, drives the production `vb_core::taint::join_taint` and `vb_core::taint::eval_taint_for_expr` (production pub fns), and asserts:
1. **Lattice monotonicity**: `join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))` for all `a, b, c ∈ Taint`.
2. **Secret absorption**: `join_taint(Secret, x) == Secret` and `join_taint(x, Secret) == Secret` for all `x`.
3. **Clean identity**: `join_taint(Clean, x) == x` and `join_taint(x, Clean) == x` for all `x`.
4. **Compaction monotonicity**: For any list of `Taint` values, the `accumulate` op never decreases taint severity (Clean < DerivedFromSecret < Secret under the production order).
5. **End-to-end expr taint**: For any proptest-generated `ExprAst` over slots whose taint vector is proptest-generated, the taint of the result is `≥` the taint of every input slot (Secret absorption across all `Taint × Taint` pairs).

**File**: `crates/vb_core/tests/proptest_taint_propagation_section38.rs` (NEW, ~320 lines)
- `fn arb_taint() -> impl Strategy<Value = Taint>` (3 variants: Clean, DerivedFromSecret, Secret).
- `fn arb_taint_vec(max_len: usize) -> impl Strategy<Value = Vec<Taint>>`.
- `fn arb_typed_slot(max_slots: usize) -> impl Strategy<Value = Vec<(SlotValue, Taint)>>`.
- `proptest!` block 1: `prop_taint_join_associative(a in arb_taint(), b in arb_taint(), c in arb_taint())`.
- `proptest!` block 2: `prop_taint_join_commutative(a in arb_taint(), b in arb_taint())`.
- `proptest!` block 3: `prop_taint_join_secret_is_absorbing(x in arb_taint())`.
- `proptest!` block 4: `prop_taint_join_clean_is_identity(x in arb_taint())`.
- `proptest!` block 5: `prop_taint_accumulate_is_monotonic(v in arb_taint_vec(16))`.
- `proptest!` block 6: `prop_taint_eval_expr_is_at_least_max_input_taint(expr in arb_expr_ast_depth_4(), slots in arb_typed_slot(8))`.
- `proptest!` block 7: `prop_taint_finish_result_never_secret_for_clean_inputs(expr in arb_expr_ast_depth_4(), slots in arb_typed_clean_slot(8))` — for any expression over only Clean-tainted inputs, the result taint must be Clean (this is the Section 38 Taint-safety contract).

**Acceptance**:
- File compiles under `cargo check -p vb_core --tests --all-features`.
- `cargo test -p vb_core --test proptest_taint_propagation_section38` passes ≥256 cases per block.
- Every `prop_assert!` uses concrete variant matching (`Taint::Secret(_)`, `Taint::DerivedFromSecret(_)`, `Taint::Clean`).
- The proptest is bound to the production `vb_core::taint` module — no duplicate test-local reimplementation of join semantics.

**Test count delta**: +7 proptest functions (≥1,792 cases total).
**Risk**: MEDIUM — the existing hand-coded tests already exhaust the `Taint³` lattice (27 triples), so a proptest that asserts the same predicates is mostly a coverage-density win. The risk is in block 7: if production `eval_taint_for_expr` has a bug that allows Secret to leak into a Clean-only context, the proptest will fail, which is the desired outcome (this is exactly the Section 38 Taint-safety contract).
**Hours**: 6
**Bead ID**: `vb-cs38.3`

---

### 2.4 NEW: `error_recovery` property test (SHIP-BLOCKER #4)

**Defect**: `recovery_property_tests.rs` (`crates/vb_storage/tests/recovery_property_tests.rs`, 108 lines) has 7 proptests, but they only cover recovery-helper predicates (`hydrate_dimensions_positive`, `replay_step_order_diverges`, etc.) with `prop_assert!` on `bool` returns. The Section 38 mandate "error_recovery: no proptest for recovery from fuzz-malformed journal records" is unmet: there is no proptest that takes proptest-generated malformed bytes, drives `decode_journal_event` and the recovery `hydrate_*` functions, and asserts the recovery pipeline either rejects the bad bytes with the expected `JournalError` variant or continues past them with a non-corrupted state.

**Fix**: Write a proptest that generates malformed-but-typed byte sequences (truncated headers, payload-digest mismatches, payload-length overflow, unknown record kinds, bad magic, post-decode `InvalidEvent` cases) and asserts:
1. The decode function never panics.
2. The decode function returns an error in the `BadMagic | UnknownRecordKind | PayloadTooLarge | HeaderChecksumMismatch | PayloadDigestMismatch | UnexpectedEof | UnexpectedTrailingBytes | PostcardDecodeFailed | InvalidEvent | HeaderLengthMismatch | UnsupportedSchemaVersion | MigrationRequired | RecordKindFamilyMismatch` set.
3. For the **predicate-only** recovery surface (`hydrate_events_preconditions`, `hydrate_dimensions_positive`, `hydrate_snapshot_tail_seq_after_snapshot` — all `pub const fn` at `crates/vb_storage/src/recovery/hydrate.rs:63-71` and `crates/vb_storage/src/recovery/replay/core.rs`), the proptest asserts the predicates produce stable decisions across two calls on the same input (idempotence).
4. The `replay_attempt_is_current` / `replay_attempt_is_stale` predicates form a partition of the `attempt × max_attempt` space (the tests in `recovery_property_tests.rs:99-104` cover the deterministic case; the proptest covers the fuzzed case).

**File**: `crates/vb_storage/tests/proptest_error_recovery_section38.rs` (NEW, ~280 lines)
- `fn arb_malformed_bytes() -> impl Strategy<Value = Vec<u8>>` — generates 8 classes of malformed bytes (truncate, corrupt magic, corrupt header CRC, payload-length overflow, trailing bytes, postcard-deserialize-fail, valid-envelope-but-invalid-event, unknown-kind-id).
- `proptest!` block 1: `prop_decode_journal_event_never_panics(bytes in arb_malformed_bytes())`.
- `proptest!` block 2: `prop_decode_journal_event_returns_typed_error(bytes in arb_malformed_bytes())`.
- `proptest!` block 3: `prop_decode_journal_event_no_panic_for_max_payload_len(bytes in prop::collection::vec(any::<u8>(), 0..1024))`.
- `proptest!` block 4: `prop_recovery_hydrate_isolates_malformed_events(valid_prefix in arb_valid_journal_events(8), malformed in arb_malformed_bytes(), valid_suffix in arb_valid_journal_events(8))` — produces a `Vec<JournalEvent>` that interleaves valid events with semantically-invalid events (run_id=0, attempt=0, seq-overflow); calls the predicate `hydrate_events_preconditions(&combined)`; if the prefix is non-empty, asserts the predicate is stable across two calls (idempotence). Note: the actual event-stream hydration function is private to `FjallJournal::replay`; the proptest uses the public predicate surface only.
- `proptest!` block 5: `prop_decode_journal_event_idempotent_on_retry(bytes in prop::collection::vec(any::<u8>(), 1..256))` — calling `decode_journal_event` twice on the same bytes produces the same `Result<_, _>` discriminant (Ok-or-Error variant matches; for Err, the variant matches; for Ok, the events are equal).
- `proptest!` block 6: `prop_replay_attempt_monotonic_attempts(attempts in prop::collection::vec(0u16..16, 0..32))` — for any sequence of replay attempt counters, the `replay_attempt_is_current` / `replay_attempt_is_stale` predicates produce a partition (`x.is_current || x.is_stale`, never both).

**Acceptance**:
- File compiles under `cargo check -p vb_storage --tests --all-features`.
- `cargo test -p vb_storage --test proptest_error_recovery_section38` passes ≥256 cases per block.
- The malformed-bytes strategy uses the public `encode_journal_event_record` function (not raw byte concatenation) for the valid-event prefix, so the test does not smuggle implementation assumptions.
- The proptest never panics, by design and by `#[test]#[should_panic] = "expected"]` is NOT used.

**Test count delta**: +6 proptest functions (≥1,536 cases total).
**Risk**: HIGH — the public `hydrate_*` surface is a set of `pub const fn` predicates (`hydrate_events_preconditions`, `hydrate_dimensions_positive`, `hydrate_snapshot_tail_seq_after_snapshot`) at `crates/vb_storage/src/recovery/hydrate.rs:63-71`, not a full event-stream hydration entry point. The proptest exercises the predicate surface plus `decode_journal_event` directly; a deeper "stream-level hydration" proptest would need to await a future refactor that exposes a `pub fn hydrate_event_stream(bytes: &[u8]) -> Result<RecoveryFrame, JournalError>` entry point. Mitigation: this plan only writes the predicate-level proptests; the stream-level proptest is filed as a separate preconditioning sub-bead (`vb-cs38.4.1`) and tracked in the same parent. The 4 decode-side proptests (decode-never-panics, decode-typed-error, idempotent, replay-attempt) are LOW risk and can ship independently.
**Hours**: 6
**Bead ID**: `vb-cs38.4`

---

### 2.5 STRENGTHEN: `digest_stability` alias → broader scope (alias #1)

**Defect**: `crates/vb_compile/tests/proptest_digest_determinism.rs` only fuzzes the Ask variant. The Section 38 mandate "Digest stability: Same input produces same compiled digest" applies to **all** `StepPrimitive` variants, not just Ask.

**Fix**: Add 5 new proptest functions to the existing file:
- `prop_digest_determinism_for_set_steps` — generate `StepPrimitive::Set` with random `output` strings and `value` literals.
- `prop_digest_determinism_for_choose_steps` — generate `Choose` with 2–8 random branches.
- `prop_digest_determinism_for_together_steps` — generate `Together` with 1–4 children.
- `prop_digest_determinism_for_foreach_steps` — generate `ForEach` with random body.
- `prop_digest_determinism_for_reduce_steps` — generate `Reduce` with random body.

**File**: append to `crates/vb_compile/tests/proptest_digest_determinism.rs` (existing, +200 lines, no new file).
**Acceptance**: Each new proptest ≥256 cases, all pass, all bound to `vb_compile::canonical_digest`.
**Test count delta**: +5 proptest functions.
**Risk**: LOW — the existing `workflow_source_strategy()` helper generates an `Ask` step; the new strategies are siblings. The determinism predicate is the same (`canonical_digest(s1) == canonical_digest(s2)`).
**Hours**: 2
**Bead ID**: `vb-cs38.5`

---

### 2.6 STRENGTHEN: `for_each_ordering` alias → add proptest lane (alias #2)

**Defect**: `for_each_ordering` is currently only `kani_for_each_ordering` at `crates/vb_runtime/src/verification/kani/kani_for_each_ordering.rs:46` (Kani-only, cfg-kani). Section 38 requires property tests, not just Kani.

**Fix**: Add a runtime proptest that:
1. Builds a `ValueStore` and a `RunFrame` with N items (1 ≤ N ≤ 32).
2. Calls `for_each_start` → loops calling `for_each_next` until `for_each_join` returns a Done signal.
3. Records the sequence of `item_slot` writes (each `for_each_next` binds one item to the item slot).
4. Asserts the recorded sequence equals the original input list in order.

**File**: `crates/vb_runtime/tests/proptest_for_each_ordering_section38.rs` (NEW, ~180 lines)
- `fn arb_input_list(max_len: u8) -> impl Strategy<Value = Vec<SlotValue>>` — uses `proptest::collection::vec(arb_slot_value(), 0..=max_len as usize)`.
- `proptest!` block 1: `prop_for_each_emits_items_in_input_order(items in arb_input_list(32))`.
- `proptest!` block 2: `prop_for_each_empty_input_jumps_to_done` (deterministic, no fuzz input).
- `proptest!` block 3: `prop_for_each_one_input_emits_exactly_one_iteration` (deterministic, edge).
- `proptest!` block 4: `prop_for_each_max_input_under_fanout_limit` (deterministic, edge at the 256-item fanout boundary).

**Acceptance**: ≥256 cases per proptest, all pass, all bound to production `for_each_start`/`for_each_next`/`for_each_join` (not a duplicate reimplementation).
**Test count delta**: +4 proptest functions.
**Risk**: LOW — the Kani harness already proves the function-level invariant; the proptest is the runtime, multi-iteration witness.
**Hours**: 2
**Bead ID**: `vb-cs38.6`

---

### 2.7 STRENGTHEN: `resource_budget` alias → add retry/time-limit lanes (alias #3)

**Defect**: `crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs` (617 lines) has 12 proptests for ActionTicket attempt-fence invariants but does not cover: (a) **retry-attempt ceiling** under fuzzed `RetryPolicy::max_attempts`, (b) **time-limit** budget arithmetic (`base_delay_ms` × `exponential_backoff`), (c) **per-step budget** arithmetic (the `StepBudget::try_take` domain contract). Section 38 requires "Bound enforcement: Retry attempts never exceed limit; collect never exceeds page/item/time limits."

**Fix**: Add 4 proptest blocks to the same file (no new file, +180 lines):
- `prop_retry_attempt_never_exceeds_max_attempts(attempts in 0u32..u32::MAX, max_attempts in 0u16..64)`.
- `prop_retry_attempt_count_equals_calls_under_normal_completion(attempt_count in 0u16..16)`.
- `prop_step_budget_try_take_never_underflows(initial in 0u32..u32::MAX, takes in 0u32..u32::MAX)` — bound via `prop_assume!` to safe arithmetic (initial ≤ u32::MAX / 2, takes ≤ initial + 1).
- `prop_retry_policy_base_delay_under_fuzz(base_ms in 0u32..100_000, exponential in any::<bool>())` — assert `policy.compute_delay(attempt)` is bounded by `base_ms * 2^max_attempts` (or similar production formula).

**Acceptance**: ≥256 cases per proptest, all pass, all bound to production `crate::engine::RetryPolicy` and `vb_core::step_budget::StepBudget`.
**Test count delta**: +4 proptest functions.
**Risk**: MEDIUM — the exact `StepBudget::try_take` API surface must be verified; if the function signature differs from the plan assumption, the proptest is adjusted at the start of the bead (no production change).
**Hours**: 3
**Bead ID**: `vb-cs38.7`

---

### 2.8 STRENGTHEN: `bound_enforcement` alias → add collect-page/time limits (alias #4)

**Defect**: `crates/vb_core/src/workflow/proptest_workflow.rs` (523 lines) has 10 proptests for workflow validation bounds, but only at `CompiledWorkflow::try_from_parts` time. The Section 38 "collect never exceeds page/item/time limits" requires runtime enforcement tests, not just validation-time. The `proptest_collect_budget.rs` and `proptest_collect_traversal.rs` files in `crates/vb_core/src/workflow/` cover collect-budget arithmetic, but no proptest exercises the **runtime collect primitive** (`vb_runtime::primitives::collect::collect_page`) under fuzzed `(page_size, item_count, time_budget_ms)` tuples.

**Fix**: Add 3 proptest blocks to the existing `crates/vb_core/src/workflow/proptest_collect_budget.rs` file (no new file, +150 lines):
- `prop_collect_page_never_returns_more_than_page_size(page_size in 1usize..64, item_count in 0usize..1024)` — call `collect_page` with the generated parameters, assert returned Vec length ≤ `page_size`.
- `prop_collect_page_returns_correct_count_when_items_fit(page_size in 1usize..64, item_count in 0usize..64)` — assert returned Vec length == `min(page_size, item_count)`.
- `prop_collect_page_total_over_pages_equals_input_size(page_size in 1usize..32, item_count in 0usize..256)` — drive the collect-page loop until done, count total returned items, assert it equals `item_count`.

**Acceptance**: ≥256 cases per proptest, all pass, all bound to production `vb_runtime::primitives::collect::collect_page` (or its public re-export).
**Test count delta**: +3 proptest functions.
**Risk**: MEDIUM — `collect_page` may have a different signature than the plan assumes; verify before writing the proptest. If collect-page is still under construction, file a preconditioning note and ship the validation-time proptest extensions first.
**Hours**: 2
**Bead ID**: `vb-cs38.8`

---

### 2.9 NEW: `layout_stability` property test (alias #5, MISSING)

**Defect**: No file in the tree matches `layout_stability` or contains the `layout` proptest strategy. Section 38 requires "Layout stability: Slot layout and accessor layout stable for same workflow."

**Fix**: Add a proptest that, for any `WorkflowParts` with proptest-generated slot/step indices, the `CompiledWorkflow::try_from_parts` produces the same `SlotIdx → ConstIdx` mapping across two calls. This is essentially a "compiled-workflow is deterministic" property.

**File**: `crates/vb_core/tests/proptest_layout_stability_section38.rs` (NEW, ~200 lines)
- `fn arb_workflow_parts(max_nodes: u16) -> impl Strategy<Value = WorkflowParts>` — generates 1–N compiled nodes with bounded `SlotIdx` (0–64) and `StepIdx` (0–N).
- `proptest!` block 1: `prop_compiled_workflow_digest_matches_for_identical_parts(parts in arb_workflow_parts(32))` — call `CompiledWorkflow::try_from_parts(parts.clone()).unwrap()` twice, assert their `WorkflowDigest` is byte-equal.
- `proptest!` block 2: `prop_compiled_workflow_slot_layout_matches_for_identical_parts(parts in arb_workflow_parts(32))` — assert the `node[0].output` slot indices match across two compiles.
- `proptest!` block 3: `prop_compiled_workflow_node_count_matches_input(parts in arb_workflow_parts(32))` — assert `nodes.len() == parts.nodes.len()` (no compaction, no expansion).
- `proptest!` block 4: `prop_compiled_workflow_step_index_preserved(parts in arb_workflow_parts(32))` — for each node, `node.id == parts.nodes[i].id` after a successful compile.

**Acceptance**: ≥256 cases per proptest, all pass, all bound to production `CompiledWorkflow::try_from_parts` and `WorkflowDigest`.
**Test count delta**: +4 proptest functions.
**Risk**: LOW — `try_from_parts` is a pure function over `WorkflowParts`; the determinism property is what the existing `proptest_digest_determinism.rs` file already tests for the **canonical** digest, but this file tests the **per-compile** digest.
**Hours**: 2
**Bead ID**: `vb-cs38.9`

---

## 3. Coverage Evidence to Replace

### 3.1 Replace the stub `tarpaulin-report.json` and add a real `lcov.info`

**Defect**: `tarpaulin-report.json` at the repo root is `{}` (3 bytes). `coverage.log` and `llvm-cov.log` do not exist in the tree at all (they were referenced in the task description but `find` returns zero results). The `moon run :coverage` task at `.moon/tasks/all.yml:429-447` runs `cargo llvm-cov test --quiet --workspace --all-targets --all-features --lcov --output-path target/llvm-cov/lcov.info -- action::tests::validate_action_outcome_failed_always_succeeds` — this filters to **one** test by `--` argument and writes only a single test's coverage into `lcov.info`, producing a thin report that satisfies the "coverage wiring" gate but does not cover the workspace.

**Fix**: Two changes:
1. **Delete the stub**: Remove `tarpaulin-report.json` (it is gitignored but `find` shows it still in tree).
2. **Wire a real coverage task**: Replace the smoke `coverage` task with a **workspace-wide** `llvm-cov` run. Add `--branch-coverage` to the `cargo llvm-cov` invocation (this is the flag that fixes the "0 branches" finding). Use `--no-fail-fast` and a 30-minute timeout.

**File**: edit `.moon/tasks/all.yml` lines 429-447 (existing `coverage` task).
**New task body**:
```yaml
coverage:
  script: |
    set -euo pipefail
    mkdir -p target/llvm-cov
    mkdir -p target/moon-locks
    mkdir -p target/tmp
    flock --shared target/moon-locks/source-mutation.lock env TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= RUSTFLAGS="-Dwarnings -Cinstrument-coverage" timeout 30m rustup run nightly-2026-04-28 cargo llvm-cov test --no-fail-fast --workspace --all-targets --all-features --branch-coverage --lcov --output-path target/llvm-cov/lcov.info --html --output-dir target/llvm-cov/html
    # Reject the report if it is suspiciously small (< 100 KB) or missing the BRDA records.
    test -s target/llvm-cov/lcov.info
    report_bytes=$(stat -c %s target/llvm-cov/lcov.info)
    if [ "$report_bytes" -lt 102400 ]; then
      echo "ERROR: lcov.info is $report_bytes bytes; expected ≥ 102400 (100 KB)" >&2
      exit 1
    fi
    if ! grep -q '^BRDA:' target/llvm-cov/lcov.info; then
      echo "ERROR: lcov.info has no BRDA records; branch coverage flag did not take effect" >&2
      exit 1
    fi
```

**Acceptance**:
- `moon run :coverage` produces a real `target/llvm-cov/lcov.info` ≥ 100 KB with both `DA:` (line) and `BRDA:` (branch) records.
- The report is HTML-browsable at `target/llvm-cov/html/index.html`.
- The `tarpaulin-report.json` is removed (`rm -f tarpaulin-report.json`).
- A new `evidence/coverage/lcov-info-summary.txt` is generated by the task and committed:
  ```
  cargo llvm-cov report --lcov --output-path target/llvm-cov/lcov.info
  cargo llvm-cov report --summary-only > evidence/coverage/llvm-cov-summary.txt
  ```

**Test count delta**: N/A (this is infrastructure, not tests).
**Risk**: HIGH — a workspace-wide `cargo llvm-cov test` may take 30–45 minutes on this codebase, and the RUSTFLAGS `-Cinstrument-coverage` may collide with existing compiler flags used by other Moon tasks. Mitigation: use `flock` on `target/moon-locks/source-mutation.lock` (already in place) to prevent parallel runs, and use a separate `CARGO_TARGET_DIR` (`target/llvm-cov-target`) to avoid invalidating the regular test target dir.
**Hours**: 4
**Bead ID**: `vb-cs38.10`

---

### 3.2 Add a test-density gate Moon task

**Defect**: The 5x test density requirement is documented in `crates/vb_ipc/test-suite-review.md:9` ("Density audit (718 tests / 143 functions = 5.0x — target ≥5x)") and in `BIG-ASS-TESTING-TO-FIX.md:198` ("density audit Tier 0 not automated in CI"). No Moon task enforces it; the `5.0x` value is computed by the test-reviewer agent ad hoc, not by a reproducible script.

**Fix**: Add a new Moon task `test-density-gate` that runs a small Rust binary counting `#[test]` functions vs. `pub fn` declarations per crate, computes the density ratio, and fails if any crate falls below 5.0x.

**File**: NEW `scripts/check-test-density.rs` (~150 lines) + new task in `.moon/tasks/all.yml`.
**New task body**:
```yaml
test-density-gate:
  script: |
    set -euo pipefail
    mkdir -p target/gate-tools
    rustc --edition=2024 scripts/check-test-density.rs -o target/gate-tools/check-test-density
    target/gate-tools/check-test-density --workspace --min-density 5.0 --fail-below
  toolchains:
    - rust
  inputs:
    - '@globs(sources)'
    - '@globs(tests)'
    - 'scripts/check-test-density.rs'
    - '.moon/tasks/all.yml'
  options:
    cache: false
    runInCI: true
```

**Algorithm for `check-test-density.rs`**:
1. For each crate in `crates/`, walk `src/**/*.rs` and `tests/**/*.rs`.
2. Count `pub fn` declarations (regex: `^\s*pub\s+(?:async\s+)?fn\s+\w+`).
3. Count `#[test]` and `#[tokio::test]` declarations.
4. Compute `tests / pub_fns` per crate.
5. Exit with non-zero status and print a table for any crate below 5.0x.

**Insert into `.moon.yml` pipeline** (after `lint-src`):
```yaml
pipeline:
  - 'fmt'
  - 'lint-src'
  - 'test-density-gate'   # NEW
  - 'check'
  ...
```

**Acceptance**:
- `moon run :test-density-gate` exits 0 on the current tree (assuming density ≥ 5x) or reports a clear table for any deficient crate.
- The script writes a per-crate density ledger to `target/gate-tools/test-density.csv` for archival.
- A pre-existing helper at `crates/vb_yaml/src/profile_error_variants_tests.rs:7` already documents the 5x target; the new script enforces it.

**Test count delta**: N/A (infrastructure).
**Risk**: LOW — the script is a small file walker; the regex may miss edge cases (e.g., `pub(crate) fn`, `pub(super) fn`) and require iteration. Mitigation: count `pub fn`, `pub(crate) fn`, `pub(super) fn`, and `pub(in path) fn` together.
**Hours**: 3
**Bead ID**: `vb-cs38.11`

---

### 3.3 Fix branch coverage collection

**Defect**: `vb_ipc/test-suite-review.md:9` reports "Branch coverage: not collected (cargo-llvm-cov reported 0 branches for all files)" and `vb_compile/test-suite-review.md:132` says "cargo llvm-cov reports 0 branches for all files. The toolchain or build configuration is not emitting branch coverage data." The cause is missing `--branch-coverage` on the `cargo llvm-cov` invocation, plus a stale RUSTFLAGS that does not request branch instrumentation.

**Fix**: Already covered by §3.1 above (the new `coverage` task passes `--branch-coverage` and includes a self-test `grep '^BRDA:'` to fail-fast if no branch records are produced). No separate work item is needed; this is a sub-deliverable of `vb-cs38.10`.

**Acceptance**: see §3.1.
**Hours**: 0 (rolled into vb-cs38.10).
**Bead ID**: covered by `vb-cs38.10`.

---

## 4. Per-Item Summary Table

| # | Bead | Defect | Fix | Acceptance | Test Δ | Hours |
|---|------|--------|-----|------------|--------|-------|
| 1 | `vb-cs38.1` | No proptest for production Mutex paths | New `proptest_concurrency_safety.rs` with 3 blocks | Compiles, ≥256 cases/block, all bound to production `JournalWriterQueue` / `BoundedActionCompletionQueue` / `FjallJournal` | +3 proptests | 4 |
| 2 | `vb-cs38.2` | `bytecode_ast_parity.rs` does not exist; comment is a lie | New `proptest_bytecode_ast_parity.rs` with 4 blocks + interpreter + delete comment | Compiles, ≥256 cases/block, delete `crates/vb_compile/src/lib.rs:64-66` | +4 proptests | 5 |
| 3 | `vb-cs38.3` | 2,578 lines of `#[test]` + 0 proptest in taint | New `proptest_taint_propagation_section38.rs` with 7 blocks | Compiles, ≥256 cases/block, all bound to production `vb_core::taint` | +7 proptests | 6 |
| 4 | `vb-cs38.4` | No proptest for fuzz-malformed journal records | New `proptest_error_recovery_section38.rs` with 6 blocks | Compiles, ≥256 cases/block, never panics, recovery-isolates-malformed invariant | +6 proptests | 6 |
| 5 | `vb-cs38.5` | `digest_stability` only covers Ask | Append 5 blocks to `proptest_digest_determinism.rs` | All primitive variants covered | +5 proptests | 2 |
| 6 | `vb-cs38.6` | `for_each_ordering` is Kani-only | New `proptest_for_each_ordering_section38.rs` with 4 blocks | Runtime proptest, not just Kani | +4 proptests | 2 |
| 7 | `vb-cs38.7` | `resource_budget` missing retry/time-limit lanes | Append 4 blocks to `proptest_attempt_fence.rs` | Retry ceiling + time-limit + step-budget covered | +4 proptests | 3 |
| 8 | `vb-cs38.8` | `bound_enforcement` is validation-time only | Append 3 blocks to `proptest_collect_budget.rs` | Runtime collect-page bounds covered | +3 proptests | 2 |
| 9 | `vb-cs38.9` | `layout_stability` MISSING entirely | New `proptest_layout_stability_section38.rs` with 4 blocks | Compiled-workflow determinism + slot layout + node count | +4 proptests | 2 |
| 10 | `vb-cs38.10` | Stub `tarpaulin-report.json` + smoke `coverage` task | Replace `coverage` task with workspace-wide `--branch-coverage` run; size assertion; HTML report | `lcov.info` ≥ 100 KB with `BRDA:` records | N/A | 4 |
| 11 | `vb-cs38.11` | 5x test density not in CI | New `scripts/check-test-density.rs` + `test-density-gate` Moon task + pipeline entry | Script exits 0 when all crates ≥ 5.0x; CSV ledger written | N/A | 3 |
| 12 | (rolled into #10) | Branch coverage broken | `--branch-coverage` flag + self-test | `BRDA:` records present in `lcov.info` | N/A | 0 |

**Total test count delta**: +40 proptest functions (≥ 10,240 cases total at default `ProptestConfig::with_cases(256)`).
**Total work hours**: 4 + 5 + 6 + 6 + 2 + 2 + 3 + 2 + 2 + 4 + 3 = **41 hours** (~5.13 engineer-days).
**Total bead count**: 11 (10 active + 1 rolled-in).

---

## 5. Dependency Graph and Execution Order

```
vb-cs38.1 ─┐
vb-cs38.2 ─┤
vb-cs38.3 ─┤
vb-cs38.4 ─┤
vb-cs38.5 ─┼──→  (all independent; can parallelize across polecats)
vb-cs38.6 ─┤
vb-cs38.7 ─┤
vb-cs38.8 ─┤
vb-cs38.9 ─┤
           │
vb-cs38.10 ─┤  (independent of tests; depends on `llvm-cov` toolchain)
vb-cs38.11 ─┘  (independent of tests; depends on no concurrent Moon run)
```

All 11 beads are mutually independent. The fastest execution path is one engineer-day for `vb-cs38.10` + `vb-cs38.11` (parallel), three engineer-days for the four SHIP-BLOCKER proptests (`vb-cs38.1` through `vb-cs38.4`, one per day), and one engineer-day for the five alias strengthenings (`vb-cs38.5` through `vb-cs38.9`, two per half-day).

---

## 6. Definition of Done

The Section 38 property-test gap remediation is **DONE** when **all** of the following are true:

| # | Criterion | Verification command | Expected |
|---|-----------|----------------------|----------|
| 1 | All 4 SHIP-BLOCKER proptests exist and pass | `cargo test --workspace --tests --all-features -- 'proptest_concurrency_safety' 'proptest_bytecode_ast_parity' 'proptest_taint_propagation_section38' 'proptest_error_recovery_section38'` | 4 files, 0 failures, ≥ 256 cases each |
| 2 | All 5 alias strengthenings land | `cargo test --workspace --tests --all-features -- 'proptest_digest_determinism' 'proptest_for_each_ordering_section38' 'proptest_attempt_fence' 'proptest_collect_budget' 'proptest_layout_stability_section38'` | New functions present, all pass |
| 3 | The `// TEMPORARILY DISABLED` comment is removed | `rg -n "TEMPORARILY DISABLED" crates/vb_compile/src/lib.rs` | 0 matches |
| 4 | `tarpaulin-report.json` no longer in tree | `find . -name "tarpaulin-report.json" -not -path "*/target/*"` | 0 matches |
| 5 | Real `lcov.info` is produced | `moon run :coverage && stat -c %s target/llvm-cov/lcov.info` | ≥ 102400 bytes |
| 6 | `lcov.info` contains branch records | `grep -c '^BRDA:' target/llvm-cov/lcov.info` | > 0 |
| 7 | Test-density gate runs in CI | `moon run :test-density-gate` | Exit 0 (or 1 with table) |
| 8 | `moon ci` pipeline includes the new task | `grep -A 1 'test-density-gate' .moon.yml` | Present |
| 9 | Test count delta verified | `cargo test --workspace --no-run 2>&1 \| rg -c 'Running unittests'` then divide by crate | +40 proptests, total ≥ 1,300 `#[test]` functions |
| 10 | All 11 beads closed in bd | `bd list --status closed --limit 0 --json \| jq '.[] \| select(.id \| startswith("vb-cs38"))'` | 11 closed |
| 11 | Section 38 property test count | `rg -l 'proptest!' crates/*/tests/*.rs crates/*/src/**/*.rs 2>/dev/null \| wc -l` | ≥ 12 files (currently 0 of the new ones) |
| 12 | Truth-serum audit | `bash scripts/check-vb-jpq7-closure-evidence.py --parent vb-cs38` | PASS |

---

## 7. Risk Register and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Proptest 38.1 (concurrency) flakes under load | Test reliability | Use `Arc<>` (not `Box<>`), join all threads before asserting, fork-disabled mode, run with `--test-threads=1` in CI |
| Proptest 38.2 (bytecode/AST) requires writing a tiny interpreter | Scope creep | Limit interpreter to I64/Bool/Null + Add/Sub/Mul/Div/Neg/Not/Ref/"empty" helper; cover F64 with `prop_assume!` to skip |
| Proptest 38.3 (taint) exposes a real production bug | Bead cost overrun | If the test fails, that is exactly the Section 38 contract violation it is meant to detect. Open a follow-up bead for the production fix; do not weaken the test. |
| Proptest 38.4 (error recovery) API mismatch | Plan revision | Read `crates/vb_storage/src/recovery/hydrate.rs` first; if signatures differ, file a preconditioning sub-bead and ship the 4 encode-side proptests first. |
| `cargo llvm-cov` 30-min timeout | CI budget | Cache `target/llvm-cov-target` between runs; use `--no-fail-fast` so a single test failure does not invalidate the report |
| Test-density gate false-positives on crates with few `pub fn` (e.g., `vb_doc`) | Test brittleness | Allow per-crate overrides via a `scripts/test-density.allow` file; default threshold 5.0x but allow 3.0x for crates with < 20 `pub fn` |
| Branch coverage on crates with no conditionals (e.g., type-only modules) | False FAIL | The grep `^BRDA:` check is at the file level; a crate may legitimately have 0 branches in some files. Mitigation: check `^BRDA:` count > 100 globally, not per-file. |

---

## 8. Out-of-Scope (Explicitly Not Addressed)

These Round 4 findings are out of scope for this plan and remain in their existing beads:

- **Section 65 taxonomy migration** (`vb-yfveq`) — already in flight, P0 open.
- **trybuild silent-pass** (resolved under `vb-j58jl`).
- **ArrayQueue vs crossbeam_channel** (Section 50) — needs separate P0 fix.
- **tick_shard API missing** (Section 30) — separate P0.
- **ui/evaluate/benchmark CLI commands** (Section 33) — separate P0.
- **Section 39 benchmarks** (24/40 missing) — separate bulk-fix plan.
- **densify audit on `vb_ui_model`** (1.9x) — separate, post-merge.

This plan is **only** the Section 38 property-test gap remediation and the coverage-evidence fix surface.
