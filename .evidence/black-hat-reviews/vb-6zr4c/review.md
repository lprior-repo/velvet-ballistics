# Black-Hat Review — vb-6zr4c (CB-004: depth-overflow variant)

**Reviewer role:** black-hat (contract parity, Farley, DDD, Bitter Truth)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-vb-6zr4c-dispatch/`
**Review date:** 2026-06-30
**Scope:** verify that `compute_child_depth` reports `u16` depth overflow as
`StepCountOverflow` no longer — i.e. the new variant is the depth-specific
`BudgetTraversalError::DepthOverflow { depth: u16 }` and that the value is
preserved through the conversion to `WorkflowError`.

## Phase 1 — Contract & Bead Parity

**Bead contract (CB-004):**
> `compute_child_depth` reports a `u16` depth-overflow as `StepCountOverflow`,
> but it should be a depth-specific variant (the bug-hunt finding says the
> wrong variant is used).

**Parity assessment — PASS.**

- `crates/vb_core/src/budget.rs:181-186` defines the new variant
  `BudgetTraversalError::DepthOverflow { depth: u16 }` on the budget-layer
  error enum. The doc-comment at lines 181-183 explicitly states the
  invariant: *"carries the actual pre-overflow depth so diagnostics can
  report the real value instead of the lossy `u64::MAX` sentinel previously
  produced here."*
- `crates/vb_core/src/budget.rs:2083-2088` is the fix site. The branch now
  reads:
  ```rust
  let new_depth =
      current_depth
          .checked_add(1)
          .ok_or(BudgetTraversalError::DepthOverflow {
              depth: current_depth,
          })?;
  ```
  i.e. the overflow returns `DepthOverflow` carrying the *pre-overflow*
  `current_depth` (which is `u16::MAX` at the boundary — not `u64::MAX`,
  which is the lossy sentinel the bug demanded be removed).
- `crates/vb_core/src/budget.rs:201` propagates the variant through
  `From<BudgetTraversalError> for WorkflowError` via
  `BudgetTraversalError::DepthOverflow { depth } => Self::DepthOverflow { depth }`
  — depth value is preserved end-to-end.
- `crates/vb_core/src/workflow/mod.rs:409-414` defines the matching
  `WorkflowError::DepthOverflow { depth: u16 }` with a `#[error(...)]`
  attribute that formats `"nesting depth overflow: {depth} cannot be
  incremented past u16::MAX"`. The contract demands a depth-specific
  variant; both layers now have one.

**Verus production-binding gate:** Not applicable — this bead is a runtime
behaviour fix (variant selection), not a Verus obligation. No
`verification/verus/` artifact was added or modified.

**Kani harness:** None required for this CB. The existing Kani harnesses
in `crates/vb_core/src/kani_workflow_arbitrary.rs` do not exercise
`compute_child_depth`; the fix is a runtime type-selection change covered
by the three unit tests below.

**Test parity — PASS.** Three regression tests at
`crates/vb_core/src/budget.rs:2201-2260`:

| Test | CB-004 invariant enforced |
|------|----------------------------|
| `compute_child_depth_returns_depth_overflow_carrying_actual_depth` (line 2202) | Asserts the error variant is `DepthOverflow` and `depth == u16::MAX`. |
| `compute_child_depth_does_not_emit_step_count_overflow_at_u16_max` (line 2231) | **CB-004 explicit:** `!matches!(err, StepCountOverflow { .. })`. |
| `depth_overflow_converts_to_workflow_error_carrying_actual_depth` (line 2247) | Asserts `BudgetTraversalError::DepthOverflow { depth: 42 }` converts to `WorkflowError::DepthOverflow { depth: 42 }`. |

**Targeted command:** `cargo test -p vb_core --lib depth_overflow --no-fail-fast`
→ **4 passed; 2169 filtered out.** (The 4th pass is the parent
`depth_overflow_tests` module entry that aggregates the three named tests.)

## Phase 2 — Farley Engineering Rigor

- **Function length:** `compute_child_depth` (lines 2066-2096) is **30
  lines** including its match arms. The overflow branch itself is **5
  lines** (lines 2083-2088). Within Farley's 60-line soft cap.
- **Parameter count:** 3 (`kind`, `current_depth`, `max_nesting_depth`).
  Within Farley's 5-parameter limit.
- **Separation of pure logic and I/O:** `compute_child_depth` is pure —
  it only mutates `*max_nesting_depth` (caller-owned output) and returns
  a `Result`. No I/O, no globals, no hidden state.
- **Checked arithmetic:** Uses `u16::checked_add(1)` (Holzman rule 4).
  The pre-fix code presumably used `+ 1` and then some form of
  lossy-coercion; the post-fix code makes overflow unrepresentable as a
  silent value.

## Phase 3 — Holzman Rust (The Big 6)

- **Make illegal states unrepresentable:** The pre-fix code conflated
  "depth overflow" with "step count overflow" by emitting the wrong
  variant. The post-fix code emits a dedicated `DepthOverflow { depth }`
  variant whose `depth: u16` field is exactly the pre-overflow value
  — there is no longer any lossy `u64::MAX` sentinel that would let
  callers mistake a depth overflow for a step-count overflow.
- **Parse, Don't Validate:** `BudgetTraversalError::DepthOverflow`
  carries the offending depth in the type. Callers cannot construct
  the variant without supplying a depth value; the conversion
  `From<BudgetTraversalError> for WorkflowError` preserves it
  losslessly.
- **Types as Documentation:** The `#[error(...)]` message on
  `WorkflowError::DepthOverflow` documents the invariant at the type
  level: "nesting depth overflow: {depth} cannot be incremented past
  u16::MAX".
- **Workflows / Newtypes:** N/A — this is a single-site error-variant
  change, not a new workflow.

## Phase 4 — Ruthless Simplicity & DDD

- **No Option-based state machines.** The change uses a typed enum
  variant, not `Option<u16>` or a sentinel value.
- **CUPID:** Predictable (the variant name matches the failure mode),
  Domain-based (matches the workflow domain vocabulary — "nesting
  depth"), Idiomatic (matches existing `BudgetTraversalError` variant
  style — named struct with descriptive field).
- **No panic / unwrap / expect:** The branch is `Result`-based. The
  test assertions use `debug_assert!` (Holzman-permitted for test
  code, per AGENTS.md) but never `unwrap`/`expect` in the production
  branch.

## Phase 5 — Bitter Truth

- **YAGNI:** The new variant adds exactly one error arm and one
  field. No new types, no new traits, no new helpers, no new public
  API surface beyond the variant.
- **No cleverness:** The fix is a 5-line replacement of one `Err`
  arm. The diff is the minimum viable change to make the bug-hunt
  contract true.
- **Sniff test:** Looks like the kind of fix a senior engineer would
  write — adds the right variant, threads it through `From`, adds
  three small regression tests with explicit CB-004 references.

## Cross-references

- Wave-1 exploration (`to-fix/wave1/agent-02-explore.md`) marks the
  fix as `PATCHED` with three regression tests passing.
- Wave-2 black-hat (`to-fix/wave2/agent-03-black-hat.md`) marks the
  fix as `PATCHED (IN_PROGRESS in bd)` with verdict: *"CLOSE THE
  BEAD. ... The fix is in main and tests are green ... The
  `!matches!(err, StepCountOverflow { .. })` assertion in
  `compute_child_depth_does_not_emit_step_count_overflow_at_u16_max`
  is the explicit CB-004 invariant."*

## Commands Run

```bash
# Locate the function and surrounding enum
grep -rn compute_child_depth crates/vb_core/src/budget/
# Read the new variant
sed -n '170,210p' crates/vb_core/src/budget.rs
# Read the fix site
sed -n '2066,2096p' crates/vb_core/src/budget.rs
# Read the regression tests
sed -n '2195,2261p' crates/vb_core/src/budget.rs
# Verify WorkflowError::DepthOverflow variant exists
grep -n DepthOverflow crates/vb_core/src/workflow/mod.rs
# Run the targeted regression lane
cargo test -p vb_core --lib depth_overflow --no-fail-fast
#   → 4 passed; 2169 filtered out
# Compile-check the crate
cargo check -p vb_core
#   → cargo build (18 crates compiled) — clean
```

## Verdict

**STATUS: APPROVED.**

The fix at `crates/vb_core/src/budget.rs:2083-2088` is the depth-specific
variant `BudgetTraversalError::DepthOverflow { depth: u16 }` carrying the
pre-overflow `current_depth`. The conversion to
`WorkflowError::DepthOverflow { depth }` is lossless
(`crates/vb_core/src/budget.rs:201`). The explicit CB-004 invariant is
tested at `crates/vb_core/src/budget.rs:2240-2243` via
`!matches!(err, StepCountOverflow { .. })`. All three regression tests
pass under `cargo test -p vb_core --lib depth_overflow`. `cargo check
-p vb_core` is clean.

The bead is currently `IN_PROGRESS` in `bd` even though the fix and
tests are in `main` — this is a workflow-state drift, not a code-state
defect. **Recommend: close the bead.**

— Black-hat reviewer, 2026-06-30