# vb-eirmk.13.3 Contract: Split replay/ops/tests.rs by category

**Bead:** vb-eirmk.13.3
**Status:** implementation
**Target file:** `crates/vb_core/src/replay/ops/tests.rs` (2702 lines)

## Problem

`crates/vb_core/src/replay/ops/tests.rs` is 2702 lines — well past the Holzman Rust source-length budget and approaching the 1500-line ceiling for test files. This increases review cost, compile time, and makes test discovery harder.

## Plan

Split the 2702-line `tests.rs` into four files:

1. **`tests.rs`** (short shell) — keeps imports, helpers, and `include!` macros for the three chunk files. Target: ~50 lines.
2. **`tests_stack.rs`** — ReplayExprStack lifecycle tests + stack helper tests (pop_pair, pop_i64_pair) + eval arithmetic (add, sub, mul, div) + comparisons (gt, lt, gte, lte) + equality/inequality + logic (and, or, not). Target: ~900 lines.
3. **`tests_expr.rs`** — eval slot/const/accessor load + eval helper ops + eval_expr_for_replay + blackhat arithmetic boundaries + blackhat taint + stack overflow + blackhat helper engine match tests + f64 mixed tests. Target: ~1150 lines.
4. **`tests_ce003.rs`** — ce003 taint join tests + blackhat_helper_unique_rejects_non_list. Target: ~400 lines.

All four files are included via `include!` from `tests.rs`, so all functions live in the same lexical module with shared imports and helper functions.

## Mechanical Behavior Preservation

- **Exact code text preserved:** Every test, helper, assertion, attribute, and comment is relocated verbatim. No semantic cleanup or reformatting.
- **Same lexical namespace:** All tests remain in the same `tests` module via `include!`. No path changes.
- **Same module structure:** `#[cfg(test)] mod tests` in `ops.rs` continues to include the same module name.
- **No suppressions:** Do not add `#[allow(...)]` or `#[expect(...)]` attributes.
- **No dependencies or gate changes:** Cargo.toml and CI config untouched.
- **Common imports and helpers stay in tests.rs:** All shared imports, make_plan, make_expr_program, make_frame, make_minimal_plan_with_constants, replay_err_to_core, eval_stack_helper, check_helper_type_error, insert_text, insert_numbers, and f64_value remain in the main file, not duplicated.

## Test Inventory Verification

Capture a sorted inventory of all `#[test]` function names BEFORE and AFTER the split:

```bash
# Before
grep -oP '^\s*fn\s+\K[a-z_0-9]+(?=\(\))' tests.rs | sort > /tmp/vb13-3-before-tests.txt

# After — collect from all four files
{ grep -oP '^\s*fn\s+\K[a-z_0-9]+(?=\(\))' tests_stack.rs;
  grep -oP '^\s*fn\s+\K[a-z_0-9]+(?=\(\))' tests_expr.rs;
  grep -oP '^\s*fn\s+\K[a-z_0-9]+(?=\(\))' tests_ce003.rs; } | sort > /tmp/vb13-3-after-tests.txt

diff /tmp/vb13-3-before-tests.txt /tmp/vb13-3-after-tests.txt
```

The diff must be empty: exact same test function names and count.

## Rustfmt

Run `rustfmt` only on the four touched files using the pinned toolchain. Do not reformat the whole repository.

## Acceptance Criteria

1. All four files exist with correct content.
2. Tests compile: `cargo test --package vb_core --lib --no-run`
3. Tests pass: `cargo test --package vb_core --lib -- replay::ops`
4. Test inventory identical before/after (empty diff).
5. Each chunk file is under 1400 physical lines.
6. Parent agent independently compares test inventory and runs nextest + source gate.

## Non-goals

- Do not refactor or improve any test logic.
- Do not add, remove, or rename any tests.
- Do not change imports beyond what splitting requires.
- Do not run or modify other test modules.
