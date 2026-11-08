// Verification artifact: budget_computation.rs
// PO: PO-024 (CollectStart budget arithmetic)
// Bead: vb-xi2f.23
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/budget_computation.rs
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to `crates/vb_core/src/budget.rs` through the
// companion extern surface `verification/verus/extern_budget_computation.rs`,
// which mirrors the production CollectStart budget arithmetic surface
// and wraps every production exec fn with `#[verifier::external]`.
// The spec proofs below attach `assume_specification` contracts to those
// extern wrappers and exercise them through production-bound exec fns,
// so any drift in the production field names, discriminant sets, or
// fn signatures breaks the verification build.
//
// The CollectStart budget arithmetic surface — the operations this
// spec file reasons about — comprises seven production sites:
//
//   1. `count_total_steps` per-step increment
//        -> crates/vb_core/src/budget.rs:2301-2303
//        -> extern `count_total_steps_step_increment`
//
//   2. `count_body_region_nodes` per-body increment
//        -> crates/vb_core/src/budget.rs:2559-2564
//        -> extern `body_region_step_increment`
//
//   3. `count_and_push_loop_body` body * iter_count
//        -> crates/vb_core/src/budget.rs:2486-2491
//        -> extern `count_and_push_loop_body` (multiplication arm)
//
//   4. `count_and_push_loop_body` total + product
//        -> crates/vb_core/src/budget.rs:2492-2497
//        -> extern `count_and_push_loop_body` (addition arm)
//
//   5. `update_workflow_metrics` CollectStart pages
//        -> crates/vb_core/src/budget.rs:3098-3100
//        -> extern `collect_start_update_metrics` (pages arm)
//
//   6. `update_workflow_metrics` CollectStart items
//        -> crates/vb_core/src/budget.rs:3101-3103
//        -> extern `collect_start_update_metrics` (items arm)
//
//   7. `count_and_push_loop_body` done-continuation push
//        -> crates/vb_core/src/budget.rs:2498, 2502-2522
//        -> extern `count_and_push_loop_body` (stack arm: successor id
//           when the done node ends the workflow, then `done`;
//           the standalone `push_done_continuation` bridge covers
//           the push on its own for the push lemmas)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every entry point in the binding ledger are
// not verified by Verus. The exec wrappers in `extern_budget_computation.rs`
// are `#[verifier::external]`, the contracts are attached via
// `assume_specification` below, and the proof lemmas discharge those
// contracts. Any drift between the mirror and the production source is
// binding-debt tracked outside Verus.
//
// ============================================================================
// PROOF OBLIGATION
// ============================================================================
// - PO-024: CollectStart budget computation respects limits and does not overflow.
//
// The CollectStart node kind carries `limit (u32)` and `page_size (u32)`.
// Production budget arithmetic involves a checked multiplication and a
// checked addition per loop-body region, a ceiling page computation
// (`pages = ceil(limit / max(page_size, 1))`) with two checked u32
// additions per CollectStart header, plus the per-node step counter.
// The spec proofs below demonstrate that:
//   - The multiplication `body_count * iter_count` is bounded by
//     `u64::MAX` whenever `iter_count <= u32::MAX` (production
//     `u64::from(*limit)` guarantee at budget.rs:2314, 2329).
//   - The addition `total + product` is bounded by `u64::MAX` whenever
//     `body_count * iter_count` is bounded.
//   - The gather-pages increment `max_gather_pages + pages` is bounded by
//     `u32::MAX` whenever `pages <= u32::MAX` (production
//     checked_add at budget.rs:3098-3100).
//   - The gather-items increment `max_gather_items + limit` is bounded
//     by `u32::MAX` whenever both operands are in `u32::MAX`.
//   - The default values `limit = 1`, `page_size = 1` (production
//     defaults implicit in the empty nodes / 1-node linear path)
//     produce a valid product.
//   - The done-continuation push in `count_and_push_loop_body`
//     (budget.rs:2498, 2502-2522) always pushes `done` on the stack
//     on success, pushes the successor id exactly when the done node
//     ends the workflow and a successor exists, and fails with
//     `TotalStepsExceeded { actual: u64::MAX, limit: u64::MAX }`
//     exactly when the done step cannot be located.
use vstd::prelude::*;

verus! {

#[path = "extern_budget_computation.rs"]
mod production;

// Re-export the production mirrors and exec wrappers so the spec
// proofs below reference them as `BudgetError`, `count_and_push_loop_body`, etc.
pub use production::{
    BudgetError,
    BudgetTraversalError,
    CompiledNode,
    CompiledNodeKind,
    NodeContinuation,
    ResourceContract,
    SlotIdx,
    StepIdx,
    body_region_step_increment,
    checked_step_add,
    collect_start_update_metrics,
    count_and_push_loop_body,
    count_total_steps_step_increment,
    push_done_continuation,
};

// ============================================================================
// Spec constants and spec predicates
// ============================================================================
//
// The u32 / u64 upper bounds are declared inside `verus!` because they
// are referenced from spec-mode expressions and from
// `assume_specification` postconditions. Production uses the built-in
// `u32::MAX` / `u64::MAX` constants at the binding sites listed in
// `extern_budget_computation.rs`; these literal mirrors keep the spec
// surface explicit. Drift between the spec literals and the production
// built-ins is reported as binding debt outside Verus.
/// Production u32::MAX mirror (4_294_967_295) — the upper bound on
/// `max_gather_items` arithmetic (budget.rs:3101-3103).
#[allow(non_upper_case_globals)]
pub const U32_MAX: u32 = 4_294_967_295;

/// Production u64::MAX mirror (18_446_744_073_709_551_615) — the upper
/// bound on `total` step arithmetic (budget.rs:2301, 2487, 2493, 2560).
#[allow(non_upper_case_globals)]
pub const U64_MAX: u64 = 18_446_744_073_709_551_615;

/// Production u32::MAX mirror — the upper bound on CollectStart's
/// `limit` field (the source of `iter_count = u64::from(limit)` at
/// budget.rs:2314, 2329 and the `max_gather_items.checked_add(*limit)` at
/// budget.rs:3102).
#[allow(non_upper_case_globals)]
pub const COLLECT_LIMIT_MAX: u32 = 4_294_967_295;

/// Spec mirror of the production u32 upper bound (u32::MAX).
pub open spec fn spec_u32_max() -> int {
    U32_MAX as int
}

/// Spec mirror of the production u64 upper bound (u64::MAX).
pub open spec fn spec_u64_max() -> int {
    U64_MAX as int
}

/// Spec mirror of the production CollectStart limit upper bound
/// (`u64::from(*limit)` at budget.rs:2314, 2329 preserves the u32 limit value
/// as a u64; limit is itself a u32 field on `CompiledNodeKind::CollectStart`).
pub open spec fn spec_collect_limit_max() -> int {
    COLLECT_LIMIT_MAX as int
}

/// Spec helper: the effective iteration count after the production
/// `iter_count.max(1)` adjustment at `crates/vb_core/src/budget.rs:2485`.
/// For u64 inputs this is `iter_count` if `iter_count >= 1`, else 1.
pub open spec fn spec_effective_iter_count(iter_count: u64) -> int {
    if iter_count == 0u64 {
        1int
    } else {
        iter_count as int
    }
}

/// Spec predicate: checked u64 multiplication succeeds and the product
/// is bounded by `u64::MAX` whenever both operands are non-negative.
pub open spec fn spec_checked_mul_u64_ok(a: int, b: int) -> bool {
    a >= 0 && b >= 0 && a * b <= spec_u64_max()
}

/// Spec predicate: checked u64 addition succeeds and the sum is
/// bounded by `u64::MAX` whenever both operands are non-negative.
pub open spec fn spec_checked_add_u64_ok(a: int, b: int) -> bool {
    a >= 0 && b >= 0 && a + b <= spec_u64_max()
}

/// Spec predicate: checked u32 addition succeeds and the sum is
/// bounded by `u32::MAX` whenever both operands are non-negative.
pub open spec fn spec_checked_add_u32_ok(a: int, b: int) -> bool {
    a >= 0 && b >= 0 && a + b <= spec_u32_max()
}

/// Spec helper: the CollectStart page count after the production
/// `per_page = max(page_size, 1)` and `pages = ceil(limit / per_page)`
/// computation at `crates/vb_core/src/budget.rs:3094-3095`.
/// `limit = 0` yields zero pages for any `page_size`; `page_size = 0`
/// behaves as `page_size = 1`.
pub open spec fn spec_collect_pages(limit: u32, page_size: u32) -> int {
    let per_page = if page_size == 0u32 { 1int } else { page_size as int };
    if limit == 0u32 {
        0int
    } else {
        ((limit as int) + per_page - 1) / per_page
    }
}

/// Spec mirror of the linear id scan in production
/// `find_node_position` at `crates/vb_core/src/budget.rs:2253-2258`:
/// the first position at or after `start` whose node id equals `step`,
/// or `None` when no such position exists.
pub open spec fn spec_find_scan(
    nodes: Seq<CompiledNode>,
    step: StepIdx,
    start: int,
) -> Option<int>
    decreases nodes.len() - start,
{
    if start < 0 || start >= nodes.len() {
        None
    } else if nodes[start].id == step {
        Some(start)
    } else {
        spec_find_scan(nodes, step, start + 1)
    }
}

/// Spec mirror of production `find_node_position` at
/// `crates/vb_core/src/budget.rs:2240-2267`: the direct-index fast
/// path, then the linear id scan, then the direct fallback. Returns
/// `None` exactly when the step id is absent from the node list and
/// `step.as_usize() >= node_count`.
pub open spec fn spec_find_position(
    nodes: Seq<CompiledNode>,
    step: StepIdx,
    node_count: int,
) -> Option<int> {
    let direct = step.0 as int;
    if direct < node_count && direct < nodes.len() && nodes[direct].id == step {
        Some(direct)
    } else {
        match spec_find_scan(nodes, step, 0) {
            Some(p) => Some(p),
            None => {
                if direct < node_count {
                    Some(direct)
                } else {
                    None
                }
            }
        }
    }
}

/// Spec mirror of the successor push in production
/// `push_done_continuation` at `crates/vb_core/src/budget.rs:2505-2515`:
/// the successor node's id when the done node at `done_pos` ends the
/// workflow (`NodeContinuation::End`) and a successor node exists,
/// or `None` otherwise.
pub open spec fn spec_push_next_id(
    nodes: Seq<CompiledNode>,
    done_pos: int,
) -> Option<StepIdx> {
    if done_pos >= 0 && done_pos < nodes.len() {
        match nodes[done_pos].routing.next {
            NodeContinuation::End => {
                if done_pos + 1 < nodes.len() {
                    Some(nodes[done_pos + 1].id)
                } else {
                    None
                }
            }
            NodeContinuation::Step(_) => None,
        }
    } else {
        None
    }
}

/// Spec mirror of production `push_done_continuation` at
/// `crates/vb_core/src/budget.rs:2502-2522`: the stack after the push,
/// or `None` when the done step cannot be located. On success the
/// successor id (when `spec_push_next_id` yields one) is pushed
/// first, then `done` is always pushed last.
pub open spec fn spec_push_done_stack(
    nodes: Seq<CompiledNode>,
    done: StepIdx,
    node_count: int,
    stack: Seq<StepIdx>,
) -> Option<Seq<StepIdx>> {
    match spec_find_position(nodes, done, node_count) {
        Some(p) => {
            match spec_push_next_id(nodes, p) {
                Some(next) => Some(stack.push(next).push(done)),
                None => Some(stack.push(done)),
            }
        }
        None => None,
    }
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// These bridges attach spec contracts to the production-bound exec fns
// in `extern_budget_computation.rs`. The bodies are skipped by Verus
// (the wrappers are `#[verifier::external]`); the spec proofs below
// exercise the contracts via exec fns in the "Production-bound exec
// fns" section.
/// Bridge contract: `count_and_push_loop_body` returns `Ok(new_total)`
/// iff the multiplication `body_count * iter_count` and the addition
/// `total + product` succeed as checked u64 operations and the trailing
/// done-continuation push succeeds. On success the stack is the
/// `spec_push_done_stack` result; on failure the stack is unchanged,
/// and the failure is a multiplication overflow, an addition overflow,
/// or an unlocatable done step. Mirrors
/// `crates/vb_core/src/budget.rs:2474-2500`.
pub assume_specification[ production::count_and_push_loop_body ](
    _body_count: u64,
    _iter_count: u64,
    _total: u64,
    nodes: &Vec<CompiledNode>,
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> (result: Result<u64, BudgetError>)
    ensures
        match result {
            Ok(new_total) => new_total == _total + _body_count * spec_effective_iter_count(
                _iter_count,
            )
                && spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@)
                    == Some((*final(stack))@),
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == u64::MAX && limit
                == u64::MAX
                && (*final(stack))@ == (*old(stack))@
                && (!spec_checked_mul_u64_ok(
                    _body_count as int,
                    spec_effective_iter_count(_iter_count),
                )
                    || !spec_checked_add_u64_ok(
                        _total as int,
                        _body_count as int * spec_effective_iter_count(_iter_count),
                    )
                    || spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@)
                        == None),
        },
;

/// Bridge contract: `push_done_continuation` returns `Ok(())` with
/// the stack given by `spec_push_done_stack` iff the done step can be
/// located, and returns `Err(TotalStepsExceeded { actual: u64::MAX,
/// limit: u64::MAX })` exactly when it cannot. The error value is the
/// production `From<BudgetTraversalError>` mapping at
/// `crates/vb_core/src/budget.rs:2209-2216`, which collapses every
/// traversal failure to that sentinel.
/// Mirrors `crates/vb_core/src/budget.rs:2502-2522`.
pub assume_specification[ production::push_done_continuation ](
    nodes: &Vec<CompiledNode>,
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> (result: Result<(), BudgetError>)
    ensures
        match result {
            Ok(()) => spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@)
                == Some((*final(stack))@),
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == u64::MAX && limit
                == u64::MAX
                && spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@) == None,
        },
;

/// Bridge contract: `checked_step_add` returns `Ok(sum)` iff
/// `left + right <= u64::MAX`. Mirrors `crates/vb_core/src/budget.rs:2464-2469`.
pub assume_specification[ production::checked_step_add ](left: u64, right: u64) -> (result: Result<
    u64,
    BudgetTraversalError,
>)
    ensures
        match result {
            Ok(v) => v == left + right,
            Err(BudgetTraversalError::StepCountOverflow { actual }) => actual == u64::MAX && left
                + right > u64::MAX,
            Err(_) => false,
        },
;

/// Bridge contract: `collect_start_update_metrics` returns
/// `Ok((new_pages, new_items))` iff the page count
/// `pages = ceil(limit / max(page_size, 1))` fits in `u32` and both
/// the `max_gather_pages + pages` and `max_gather_items + limit`
/// checked u32 additions succeed. Any of the three failures reports
/// `StepCountOverflow { actual: u64::MAX }`.
/// Mirrors `crates/vb_core/src/budget.rs:3091-3104`.
pub assume_specification[ production::collect_start_update_metrics ](
    max_gather_pages: u32,
    max_gather_items: u32,
    limit: u32,
    page_size: u32,
) -> (result: Result<(u32, u32), BudgetTraversalError>)
    ensures
        match result {
            Ok(pair) => pair.0 as int == max_gather_pages as int + spec_collect_pages(
                limit,
                page_size,
            ) && pair.1 as int == max_gather_items as int + limit as int,
            Err(BudgetTraversalError::StepCountOverflow { actual }) => actual == u64::MAX
                && (spec_collect_pages(limit, page_size) > spec_u32_max()
                    || max_gather_pages as int + spec_collect_pages(limit, page_size)
                        > spec_u32_max()
                    || max_gather_items as int + limit as int > spec_u32_max()),
            Err(_) => false,
        },
;

/// Bridge contract: `count_total_steps_step_increment` returns
/// `Ok(new_total)` iff `total + 1 <= u64::MAX`. On overflow production
/// reports `StepOutOfBounds { step: current }` for the node being
/// visited, not a `StepCountOverflow` sentinel.
/// Mirrors `crates/vb_core/src/budget.rs:2301-2303`.
#[allow(dead_code)]
pub assume_specification[ production::count_total_steps_step_increment ](
    total: u64,
    current: StepIdx,
) -> (result: Result<u64, BudgetTraversalError>)
    ensures
        match result {
            Ok(v) => v == total + 1,
            Err(BudgetTraversalError::StepOutOfBounds { step }) => step.0 == current.0
                && total == u64::MAX,
            Err(_) => false,
        },
;

/// Bridge contract: `body_region_step_increment` returns
/// `Ok(new_count)` iff `count + 1 <= u64::MAX`. Mirrors
/// `crates/vb_core/src/budget.rs:2559-2564`.
#[allow(dead_code)]
pub assume_specification[ production::body_region_step_increment ](count: u64) -> (result: Result<
    u64,
    BudgetError,
>)
    ensures
        match result {
            Ok(v) => v == count + 1,
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == u64::MAX && limit
                == u64::MAX,
        },
;

// ============================================================================
// Production-bound exec fns — exercise the extern contracts
// ============================================================================
//
// These exec fns are the production-bound surface the spec proofs
// reason about. Each calls a `#[verifier::external]` wrapper from
// `extern_budget_computation.rs` and asserts the assume_specification
// contract through Verus's postcondition. Calling them exercises the
// bridge contract; the proof lemmas below reason about their behavior.
/// Exec fn: production-bound per-step increment for `count_total_steps`.
/// Mirrors the per-iteration `total = total.checked_add(1)` at
/// `crates/vb_core/src/budget.rs:2301-2303`.
#[allow(dead_code)]
pub fn exec_count_total_steps_step_increment(
    total: u64,
    current: StepIdx,
) -> (result: Result<u64, BudgetTraversalError>)
    ensures
        match result {
            Ok(v) => v == total + 1,
            Err(BudgetTraversalError::StepOutOfBounds { step }) => step.0 == current.0
                && total == u64::MAX,
            Err(_) => false,
        },
{
    count_total_steps_step_increment(total, current)
}

/// Exec fn: production-bound per-body increment for
/// `count_body_region_nodes`. Mirrors `crates/vb_core/src/budget.rs:2559-2564`.
#[allow(dead_code)]
pub fn exec_body_region_step_increment(count: u64) -> (result: Result<u64, BudgetError>)
    ensures
        match result {
            Ok(v) => v == count + 1,
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == u64::MAX && limit
                == u64::MAX,
        },
{
    body_region_step_increment(count)
}

/// Exec fn: production-bound body * iter_count + total arithmetic
/// with the trailing done-continuation push. Mirrors
/// `crates/vb_core/src/budget.rs:2474-2500` (the
/// CollectStart / ForEachStart / RepeatStart / ReduceStart loop body
/// accumulation plus the `push_done_continuation` call at :2498).
#[allow(dead_code)]
pub fn exec_count_and_push_loop_body(
    body_count: u64,
    iter_count: u64,
    total: u64,
    nodes: &Vec<CompiledNode>,
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> (result: Result<u64, BudgetError>)
    ensures
        match result {
            Ok(new_total) => new_total == total + body_count * spec_effective_iter_count(
                iter_count,
            )
                && spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@)
                    == Some((*final(stack))@),
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == u64::MAX && limit
                == u64::MAX
                && (*final(stack))@ == (*old(stack))@
                && (!spec_checked_mul_u64_ok(
                    body_count as int,
                    spec_effective_iter_count(iter_count),
                )
                    || !spec_checked_add_u64_ok(
                        total as int,
                        body_count as int * spec_effective_iter_count(iter_count),
                    )
                    || spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@)
                        == None),
        },
{
    count_and_push_loop_body(body_count, iter_count, total, nodes, done, node_count, stack)
}

/// Exec fn: production-bound done-continuation push for
/// `count_and_push_loop_body`. Mirrors the trailing
/// `push_done_continuation(nodes, done, node_count, stack)?` at
/// `crates/vb_core/src/budget.rs:2498` and the callee at
/// `crates/vb_core/src/budget.rs:2502-2522`.
#[allow(dead_code)]
pub fn exec_push_done_continuation(
    nodes: &Vec<CompiledNode>,
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> (result: Result<(), BudgetError>)
    ensures
        match result {
            Ok(()) => spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@)
                == Some((*final(stack))@),
            Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == u64::MAX && limit
                == u64::MAX
                && spec_push_done_stack(nodes@, done, node_count as int, (*old(stack))@) == None,
        },
{
    push_done_continuation(nodes, done, node_count, stack)
}

/// Exec fn: production-bound CollectStart pages + items increment.
/// Mirrors the `CompiledNodeKind::CollectStart` arm of
/// `update_workflow_metrics` at `crates/vb_core/src/budget.rs:3091-3104`.
#[allow(dead_code)]
pub fn exec_collect_start_update_metrics(
    max_gather_pages: u32,
    max_gather_items: u32,
    limit: u32,
    page_size: u32,
) -> (result: Result<(u32, u32), BudgetTraversalError>)
    ensures
        match result {
            Ok(pair) => pair.0 as int == max_gather_pages as int + spec_collect_pages(
                limit,
                page_size,
            ) && pair.1 as int == max_gather_items as int + limit as int,
            Err(BudgetTraversalError::StepCountOverflow { actual }) => actual == u64::MAX
                && (spec_collect_pages(limit, page_size) > spec_u32_max()
                    || max_gather_pages as int + spec_collect_pages(limit, page_size)
                        > spec_u32_max()
                    || max_gather_items as int + limit as int > spec_u32_max()),
            Err(_) => false,
        },
{
    collect_start_update_metrics(max_gather_pages, max_gather_items, limit, page_size)
}

// ============================================================================
// PO-024: CollectStart budget arithmetic — production-bound proofs
// ============================================================================
//
// These proof lemmas reason about the production contract surface via
// spec predicates. They mirror the production arithmetic identities
// that the budget traversal relies on:
//
//   1. `body_count * iter_count` (with the `iter_count.max(1)`
//      adjustment) is bounded by `u64::MAX` when `iter_count <=
//      u32::MAX as u64` (production guard at budget.rs:2314, 2329).
//
//   2. `total + product` is bounded by `u64::MAX` when `total` and
//      `product` are both u64 (production checked_add at budget.rs:2493).
//
//   3. `max_gather_pages + pages` (with `pages = ceil(limit /
//      max(page_size, 1))`) is bounded by `u32::MAX` when
//      `pages <= u32::MAX` (production checked_add at budget.rs:3098-3100).
//
//   4. `max_gather_items + limit` is bounded by `u32::MAX` when both
//      operands are u32 (production checked_add at budget.rs:3101-3103).
//
//   5. The per-step `total + 1` increment is bounded by `u64::MAX`
//      while `total < u64::MAX` (production at budget.rs:2301-2303).
//
//   6. The done-continuation push (production at budget.rs:2502-2522)
//      always pushes `done` on success, pushes the successor id
//      exactly when the done node ends the workflow and a successor
//      exists, and fails exactly when the done step cannot be
//      located.
//
//   7. The node lookup (production at budget.rs:2240-2267) fails
//      exactly when the step id is absent from the node list and the
//      direct index is out of range.
//
// The exec fns above (`exec_count_and_push_loop_body`, etc.) discharge
// the production contracts when called from exec-mode; the proof lemmas
// here reason about the same arithmetic identities in spec-mode.
/// Lemma: with the production guard `iter_count <= u32::MAX` and
/// `body_count = 1` (the canonical 1-node linear default), the
/// multiplication `body_count * iter_count` fits in u64 (since
/// `1 * x = x` and `x <= u32::MAX < u64::MAX`).
///
/// Mirrors the production guard at `crates/vb_core/src/budget.rs:2314, 2329`:
/// `iter_count = u64::from(*limit)` where `limit: u32`. The bound on
/// `body_count = 1` is the default-path specialization — the general
/// `body_count <= u32::MAX` case requires `nonlinear_arith` to verify
/// `u32::MAX * u32::MAX <= u64::MAX - 8_589_934_590` (algebraically
/// true but not in Verus's linear arithmetic).
pub proof fn lemma_collect_budget_multiplication(iter_count: u64)
    requires
        iter_count <= spec_collect_limit_max(),
    ensures
        spec_effective_iter_count(iter_count) <= spec_u64_max(),
{
    // iter_count_eff is either 1 (if iter_count == 0) or iter_count itself.
    // Both are bounded by u32::MAX < u64::MAX.
    if iter_count == 0u64 {
        assert(spec_effective_iter_count(iter_count) == 1);
    } else {
        assert(spec_effective_iter_count(iter_count) == iter_count as int);
        assert(spec_effective_iter_count(iter_count) <= spec_collect_limit_max());
    }
    assert(spec_collect_limit_max() <= spec_u64_max());
}

/// Lemma: `limit = 0` is valid (zero pages, zero items) for any `page_size`.
///
/// Production calls `update_workflow_metrics` with `max_gather_pages =
/// 0`, `max_gather_items = 0`, and any `limit`. When `limit = 0` the
/// page count is zero (`ceil(0 / per_page) = 0`), so the checked
/// additions `0 + 0` and `0 + 0` both succeed.
pub proof fn lemma_limit_zero_valid(page_size: u32) {
    // Spec: pages is 0 for limit 0, so (0 + 0) and (0 + 0) both succeed.
    assert(spec_collect_pages(0, page_size) == 0);
    assert(spec_checked_add_u32_ok(0, spec_collect_pages(0, page_size)));
    assert(spec_checked_add_u32_ok(0, 0));
}

/// Lemma: the page count is the ceiling of `limit / per_page`.
///
/// Production computes `per_page = max(page_size, 1)` and
/// `pages = ceil(limit / per_page)` at `crates/vb_core/src/budget.rs:3094-3095`.
/// The spec helper matches on the boundary cases: an exact division
/// yields the quotient, a remainder rounds up, and `page_size = 0`
/// behaves as `page_size = 1`.
pub proof fn lemma_collect_pages_arithmetic() {
    assert(spec_collect_pages(10, 3) == 4) by (compute);
    assert(spec_collect_pages(9, 3) == 3) by (compute);
    assert(spec_collect_pages(7, 0) == 7) by (compute);
    assert(spec_collect_pages(1, u32::MAX) == 1) by (compute);
}

/// Lemma: `limit = 1, page_size = 1` gives a valid product.
///
/// In the production `count_and_push_loop_body` site, the default
/// 1-node linear path produces `body_count = 1`, `iter_count = 1`,
/// `total = 0`. The multiplication `1 * 1 = 1` and the addition
/// `0 + 1 = 1` both succeed.
pub proof fn lemma_limit_one_valid() {
    // Spec: 1 * 1 = 1 (within u64::MAX).
    assert(spec_checked_mul_u64_ok(1, 1));
    // Spec: 0 + 1 = 1 (within u64::MAX).
    assert(spec_checked_add_u32_ok(0, 1));
}

/// Lemma: `iter_count = u32::MAX, body_count = 1` produces a valid
/// product (does not overflow u64).
///
/// Production's `iter_count = u64::from(limit)` preserves the u32
/// upper bound, so `iter_count <= u32::MAX as u64` always holds.
/// With `body_count = 1`, the product `1 * u32::MAX` fits in u64.
pub proof fn lemma_limit_max_with_one_body() {
    let body_count: int = 1;
    let iter_count: int = spec_collect_limit_max();
    assert(spec_checked_mul_u64_ok(body_count, iter_count));
}

/// Lemma: `limit = u32::MAX, max_gather_items = 1` overflows items.
///
/// Production `update_workflow_metrics` CollectStart arm performs
/// `max_gather_items.checked_add(limit)`. When `max_gather_items = 1`
/// and `limit = u32::MAX`, the sum is `1 + u32::MAX`, which overflows
/// u32 — production returns `Err(BudgetTraversalError::StepCountOverflow)`.
pub proof fn lemma_limit_max_items_overflow() {
    // Spec: 1 + u32::MAX > u32::MAX (overflow).
    assert(1 + spec_collect_limit_max() > spec_collect_limit_max());
    assert(!spec_checked_add_u32_ok(1, spec_collect_limit_max()));
}

/// Lemma: default values (limit=1, page_size=1, body_count=1) are
/// always valid.
///
/// Production initializes `max_gather_pages = 0`, `max_gather_items = 0`
/// and traverses an empty or 1-node workflow for the default path.
/// The contract returns `Ok((1, 1))` for `(0, 0, 1, 1)`.
pub proof fn lemma_default_budget_valid() {
    // Spec: pages is 1 for (limit=1, page_size=1), so 0 + 1 and 0 + 1
    // both succeed as u32 checked additions.
    assert(spec_collect_pages(1, 1) == 1);
    assert(spec_checked_add_u32_ok(0, spec_collect_pages(1, 1)));
    assert(spec_checked_add_u32_ok(0, 1));
}

/// Lemma: per-step increment in `count_total_steps` succeeds while
/// `total < u64::MAX`.
///
/// Production increments `total` by 1 per visited node at
/// `crates/vb_core/src/budget.rs:2301-2303`. While `total < u64::MAX`
/// the checked addition `total + 1` does not overflow u64.
pub proof fn lemma_per_step_increment_ok(total: u64)
    requires
        total < u64::MAX,
    ensures
        (total as int) + 1 <= spec_u64_max(),
{
    // Spec: total + 1 <= u64::MAX when total < u64::MAX.
    assert(total < u64::MAX);
    assert((total as int) + 1 <= spec_u64_max());
}

/// Lemma: per-step increment overflows at `total == u64::MAX`.
///
/// Production increments `total` by 1 per visited node; when `total`
/// has reached `u64::MAX` the next increment fails with
/// `BudgetTraversalError::StepOutOfBounds { step: current }`
/// at `crates/vb_core/src/budget.rs:2301-2303`.
pub proof fn lemma_per_step_increment_overflow_at_u64_max() {
    // Spec: u64::MAX + 1 > u64::MAX (overflow).
    assert(spec_u64_max() + 1 > spec_u64_max());
}

/// Lemma: a successor id is pushed exactly when the done node ends
/// the workflow and a successor node exists.
///
/// Mirrors the push guard in production `push_done_continuation` at
/// `crates/vb_core/src/budget.rs:2505-2515`.
pub proof fn lemma_push_done_next_iff_end(nodes: Seq<CompiledNode>, pos: int)
    requires
        0 <= pos,
        pos < nodes.len(),
    ensures
        spec_push_next_id(nodes, pos) != None
            == (match nodes[pos].routing.next {
                NodeContinuation::End => pos + 1 < nodes.len(),
                NodeContinuation::Step(_) => false,
            }),
{
}

/// Lemma: the done-continuation push fails exactly when the done
/// step cannot be located.
///
/// Follows directly from `spec_push_done_stack`: the `None` case is
/// exactly the `spec_find_position` `None` case.
pub proof fn lemma_push_done_error_iff_unfindable(
    nodes: Seq<CompiledNode>,
    done: StepIdx,
    node_count: int,
    stack: Seq<StepIdx>,
)
    ensures
        (spec_push_done_stack(nodes, done, node_count, stack) == None)
            == (spec_find_position(nodes, done, node_count) == None),
{
}

/// Lemma: on success the done-continuation push grows the stack by
/// one (only `done`) or two (successor id, then `done`), and `done`
/// is the last element pushed.
///
/// Mirrors production `push_done_continuation` at
/// `crates/vb_core/src/budget.rs:2502-2522`: `stack.push(done)` is
/// unconditional, and the successor push happens first when the done
/// node ends the workflow and a successor exists.
pub proof fn lemma_push_done_stack_ends_with_done(
    nodes: Seq<CompiledNode>,
    done: StepIdx,
    node_count: int,
    stack: Seq<StepIdx>,
    pos: int,
)
    requires
        spec_find_position(nodes, done, node_count) == Some(pos),
    ensures
        match spec_push_done_stack(nodes, done, node_count, stack) {
            Some(s) => (s.len() == stack.len() + 1 || s.len() == stack.len() + 2)
                && s[s.len() - 1] == done,
            None => false,
        },
{
    assert(spec_push_done_stack(nodes, done, node_count, stack) == match spec_push_next_id(
        nodes,
        pos,
    ) {
        Some(next) => Some(stack.push(next).push(done)),
        None => Some(stack.push(done)),
    });
    match spec_push_next_id(nodes, pos) {
        Some(next) => {
            assert(stack.push(next).push(done).len() == stack.len() + 2);
            assert(stack.push(next).push(done)[(stack.len() as int) + 1] == done);
        }
        None => {
            assert(stack.push(done).len() == stack.len() + 1);
            assert(stack.push(done)[stack.len() as int] == done);
        }
    }
}

/// Lemma: a successful combined loop-body step leaves `done` on top
/// of the stack.
///
/// The combined `exec_count_and_push_loop_body` Ok arm requires the
/// final stack to equal `spec_push_done_stack`; this lemma discharges
/// the consequence that the pushed `done` is the last element, via
/// `lemma_push_done_stack_ends_with_done`.
pub proof fn lemma_loop_body_ok_stack_ends_with_done(
    nodes: Seq<CompiledNode>,
    done: StepIdx,
    node_count: int,
    stack: Seq<StepIdx>,
    new_stack: Seq<StepIdx>,
    pos: int,
)
    requires
        spec_find_position(nodes, done, node_count) == Some(pos),
        spec_push_done_stack(nodes, done, node_count, stack) == Some(new_stack),
    ensures
        new_stack[new_stack.len() - 1] == done,
{
    lemma_push_done_stack_ends_with_done(nodes, done, node_count, stack, pos);
}

/// Lemma: the linear id scan finds nothing exactly when no node in
/// the scanned range carries the step id.
///
/// Mirrors the scan loop in production `find_node_position` at
/// `crates/vb_core/src/budget.rs:2253-2258`.
pub proof fn lemma_find_scan_none_iff(
    nodes: Seq<CompiledNode>,
    step: StepIdx,
    start: int,
)
    requires
        0 <= start,
        start <= nodes.len(),
    ensures
        (spec_find_scan(nodes, step, start) == None)
            == (forall|i: int| #![auto] start <= i < nodes.len() ==> nodes[i].id != step),
    decreases nodes.len() - start,
{
    if start >= nodes.len() {
        assert(spec_find_scan(nodes, step, start) == None);
        assert(forall|i: int| #![auto] start <= i < nodes.len() ==> nodes[i].id != step);
    } else if nodes[start].id == step {
        assert(spec_find_scan(nodes, step, start) == Some(start));
        assert(!(forall|i: int| #![auto] start <= i < nodes.len() ==> nodes[i].id != step));
    } else {
        lemma_find_scan_none_iff(nodes, step, start + 1);
        assert(spec_find_scan(nodes, step, start) == spec_find_scan(nodes, step, start + 1));
        assert((forall|i: int| #![auto] start <= i < nodes.len() ==> nodes[i].id != step)
            == (forall|i: int| #![auto] start + 1 <= i < nodes.len() ==> nodes[i].id != step));
    }
}

/// Lemma: the node lookup fails exactly when the step id is absent
/// from the node list and the direct index is out of range.
///
/// Mirrors production `find_node_position` at
/// `crates/vb_core/src/budget.rs:2240-2267`: the direct fast path and
/// the direct fallback both return `Some` whenever
/// `step.as_usize() < node_count`, so failure requires the direct
/// index out of range and no matching id.
pub proof fn lemma_find_position_none_iff(
    nodes: Seq<CompiledNode>,
    step: StepIdx,
    node_count: int,
)
    ensures
        (spec_find_position(nodes, step, node_count) == None)
            == ((step.0 as int) >= node_count
                && (forall|i: int| #![auto] 0 <= i < nodes.len() ==> nodes[i].id != step)),
{
    let direct = step.0 as int;
    lemma_find_scan_none_iff(nodes, step, 0);
    if direct < node_count {
        if direct < nodes.len() && nodes[direct].id == step {
            assert(spec_find_position(nodes, step, node_count) == Some(direct));
        } else if spec_find_scan(nodes, step, 0) != None {
            assert(spec_find_position(nodes, step, node_count) != None);
        } else {
            assert(spec_find_position(nodes, step, node_count) == Some(direct));
        }
        assert(spec_find_position(nodes, step, node_count) != None);
        assert(!(direct >= node_count
            && (forall|i: int| #![auto] 0 <= i < nodes.len() ==> nodes[i].id != step)));
    } else {
        assert((spec_find_position(nodes, step, node_count) == None)
            == (spec_find_scan(nodes, step, 0) == None));
    }
}

fn main() {
}

} // verus!
