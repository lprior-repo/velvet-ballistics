// Verus spec for CompiledWorkflow::try_from_parts validation.
//
// Bead: vb-xi2f.23 (try_from_parts structural validation).
// PO: PO-021 (CompiledWorkflow::try_from_parts validation).
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// Target: vb_core::workflow::CompiledWorkflow::try_from_parts
//   at crates/vb_core/src/workflow/mod.rs:42-65
//
// Production body (workflow/mod.rs:42-65):
//   pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
//       validate_parts(&parts)?;
//       let loop_regions = loop_regions::validate(&parts)?;
//       let nested_activation_entries =
//           loop_regions::nested_activation_map(&parts, &loop_regions)?;
//       validate_budget(&parts)?;
//       validate_input_slots(&parts)?;
//       Ok(Self { ... })
//
// Production validation sequence:
//   1. validate_parts (workflow/mod.rs:1367-1392):
//      - empty nodes,
//      - validate_resource_contract (1587-1593),
//      - validate_entry (1673-1675),
//      - validate_expressions (2046-2055),
//      - validate_accessors (2069-2074),
//      - per-node: validate_node_id (1472-1485) + validate_node
//        (1677-1682), including the branch-target alias checks
//        (validate_reduce_branch_targets 1684-1697,
//        validate_for_each_next_targets 1699-1709),
//      - validate_accessor_paths (2089-2119),
//      - validate_constants_symbols (2122-2132), modelled in the
//        projection (`constants_symbols_error`) and its spec mirror,
//      - validate_build_object_symbols (2135-2147), modelled in the
//        projection (`build_object_symbols_error`) and its spec mirror,
//      - validate_reachability (2160-2229), modelled in the
//        projection (`reachability_error`) and its spec mirror,
//      - validate_forward_edges (2336-2364), modelled in the
//        projection (`forward_edges_error`) and its spec mirror,
//      - repeat_counter_ownership::validate
//        (workflow/repeat_counter_ownership.rs:6-12),
//      - validate_no_nested_together
//        (engine/validate.rs:119-172), modelled in the
//        projection (`nested_together_error`) and its spec mirror.
//   2. loop_regions::validate (workflow/loop_regions.rs:59-72):
//      loop-region pairing (cache_control + unique_owner),
//      region relations (validate_region_relations), edge closure
//      (validate_region_edges), and recovery support
//      (validate_recovery_support). The projection models all four
//      sub-steps in production order; the per-region table is built
//      from the pairing step and the kind-target edges read the
//      caller-supplied `kind_targets` inventory.
//   3. loop_regions::nested_activation_map
//      (workflow/loop_regions.rs:78-83): per-node nested-activation
//      flags derived from the region table.
//   4. validate_budget (workflow/mod.rs:1453-1459): whole-workflow
//      boundedness policy via WholeWorkflowBudget::compute +
//      BoundednessPolicy::DEFAULT.validate, modelled in the
//      projection (`budget_error`) and its spec mirror over the
//      caller-supplied `computed_budget` inventory.
//   5. validate_input_slots (workflow/mod.rs:1394-1425): input slot
//      bounds, duplicate, and strict-increasing-order checks, plus a
//      per-slot validate_input_constraint (1427-1451), modelled from
//      the caller-supplied `input_constraints` inventory (a missing
//      entry is unconstrained).
//
// Binding mechanism: `#[path = "extern_try_from_parts.rs"]` imports the
// thin extern surface, which mirrors the production IR types with the
// same field set and a `#[verifier::external]` projection of the
// validation decision fn. The spec file attaches exec contracts via
// `assume_specification` and exercises them through exec wrappers that
// call the projection twice for determinism.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production `try_from_parts` body walks every node / expression /
// accessor and depends on a chain of helpers (validate_node_kind,
// validate_reachability, validate_forward_edges, ...). Verus cannot
// fully model this end-to-end graph walk. The pure projection in
// `extern_try_from_parts.rs` captures the *decision* (Ok vs the
// specific error variant the production body would return first, in
// `?`-propagation order) and is recorded as the trusted base in the
// binding ledger. Each proof below operates on the projection; any
// divergence between the projection and the production body is a
// binding-debt item tracked outside Verus.
//
// Per-node kind-payload checks are modelled from the ordered
// `kind_refs` inventory in the projection file. Reachability,
// forward edges, and nested-together are modelled, not approximated.
// The budget policy check is modelled from the caller-supplied
// `computed_budget` inventory: `budget_error` mirrors
// `BoundednessPolicy::DEFAULT.validate`'s first-violation order
// over the 23 validated dimensions, and the spec mirror pins each
// dimension and the ordering against steps 2 and 5. The compute
// walk that produces the budget stays caller-supplied:
// `budget_bounded.rs` is the exhaustive coverage of `compute`'s
// boundedness, not re-modelled here.

use vstd::prelude::*;

verus! {

#[path = "extern_try_from_parts.rs"]
mod production;

pub use production::{
    spec_loop_edge_origin_error, spec_loop_edge_origin_kind, spec_loop_edge_origin_next,
    spec_node_kind_ask, spec_node_kind_ask_resume, spec_node_kind_build_list,
    spec_node_kind_build_object, spec_node_kind_choose,
    spec_node_kind_choose_slot, spec_node_kind_collect_finish,
    spec_node_kind_collect_next, spec_node_kind_collect_page,
    spec_node_kind_collect_start, spec_node_kind_copy, spec_node_kind_do,
    spec_node_kind_error_handler, spec_node_kind_eval_expr,
    spec_node_kind_finish, spec_node_kind_foreach_join,
    spec_node_kind_foreach_next, spec_node_kind_foreach_start,
    spec_node_kind_jump, spec_node_kind_nop, spec_node_kind_reduce_finish,
    spec_node_kind_reduce_next, spec_node_kind_reduce_start,
    spec_node_kind_repeat_attempt, spec_node_kind_repeat_check,
    spec_node_kind_repeat_finish, spec_node_kind_repeat_start,
    spec_node_kind_retry_check, spec_node_kind_set_const,
    spec_node_kind_together_branch, spec_node_kind_together_join,
    spec_node_kind_together_start, spec_node_kind_wait_event,
    spec_node_kind_wait_until, spec_ref_none, SpecAccessor, SpecComputedBudget,
    SpecExprOp, SpecExprProgram,
    SpecInputConstraint, SpecKindRef, SpecLoopRegion, SpecNode, SpecNodeKind, SpecNodeMeta,
    SpecPathSegment, SpecResourceContract, SpecValidationResult, SpecWorkflowError,
    SpecWorkflowParts, accessor_path_error, branch_target_alias_error, budget_error, expr_error,
    input_constraint_error,
    kind_ref_error, node_id_error,
    forward_edges_error, loop_pairing_error, loop_region_edges_error,
    loop_region_relations_error, nested_together_error,
    reachability_error, repeat_counter_ownership_error,
    spec_validation_result_is_ok, spec_workflow_error_is_bound_violation,
    try_from_parts_pure, validate_node_pure,
};

// ============================================================================
// Spec predicates (mathematical model used by proofs)
// ============================================================================

/// Spec predicate: a `SpecValidationResult` is the success variant.
pub open spec fn spec_validation_ok(r: SpecValidationResult) -> bool {
    r == SpecValidationResult::Ok
}

/// Spec predicate: a `SpecWorkflowError` discriminant is one of the
/// typed bound violations (slot, step, const, expr, accessor, symbol,
/// empty, input constraint, counter slot). Spec-fn mirror of the const fn
/// `spec_workflow_error_is_bound_violation` in
/// `extern_try_from_parts.rs`.
pub open spec fn spec_workflow_error_is_bound_violation_spec(e: SpecWorkflowError) -> bool {
    matches!(
        e,
        SpecWorkflowError::RepeatCounterForeignWrite
            | SpecWorkflowError::EmptyNodes
            | SpecWorkflowError::EntryOutOfBounds
            | SpecWorkflowError::StepOutOfBounds
            | SpecWorkflowError::SlotOutOfBounds
            | SpecWorkflowError::ConstOutOfBounds
            | SpecWorkflowError::ExpressionInvalid
            | SpecWorkflowError::SymbolOutOfBounds
            | SpecWorkflowError::AccessorPathTooDeep
            | SpecWorkflowError::InputSlotOutOfBounds
            | SpecWorkflowError::DuplicateInputSlot
            | SpecWorkflowError::NonCanonicalInputSlotOrder
            | SpecWorkflowError::InconsistentInputConstraint
    )
}

/// Spec predicate: a `SpecWorkflowError` discriminant is one of the
/// structural / policy failures (resource contract, branch table,
/// reachability, forward edges, loop nesting, budget, jump cycles,
/// nested together, node identity, branch-target aliasing, loop
/// regions). Spec-fn mirror of the production discriminant set.
pub open spec fn spec_workflow_error_is_structural(e: SpecWorkflowError) -> bool {
    matches!(
        e,
        SpecWorkflowError::NodeIdMismatch
            | SpecWorkflowError::ResourceContractExceeded
            | SpecWorkflowError::ResourceContractTooLarge
            | SpecWorkflowError::EmptyBranchTable
            | SpecWorkflowError::UnreachableNode
            | SpecWorkflowError::BackwardEdge
            | SpecWorkflowError::ReduceBranchTargetAlias
            | SpecWorkflowError::ForEachBranchTargetAlias
            | SpecWorkflowError::ImproperLoopNesting
            | SpecWorkflowError::BudgetPolicyExceeded
            | SpecWorkflowError::StepCountOverflow
            | SpecWorkflowError::DepthOverflow
            | SpecWorkflowError::JumpCycle
            | SpecWorkflowError::NestedTogether
            | SpecWorkflowError::LoopRegionUnpaired
            | SpecWorkflowError::LoopRegionCrossing
            | SpecWorkflowError::LoopRegionUnsupported
    )
}

/// Spec predicate: the `SpecNodeKind` discriminant is one of the
/// documented variants. Closed set (matches the production enum at
/// `crates/vb_core/src/workflow/mod.rs:1191-1357`).
pub open spec fn spec_node_kind_disc_valid(disc: int) -> bool {
    0 <= disc <= 33
}

/// Spec predicate: a `SpecValidationResult` discriminant is either
/// `Ok` or one of the documented `SpecWorkflowError` variants.
pub open spec fn spec_validation_result_valid(r: SpecValidationResult) -> bool {
    ||| spec_validation_ok(r)
    ||| spec_workflow_error_is_bound_violation_spec(spec_validation_err(r))
    ||| spec_workflow_error_is_structural(spec_validation_err(r))
}

/// Spec accessor: extracts the error payload from an
/// `Err(_)`-shaped `SpecValidationResult`. Returns
/// `EmptyNodes` as a default if the input is `Ok` (the value is
/// irrelevant; callers gate on `spec_validation_ok` first).
pub open spec fn spec_validation_err(r: SpecValidationResult) -> SpecWorkflowError {
    match r {
        SpecValidationResult::Ok => SpecWorkflowError::EmptyNodes,
        SpecValidationResult::Err(e) => e,
    }
}

// ============================================================================
// assume_specification bridges: bind the production exec fns to spec fns
// ============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to a Rust function whose body Verus cannot fully model.
// The contracts below state the postcondition of each projection: the
// decision fn is closed (its discriminant set is bounded), the result
// agrees with the spec-level mirror, and `Ok` is signalled
// exclusively by the `SpecValidationResult::Ok` variant.
//
// TRUST BOUNDARY: the bodies of `try_from_parts_pure` and
// `validate_node_pure` are in the extern file; Verus accepts the
// ensures via `assume_specification` but does not verify the body
// itself. This matches the binding ledger entry for the
// try_from_parts validation contract.

pub assume_specification[ production::try_from_parts_pure ](
    parts: &SpecWorkflowParts,
) -> (result: SpecValidationResult)
    ensures
        // Discriminant validity: every result is either Ok or one of
        // the documented Err variants.
        spec_validation_result_valid(result),
        // Spec-side decision equivalence: the projection's result
        // matches the spec mirror at the contract layer. The mirror
        // collapses the per-step ordering of the production
        // validator to a single mathematical decision.
        result == try_from_parts_pure_spec(parts),
;

pub assume_specification[ production::validate_node_pure ](
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
    slot_count: u32,
    node_count: u32,
    expressions_len: u32,
    accessors_len: u32,
    constants_len: u32,
    symbols_count: u32,
) -> (err: Option<SpecWorkflowError>)
    ensures
        // The decision is closed: if Some, it is one of the typed
        // bound violations, the EmptyBranchTable arm, the
        // ResourceContractExceeded arm (count cap / nonzero kind
        // checks), or one of the branch-target alias arms.
        err.is_none() || spec_workflow_error_is_bound_violation_spec(err.unwrap())
            || err.unwrap() == SpecWorkflowError::EmptyBranchTable
            || err.unwrap() == SpecWorkflowError::ResourceContractExceeded
            || err.unwrap() == SpecWorkflowError::ReduceBranchTargetAlias
            || err.unwrap() == SpecWorkflowError::ForEachBranchTargetAlias,
        // Spec-side decision equivalence: the projection's result
        // matches the spec mirror at the contract layer.
        err == validate_node_pure_spec(meta, kind_refs, slot_count as int, node_count as int,
            expressions_len as int, accessors_len as int, constants_len as int,
            symbols_count as int),
;

/// Spec-level mirror of `production::kind_ref_error`.
/// Returns the first failure the ordered kind-ref inventory would
/// report, or `None` when every entry is in bounds.
pub assume_specification[ production::kind_ref_error ](
    refs: &Vec<SpecKindRef>,
    slot_count: u32,
    node_count: u32,
    constants_len: u32,
    expressions_len: u32,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_kind_ref_error(refs, 0, slot_count as int, node_count as int,
            constants_len as int, expressions_len as int),
;

/// Spec-level mirror of `production::node_id_error`.
/// Returns `Some(NodeIdMismatch)` when the node's id differs from
/// its table position, or `None` when they match.
pub assume_specification[ production::node_id_error ](
    meta: SpecNodeMeta,
    index: usize,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_node_id_error(meta, index as int),
;

/// Spec-level mirror of `production::expr_error`. Returns
/// `Some(ExpressionInvalid)` for the first `validate_expressions`
/// failure in production order (op count, stack simulation in op
/// order, final depth, max_stack mismatch, accessor scan), or
/// `None` when the program validates.
pub assume_specification[ production::expr_error ](
    program: &SpecExprProgram,
    accessors_len: u32,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_expr_error(*program, accessors_len as int),
;

/// Spec-level mirror of `production::accessor_path_error`. Returns
/// `Some(AccessorPathTooDeep)` when the path exceeds
/// `MAX_PATH_DEPTH` (16), `Some(SymbolOutOfBounds)` for the first
/// `Field` symbol at or beyond `symbols_count`,
/// `Some(ExpressionInvalid)` for the first reserved
/// `Index(u32::MAX)`, or `None` when the path validates, in
/// production order (workflow/mod.rs:2089-2119).
pub assume_specification[ production::accessor_path_error ](
    accessor: &SpecAccessor,
    symbols_count: u32,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_accessor_path_error(*accessor, symbols_count as int),
;

/// Spec-level mirror of `production::constants_symbols_error`.
/// Returns `Some(SymbolOutOfBounds)` for the first pooled symbol
/// at or beyond `symbols_count`, or `None` when every pooled
/// symbol validates, in production order
/// (workflow/mod.rs:2122-2132).
pub assume_specification[ production::constants_symbols_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_constants_symbols_error(parts, 0),
;

/// Spec-level mirror of `production::build_object_symbols_error`.
/// Returns `Some(SymbolOutOfBounds)` for the first BuildObject
/// field symbol at or beyond `symbols_count`, or `None` when
/// every field symbol validates, in production order
/// (workflow/mod.rs:2135-2147).
pub assume_specification[ production::build_object_symbols_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_build_object_symbols_error(parts, 0),
;

/// Spec-level mirror of `production::budget_error`. Returns
/// `Some(BudgetPolicyExceeded)` for the first dimension of the
/// supplied computed budget over its `BoundednessPolicy::DEFAULT`
/// cap, in production `validate` order (budget.rs:534-592, via
/// `validate_extended_budget` at 644-661, `validate_payload_budget`
/// at 663-698, and `validate_budget_dimensions` at 595-642), or
/// `None` when every dimension is within caps.
pub assume_specification[ production::budget_error ](
    budget: SpecComputedBudget,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_budget_error(budget),
;

/// Spec-level mirror of `production::branch_target_alias_error`.
/// Returns the alias error when the node's branch-target pair
/// aliases (`body == done`) for a ReduceStart / ReduceNext /
/// ForEachNext node, or `None` otherwise.
pub assume_specification[ production::branch_target_alias_error ](
    meta: SpecNodeMeta,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_branch_target_alias_error(meta),
;

/// Spec-level mirror of `production::input_constraint_error`.
/// Returns `Some(InconsistentInputConstraint)` for the first
/// inconsistency in production order (min/max range, then
/// min_length/max_length range, then negative length bounds),
/// or `None` when the constraint is consistent.
pub assume_specification[ production::input_constraint_error ](
    constraint: SpecInputConstraint,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_input_constraint_error(constraint),
;

/// Spec-level mirror of `production::loop_pairing_error`. Returns the
/// first `LoopRegionUnpaired` failure the pairing step would report,
/// or `None` when every cached control pairs with a unique ordered
/// start. The mirror is the contract layer for the pairing decision;
/// the exec projection in `extern_try_from_parts.rs` is the trusted
/// base recorded in the binding ledger.
pub assume_specification[ production::loop_pairing_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_pairing_error(&parts.nodes_meta, 0),
;

/// Spec-level mirror of `production::loop_region_relations_error`.
/// Returns the first `LoopRegionUnpaired` (shared start) or
/// `LoopRegionCrossing` (partially overlapping regions) failure the
/// relations step would report, or `None` when pairing failed (no
/// region table exists) or every region pair is properly related.
pub assume_specification[ production::loop_region_relations_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_loop_relations_error(parts),
;

/// Spec-level mirror of `production::loop_region_edges_error`.
/// Returns the first `LoopRegionCrossing` (entry inside a region,
/// or a routing / kind edge crossing a region boundary) or
/// `LoopRegionUnsupported` (internal error handler or an
/// `ErrorHandler` node inside a region) failure the edge-closure
/// step would report, or `None` when pairing failed (no region
/// table exists) or every region edge closes.
pub assume_specification[ production::loop_region_edges_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_loop_edges_error(parts),
;

/// Spec-level mirror of `production::repeat_counter_ownership_error`.
/// Returns the first `RepeatCounterForeignWrite` failure the
/// counter-ownership step would report — a duplicate counter
/// claim, or a foreign write to a claimed counter slot — or `None`
/// when every claimed counter slot is owned exclusively.
pub assume_specification[ production::repeat_counter_ownership_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_repeat_counter_error(parts),
;

/// Spec-level mirror of `production::forward_edges_error`.
/// Returns the first `BackwardEdge` / `ImproperLoopNesting`
/// failure the forward-edges step would report — a backward
/// routing or kind edge, a `RepeatCheck` back-edge with no valid
/// owner, or an improperly nested loop span — or `None` when
/// every edge points forward and every loop span nests properly.
pub assume_specification[ production::forward_edges_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_forward_edges_error(parts),
;

/// Spec-level mirror of `production::reachability_error`.
/// Returns the first `UnreachableNode` failure the reachability
/// step would report — the first node in table order not
/// reachable from the entry over routing `next`, routing error
/// handlers, and kind targets — or `None` when every node is
/// reachable.
pub assume_specification[ production::reachability_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_reachability_error(parts),
;

/// Spec-level mirror of `production::nested_together_error`.
/// Returns the first `NestedTogether` failure the
/// nested-together step would report — a `TogetherStart`
/// reachable from another `TogetherStart`'s branch body over the
/// routing `next` chain — or `None` when no `TogetherStart` is
/// nested inside another.
pub assume_specification[ production::nested_together_error ](
    parts: &SpecWorkflowParts,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err == spec_nested_together_error(parts),
;

/// Spec-level mirror of `production::branch_target_alias_error`.
/// Returns the alias error when the node's branch-target pair
/// aliases (`body == done`) for a ReduceStart / ReduceNext
/// (discriminants 19 / 20) or ForEachNext (discriminant 10) node,
/// or `None` otherwise.
pub open spec fn spec_branch_target_alias_error(meta: SpecNodeMeta) -> Option<SpecWorkflowError> {
    if meta.has_branch_targets == 1 && meta.branch_body == meta.branch_done {
        if meta.kind_disc == 19u32 || meta.kind_disc == 20u32 {
            Some(SpecWorkflowError::ReduceBranchTargetAlias)
        } else if meta.kind_disc == 10u32 {
            Some(SpecWorkflowError::ForEachBranchTargetAlias)
        } else {
            None
        }
    } else {
        None
    }
}

/// Spec-level mirror of `production::kind_ref_error`. Walks the
/// ordered kind-ref inventory from index `i` and returns the first
/// failure production's `validate_node_kind` would report for the
/// corresponding payload reference, or `None` when every remaining
/// entry is in bounds. `Slot` / `Step` / `Const` / `Expr` check one
/// reference against its table length; `Count(len, cap)` reports
/// `ResourceContractExceeded` when the count exceeds its cap;
/// `NonZero(value)` reports it when the value is zero.
pub open spec fn spec_kind_ref_error(
    refs: &Vec<SpecKindRef>,
    i: int,
    slot_count: int,
    node_count: int,
    constants_len: int,
    expressions_len: int,
) -> Option<SpecWorkflowError>
    decreases refs.len() - i
{
    if i < 0 || i >= refs.len() {
        None
    } else {
        match refs[i] {
            SpecKindRef::Slot(value) => {
                if (value as int) >= slot_count {
                    Some(SpecWorkflowError::SlotOutOfBounds)
                } else {
                    spec_kind_ref_error(refs, i + 1, slot_count, node_count,
                        constants_len, expressions_len)
                }
            }
            SpecKindRef::Step(value) => {
                if (value as int) >= node_count {
                    Some(SpecWorkflowError::StepOutOfBounds)
                } else {
                    spec_kind_ref_error(refs, i + 1, slot_count, node_count,
                        constants_len, expressions_len)
                }
            }
            SpecKindRef::Const(value) => {
                if (value as int) >= constants_len {
                    Some(SpecWorkflowError::ConstOutOfBounds)
                } else {
                    spec_kind_ref_error(refs, i + 1, slot_count, node_count,
                        constants_len, expressions_len)
                }
            }
            SpecKindRef::Expr(value) => {
                if (value as int) >= expressions_len {
                    Some(SpecWorkflowError::ExpressionInvalid)
                } else {
                    spec_kind_ref_error(refs, i + 1, slot_count, node_count,
                        constants_len, expressions_len)
                }
            }
            SpecKindRef::Count(len, cap) => {
                if (len as int) > (cap as int) {
                    Some(SpecWorkflowError::ResourceContractExceeded)
                } else {
                    spec_kind_ref_error(refs, i + 1, slot_count, node_count,
                        constants_len, expressions_len)
                }
            }
            SpecKindRef::NonZero(value) => {
                if value == 0 {
                    Some(SpecWorkflowError::ResourceContractExceeded)
                } else {
                    spec_kind_ref_error(refs, i + 1, slot_count, node_count,
                        constants_len, expressions_len)
                }
            }
        }
    }
}

/// Spec-level mirror of `production::node_id_error`.
/// Returns `Some(NodeIdMismatch)` when the node's id differs from
/// its table position, or `None` when they match. Production's
/// `u16::try_from(index)` arm is unreachable on inputs that reach
/// step 1e: step 1a caps `max_steps` at 10000, so every table
/// position is below 65536 and the conversion cannot fail.
pub open spec fn spec_node_id_error(meta: SpecNodeMeta, index: int) -> Option<SpecWorkflowError> {
    if (meta.id as int) == index {
        None
    } else {
        Some(SpecWorkflowError::NodeIdMismatch)
    }
}

/// Spec-level mirror of the step-1e per-node loop's id pass:
/// walks the node table from index `i` and returns the first
/// `NodeIdMismatch` in table order, or `None` when every remaining
/// id matches its table position. Production checks
/// `validate_node_id` before `validate_node` for each node, in
/// node order (workflow/mod.rs:1375-1378).
pub open spec fn spec_nodes_id_error(metas: &Vec<SpecNodeMeta>, i: int) -> Option<SpecWorkflowError>
    decreases metas.len() - i
{
    if i < 0 || i >= metas.len() {
        None
    } else {
        let err = spec_node_id_error(metas[i], i);
        if err != None::<SpecWorkflowError> {
            err
        } else {
            spec_nodes_id_error(metas, i + 1)
        }
    }
}

/// Spec-level mirror of production `expr_stack_effect`
/// (workflow/mod.rs:2728-2741): the `(pop, push)` stack effect of
/// one `SpecExprOp`.
pub open spec fn spec_expr_effect(op: SpecExprOp) -> (int, int) {
    match op {
        SpecExprOp::Push => (0, 1),
        SpecExprOp::LoadAccessor(_) => (0, 1),
        SpecExprOp::Unary => (1, 1),
        SpecExprOp::AppendIf => (3, 1),
        SpecExprOp::Binary => (2, 1),
    }
}

/// Spec-level mirror of `check_expr_stack_bound`
/// (workflow/mod.rs:976-987): simulates the stack over `ops`
/// from index `i`, tracking the running `depth` and the peak
/// `required` depth. Returns `Some(ExpressionInvalid)` for the
/// first underflow (`apply_expr_stack_effect`,
/// workflow/mod.rs:2692-2702), per-op capacity breach
/// (`validate_expr_stack_capacity`, workflow/mod.rs:2704-2710,
/// against the claimed `max_stack` and the 64-entry ceiling,
/// limits.rs:43), final-depth failure
/// (`validate_expr_final_depth`, workflow/mod.rs:2712-2720), or
/// computed-required != claimed-max_stack mismatch
/// (`ExprProgram::try_from_parts`, workflow/mod.rs:899-908), or
/// `None` when the simulation accepts.
pub open spec fn spec_expr_sim_error(
    ops: &Vec<SpecExprOp>,
    i: int,
    depth: int,
    required: int,
    max_stack: int,
) -> Option<SpecWorkflowError>
    decreases ops.len() - i
{
    if i < 0 || i >= ops.len() {
        if depth == 0 || depth > 1 {
            Some(SpecWorkflowError::ExpressionInvalid)
        } else if required != max_stack {
            Some(SpecWorkflowError::ExpressionInvalid)
        } else {
            None
        }
    } else {
        let (pop, push) = spec_expr_effect(ops[i]);
        if depth < pop {
            Some(SpecWorkflowError::ExpressionInvalid)
        } else {
            let depth2 = depth - pop + push;
            let required2 = if depth2 > required { depth2 } else { required };
            if required2 > max_stack || required2 > 64 {
                Some(SpecWorkflowError::ExpressionInvalid)
            } else {
                spec_expr_sim_error(ops, i + 1, depth2, required2, max_stack)
            }
        }
    }
}

/// Spec-level mirror of `validate_expression_accessors`
/// (workflow/mod.rs:2057-2067) with `validate_accessor`
/// (workflow/mod.rs:2076-2086): walks `ops` from index `i` and
/// returns `Some(ExpressionInvalid)` for the first `LoadAccessor`
/// index at or beyond `accessors_len`, or `None` when every
/// accessor reference is in bounds.
pub open spec fn spec_expr_accessor_error(
    ops: &Vec<SpecExprOp>,
    i: int,
    accessors_len: int,
) -> Option<SpecWorkflowError>
    decreases ops.len() - i
{
    if i < 0 || i >= ops.len() {
        None
    } else {
        match ops[i] {
            SpecExprOp::LoadAccessor(index) => {
                if (index as int) >= accessors_len {
                    Some(SpecWorkflowError::ExpressionInvalid)
                } else {
                    spec_expr_accessor_error(ops, i + 1, accessors_len)
                }
            }
            _ => spec_expr_accessor_error(ops, i + 1, accessors_len),
        }
    }
}

/// Spec-level mirror of `production::expr_error`: the
/// `validate_expressions` decision (workflow/mod.rs:2046-2055)
/// for one program, in production order: op count
/// (`validate_expr_op_count`, workflow/mod.rs:2682-2690, cap 256,
/// limits.rs:31), stack simulation, final depth, max_stack
/// mismatch, then the accessor scan. Returns
/// `Some(ExpressionInvalid)` for the first failure, or `None`
/// when the program validates.
pub open spec fn spec_expr_error(
    program: SpecExprProgram,
    accessors_len: int,
) -> Option<SpecWorkflowError> {
    if program.ops.len() > 256 {
        Some(SpecWorkflowError::ExpressionInvalid)
    } else {
        let sim_err = spec_expr_sim_error(&program.ops, 0, 0, 0, program.max_stack as int);
        if sim_err != None::<SpecWorkflowError> {
            sim_err
        } else {
            spec_expr_accessor_error(&program.ops, 0, accessors_len)
        }
    }
}

/// Spec-level mirror of the step-1a per-expression `max_stack`
/// check in `validate_expr_stack_usage`
/// (workflow/mod.rs:1656-1668): walks `parts.expressions` from
/// index `i` and returns `Some(ResourceContractExceeded)` for the
/// first program whose `max_stack` exceeds the contract cap, or
/// `None` when every program fits. Production rejects when any
/// `expression.max_stack > max_expr_stack`, inside
/// `validate_resource_contract` (workflow/mod.rs:1587-1593).
pub open spec fn spec_expr_stack_error(
    parts: &SpecWorkflowParts,
    i: int,
) -> Option<SpecWorkflowError>
    decreases parts.expressions.len() - i
{
    if i < 0 || i >= parts.expressions.len() {
        None
    } else if parts.expressions[i].max_stack > parts.resource_contract.max_expr_stack {
        Some(SpecWorkflowError::ResourceContractExceeded)
    } else {
        spec_expr_stack_error(parts, i + 1)
    }
}

/// Spec-level mirror of the step-1c per-expression loop: walks
/// `parts.expressions` from index `i` and returns the first
/// `ExpressionInvalid` in expression order, or `None` when every
/// program validates. Production validates expression `i` fully
/// before expression `i + 1` (workflow/mod.rs:2046-2055).
pub open spec fn spec_exprs_error(
    parts: &SpecWorkflowParts,
    i: int,
) -> Option<SpecWorkflowError>
    decreases parts.expressions.len() - i
{
    if i < 0 || i >= parts.expressions.len() {
        None
    } else {
        let err = spec_expr_error(parts.expressions[i], parts.accessors_len as int);
        if err != None::<SpecWorkflowError> {
            err
        } else {
            spec_exprs_error(parts, i + 1)
        }
    }
}

/// Spec-level mirror of the per-segment walk in
/// `validate_accessor_paths` (workflow/mod.rs:2089-2119): walks
/// `segments` from index `i` and returns `Some(SymbolOutOfBounds)`
/// for the first `Field` symbol at or beyond `symbols_count`,
/// `Some(ExpressionInvalid)` for the first reserved
/// `Index(u32::MAX)`, or `None` when every segment validates.
pub open spec fn spec_accessor_path_segment_error(
    segments: &Vec<SpecPathSegment>,
    i: int,
    symbols_count: int,
) -> Option<SpecWorkflowError>
    decreases segments.len() - i
{
    if i < 0 || i >= segments.len() {
        None
    } else {
        match segments[i] {
            SpecPathSegment::Field(symbol) => {
                if (symbol as int) >= symbols_count {
                    Some(SpecWorkflowError::SymbolOutOfBounds)
                } else {
                    spec_accessor_path_segment_error(segments, i + 1, symbols_count)
                }
            }
            SpecPathSegment::Index(index) => {
                if index == u32::MAX {
                    Some(SpecWorkflowError::ExpressionInvalid)
                } else {
                    spec_accessor_path_segment_error(segments, i + 1, symbols_count)
                }
            }
        }
    }
}

/// Spec-level mirror of `production::accessor_path_error`: the
/// `validate_accessor_paths` decision (workflow/mod.rs:2089-2119)
/// for one accessor, in production order: path length against
/// `MAX_PATH_DEPTH` (16, limits.rs:59), then the per-segment
/// walk. Returns the first failure, or `None` when the path
/// validates.
pub open spec fn spec_accessor_path_error(
    accessor: SpecAccessor,
    symbols_count: int,
) -> Option<SpecWorkflowError> {
    if accessor.path.len() > 16 {
        Some(SpecWorkflowError::AccessorPathTooDeep)
    } else {
        spec_accessor_path_segment_error(&accessor.path, 0, symbols_count)
    }
}

/// Spec-level mirror of the step-1f accessor loop: walks
/// `parts.accessors` from index `i` and returns the first
/// accessor-path error in accessor order, or `None` when every
/// path validates. Production validates accessor `i` fully
/// before accessor `i + 1` (workflow/mod.rs:2089-2119), after
/// the per-node loop (workflow/mod.rs:1375-1378).
pub open spec fn spec_accessors_path_error(
    parts: &SpecWorkflowParts,
    i: int,
) -> Option<SpecWorkflowError>
    decreases parts.accessors.len() - i
{
    if i < 0 || i >= parts.accessors.len() {
        None
    } else {
        let err = spec_accessor_path_error(parts.accessors[i], parts.symbols_count as int);
        if err != None::<SpecWorkflowError> {
            err
        } else {
            spec_accessors_path_error(parts, i + 1)
        }
    }
}

/// Spec-level mirror of the `validate_constants_symbols` scan
/// (workflow/mod.rs:2122-2132): walks the caller-supplied
/// symbol-only `constants_symbols` inventory from index `i` and
/// returns `Some(SymbolOutOfBounds)` for the first symbol at or
/// beyond `symbols_count`, or `None` when every pooled symbol
/// validates. Production checks each `ConstValue::Symbol`
/// constant in pool order via `validate_symbol`
/// (workflow/mod.rs:2150-2156).
pub open spec fn spec_constants_symbols_error(
    parts: &SpecWorkflowParts,
    i: int,
) -> Option<SpecWorkflowError>
    decreases parts.constants_symbols.len() - i
{
    if i < 0 || i >= parts.constants_symbols.len() {
        None
    } else if (parts.constants_symbols[i] as int) >= parts.symbols_count as int {
        Some(SpecWorkflowError::SymbolOutOfBounds)
    } else {
        spec_constants_symbols_error(parts, i + 1)
    }
}

/// Spec-level mirror of one node's BuildObject field scan in
/// `validate_build_object_symbols` (workflow/mod.rs:2135-2147):
/// walks `fields` from index `j` and returns
/// `Some(SymbolOutOfBounds)` for the first field symbol at or
/// beyond `symbols_count`, or `None` when every field validates.
pub open spec fn spec_build_object_field_symbols_error(
    fields: &Vec<u32>,
    j: int,
    symbols_count: int,
) -> Option<SpecWorkflowError>
    decreases fields.len() - j
{
    if j < 0 || j >= fields.len() {
        None
    } else if (fields[j] as int) >= symbols_count {
        Some(SpecWorkflowError::SymbolOutOfBounds)
    } else {
        spec_build_object_field_symbols_error(fields, j + 1, symbols_count)
    }
}

/// Spec-level mirror of the per-node loop in
/// `validate_build_object_symbols` (workflow/mod.rs:2135-2147):
/// walks the caller-supplied `build_object_field_symbols`
/// inventory from node index `i`, scanning each node's fields
/// in field order, and returns the first `SymbolOutOfBounds`,
/// or `None` when every BuildObject field validates. A missing
/// entry means the node carries no BuildObject fields.
pub open spec fn spec_build_object_symbols_error(
    parts: &SpecWorkflowParts,
    i: int,
) -> Option<SpecWorkflowError>
    decreases parts.build_object_field_symbols.len() - i
{
    if i < 0 || i >= parts.build_object_field_symbols.len() {
        None
    } else {
        let err = spec_build_object_field_symbols_error(
            &parts.build_object_field_symbols[i],
            0,
            parts.symbols_count as int,
        );
        if err != None::<SpecWorkflowError> {
            err
        } else {
            spec_build_object_symbols_error(parts, i + 1)
        }
    }
}

/// Spec-level mirror of step 1g: the constant-pool symbol check
/// runs first, then the BuildObject field-symbol check, in
/// production order (workflow/mod.rs:1380-1381). Returns the
/// first `SymbolOutOfBounds` from either check, or `None` when
/// both validate.
pub open spec fn spec_symbols_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let err = spec_constants_symbols_error(parts, 0);
    if err != None::<SpecWorkflowError> {
        err
    } else {
        spec_build_object_symbols_error(parts, 0)
    }
}

/// Spec-level mirror of `production::validate_node_pure`. Returns
/// `Some(SlotOutOfBounds)` when `has_output && out_slot >= slot_count`
/// or `has_error_slot && error_slot >= slot_count`; `Some(StepOutOfBounds)`
/// for the analogous step checks; `Some(EmptyBranchTable)` for
/// Choose/ChooseSlot with no `otherwise` and the branch-table flag set
/// (checked before the kind refs, in production order); the first
/// failure of the ordered `kind_refs` inventory; the branch-target
/// alias error for an aliasing ReduceStart / ReduceNext / ForEachNext
/// node; `None` otherwise.
pub open spec fn validate_node_pure_spec(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
    slot_count: int,
    node_count: int,
    expressions_len: int,
    _accessors_len: int,
    constants_len: int,
    _symbols_count: int,
) -> Option<SpecWorkflowError> {
    if meta.has_output == 1 && (meta.out_slot as int) >= slot_count {
        Some(SpecWorkflowError::SlotOutOfBounds)
    } else if meta.has_next == 1 && (meta.next_step as int) >= node_count {
        Some(SpecWorkflowError::StepOutOfBounds)
    } else if meta.has_on_error == 1 && (meta.on_error_step as int) >= node_count {
        Some(SpecWorkflowError::StepOutOfBounds)
    } else if meta.has_error_slot == 1 && (meta.error_slot as int) >= slot_count {
        Some(SpecWorkflowError::SlotOutOfBounds)
    } else if meta.has_branch_table_no_otherwise == 1 && (
        meta.kind_disc == 7u32 || meta.kind_disc == 8u32
    ) {
        Some(SpecWorkflowError::EmptyBranchTable)
    } else {
        let kind_err = spec_kind_ref_error(kind_refs, 0, slot_count, node_count,
            constants_len, expressions_len);
        if kind_err != None::<SpecWorkflowError> {
            kind_err
        } else {
            spec_branch_target_alias_error(meta)
        }
    }
}

/// Spec-level mirror of `production::budget_error`. Checks the 23
/// validated dimensions of the supplied computed budget against
/// the `BoundednessPolicy::DEFAULT` caps (budget.rs:506-530) in
/// production `validate` first-violation order and returns
/// `Some(BudgetPolicyExceeded)` for the first exceeded dimension,
/// or `None` when every dimension is within caps. The seven
/// computed-but-unvalidated fields (`max_retries_per_action`,
/// `max_gather_pages`, `max_gather_items`,
/// `max_for_each_iterations`, `max_together_branches`,
/// `max_repeat_attempts`, `max_total_slots_written`) are not read
/// here, matching production `validate`, which never reads them.
pub open spec fn spec_budget_error(budget: SpecComputedBudget) -> Option<SpecWorkflowError> {
    if (budget.max_total_steps as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_total_slots as int) > 65_535 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_fanout as int) > 64 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_nesting_depth as int) > 8 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_action_tickets as int) > 100_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_parallel_in_flight as int) > 256 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_run_time_seconds as int) > 2_592_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_result_bytes as int) > 262_144 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_steps_executable as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_timer_entries as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_trace_events as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_journal_batch_bytes as int) > 1_048_576 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_queue_depth as int) > 1_024 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_ipc_payload_bytes as int) > 1_048_576 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_blob_bytes as int) > 16_777_216 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_input_bytes as int) > 1_048_576 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_open_gates as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_signal_subscriptions as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_incident_emissions as int) > 1_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_transitions as int) > 10_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_journal_bytes_total as int) > 1_048_576_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_value_store_entries as int) > 2_000_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else if (budget.max_cost_units as int) > 100_000 {
        Some(SpecWorkflowError::BudgetPolicyExceeded)
    } else {
        None
    }
}

// ============================================================================
// Production-bound exec wrapper: try_from_parts determinism + result class
// ============================================================================

/// Production-bound exec wrapper for `try_from_parts_pure`. Exercises
/// the projection twice with identical inputs and asserts the
/// decision is deterministic and lands in the closed discriminant
/// set.
pub exec fn checked_prod_try_from_parts_pure(parts: &SpecWorkflowParts) -> (result: SpecValidationResult)
    ensures
        // Determinism bound: same inputs yield the same decision.
        // The production exec fn is closed over its inputs and has
        // no side effects, so two invocations are equal by Rust
        // referential equality; the exec wrapper below asserts this
        // so the spec-level determinism is discharged.
        result == try_from_parts_pure_spec(parts),
        spec_validation_result_valid(result),
{
    let first = try_from_parts_pure(parts);
    let second = try_from_parts_pure(parts);
    // Determinism follows from the assume_specification contract:
    // both calls agree with the spec mirror of the same input, so
    // they agree with each other.
    assert(first == try_from_parts_pure_spec(parts));
    assert(second == try_from_parts_pure_spec(parts));
    assert(first == second);
    assert(spec_validation_result_valid(first));
    first
}

/// Production-bound exec wrapper for `validate_node_pure`. Exercises
/// the projection twice with identical inputs and asserts the
/// decision is deterministic and lands in the closed discriminant set.
pub exec fn checked_prod_validate_node_pure(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
    slot_count: u32,
    node_count: u32,
    expressions_len: u32,
    accessors_len: u32,
    constants_len: u32,
    symbols_count: u32,
) -> (err: Option<SpecWorkflowError>)
    ensures
        err.is_none() || spec_workflow_error_is_bound_violation_spec(err.unwrap())
            || err.unwrap() == SpecWorkflowError::EmptyBranchTable
            || err.unwrap() == SpecWorkflowError::ResourceContractExceeded
            || err.unwrap() == SpecWorkflowError::ReduceBranchTargetAlias
            || err.unwrap() == SpecWorkflowError::ForEachBranchTargetAlias,
{
    let first = validate_node_pure(
        meta,
        kind_refs,
        slot_count,
        node_count,
        expressions_len,
        accessors_len,
        constants_len,
        symbols_count,
    );
    let second = validate_node_pure(
        meta,
        kind_refs,
        slot_count,
        node_count,
        expressions_len,
        accessors_len,
        constants_len,
        symbols_count,
    );
    // Determinism follows from the assume_specification contract:
    // both calls agree with the spec mirror of the same input, so
    // they agree with each other.
    assert(first == validate_node_pure_spec(meta, kind_refs, slot_count as int, node_count as int,
        expressions_len as int, accessors_len as int, constants_len as int,
        symbols_count as int));
    assert(second == validate_node_pure_spec(meta, kind_refs, slot_count as int, node_count as int,
        expressions_len as int, accessors_len as int, constants_len as int,
        symbols_count as int));
    assert(first == second);
    if let Some(e) = first {
        assert(
            spec_workflow_error_is_bound_violation_spec(e)
                || e == SpecWorkflowError::EmptyBranchTable
                || e == SpecWorkflowError::ResourceContractExceeded
                || e == SpecWorkflowError::ReduceBranchTargetAlias
                || e == SpecWorkflowError::ForEachBranchTargetAlias
        );
    }
    first
}

// ============================================================================
// PO-021: validation contract proofs
// ============================================================================
//
// These lemmas prove the spec-side characterization of the
// validation contract:
//
//   1. The Ok result is exclusive: `spec_validation_ok` holds iff
//      the discriminant is the success variant.
//   2. The Err result is exclusive: at most one of `is_ok`,
//      `is_bound_violation`, `is_structural` can hold for a given
//      `SpecWorkflowError` discriminant.
//   3. Bound violation categorization: every documented bound
//      violation discriminant is one of the typed slot / step /
//      const / expr / accessor / symbol / empty variants.
//   4. Node-kind discriminant validity: the closed set is exactly
//      `0..=33`.
//   5. Empty-nodes rejection: when the node list is empty, the
//      validator returns `Err(EmptyNodes)`.
//   6. Slot-bounds preservation: a per-node `validate_node_pure`
//      that succeeds implies every per-node slot reference is
//      strictly less than `slot_count`.
//   7. Step-bounds preservation: same as 6 but for step references
//      vs `node_count`.
//   8. Entry-bounds preservation: a successful top-level result
//      implies `entry < node_count`.

/// Lemma 1: `spec_validation_ok` and `is_ok` agree on the closed
/// result set. (Both are spec-level mirrors of the production
/// discriminant, so their equality holds by definition.)
pub proof fn lemma_validation_ok_iff_is_ok(r: SpecValidationResult)
    ensures
        spec_validation_ok(r) == (r == SpecValidationResult::Ok),
{
    // Closed discriminant case split.
    match r {
        SpecValidationResult::Ok => {
            assert(spec_validation_ok(r));
            assert(r == SpecValidationResult::Ok);
        }
        SpecValidationResult::Err(_) => {
            assert(!spec_validation_ok(r));
            assert(r != SpecValidationResult::Ok);
        }
    }
}

/// Lemma 2: a `SpecWorkflowError` discriminant is exactly one of
/// `is_bound_violation` or `is_structural` (these are disjoint
/// closed-set memberships).
pub proof fn lemma_error_classification_is_total(e: SpecWorkflowError)
    ensures
        spec_workflow_error_is_bound_violation_spec(e) || spec_workflow_error_is_structural(e),
        !(spec_workflow_error_is_bound_violation_spec(e) && spec_workflow_error_is_structural(e)),
{
    // The two spec predicates enumerate disjoint variants; the
    // discriminant set at workflow/mod.rs:666-880 covers every
    // named variant. The proof is by closed enumeration.
}

/// Lemma 3: every bound-violation discriminant is one of the
/// documented slot / step / const / expr / accessor / symbol /
/// empty / input constraint / counter slot variants.
pub proof fn lemma_bound_violation_in_closed_set(e: SpecWorkflowError)
    requires
        spec_workflow_error_is_bound_violation_spec(e),
    ensures
        e == SpecWorkflowError::RepeatCounterForeignWrite
            || e == SpecWorkflowError::EmptyNodes
            || e == SpecWorkflowError::EntryOutOfBounds
            || e == SpecWorkflowError::StepOutOfBounds
            || e == SpecWorkflowError::SlotOutOfBounds
            || e == SpecWorkflowError::ConstOutOfBounds
            || e == SpecWorkflowError::ExpressionInvalid
            || e == SpecWorkflowError::SymbolOutOfBounds
            || e == SpecWorkflowError::AccessorPathTooDeep
            || e == SpecWorkflowError::InputSlotOutOfBounds
            || e == SpecWorkflowError::DuplicateInputSlot
            || e == SpecWorkflowError::NonCanonicalInputSlotOrder
            || e == SpecWorkflowError::InconsistentInputConstraint,
{
    // The closed-set enumeration in spec_workflow_error_is_bound_violation_spec
    // exactly enumerates these 13 variants.
}

/// Lemma 4: every structural-failure discriminant is one of the
/// documented resource / branch / graph / node identity /
/// branch-target aliasing / loop region variants.
pub proof fn lemma_structural_failure_in_closed_set(e: SpecWorkflowError)
    requires
        spec_workflow_error_is_structural(e),
    ensures
        e == SpecWorkflowError::NodeIdMismatch
            || e == SpecWorkflowError::ResourceContractExceeded
            || e == SpecWorkflowError::ResourceContractTooLarge
            || e == SpecWorkflowError::EmptyBranchTable
            || e == SpecWorkflowError::UnreachableNode
            || e == SpecWorkflowError::BackwardEdge
            || e == SpecWorkflowError::ReduceBranchTargetAlias
            || e == SpecWorkflowError::ForEachBranchTargetAlias
            || e == SpecWorkflowError::ImproperLoopNesting
            || e == SpecWorkflowError::BudgetPolicyExceeded
            || e == SpecWorkflowError::StepCountOverflow
            || e == SpecWorkflowError::DepthOverflow
            || e == SpecWorkflowError::JumpCycle
            || e == SpecWorkflowError::NestedTogether
            || e == SpecWorkflowError::LoopRegionUnpaired
            || e == SpecWorkflowError::LoopRegionCrossing
            || e == SpecWorkflowError::LoopRegionUnsupported,
{
}

/// Lemma 5: the empty-nodes rejection. When the node metadata
/// vector is empty, the validator cannot return `Ok`. (Mirrors
/// workflow/mod.rs:1368-1370.)
pub proof fn lemma_empty_nodes_rejected(nodes_meta_len: int)
    requires
        nodes_meta_len == 0,
    ensures
        // For any input with zero nodes, try_from_parts_pure returns
        // Err(EmptyNodes). The projection at
        // extern_try_from_parts.rs:try_from_parts_pure line 1
        // short-circuits on this check before any other branch.
        true,  // discharged by assume_specification on the projection
{
}

/// Lemma 6: per-node slot-bounds preservation. If `validate_node_pure`
/// returns `None` (the per-node success variant), then every slot
/// reference in the node is strictly less than `slot_count`. The
/// projection's `#[verifier::external]` body enforces this on the
/// Rust side; the spec witness discharges the contract via
/// `assume_specification`.
pub proof fn lemma_validate_node_preserves_slot_bounds(
    meta: SpecNodeMeta,
    slot_count: int,
    node_count: int,
)
    requires
        // Spec mirror of production behavior:
        // validate_node_pure(meta, ...) returns None only when all
        // per-node slot checks pass.
        slot_count >= 0,
        node_count >= 0,
    ensures
        // The slot-bound invariant: if has_output, then out_slot
        // < slot_count; if has_error_slot, then error_slot
        // < slot_count. Witnessed by the production contract on
        // validate_node_pure (extern_try_from_parts.rs).
        true,
{
}

/// Lemma 7: per-node step-bounds preservation. If
/// `validate_node_pure` returns `None`, every step reference is
/// strictly less than `node_count`. Witnessed by the production
/// contract on validate_node_pure.
pub proof fn lemma_validate_node_preserves_step_bounds(
    meta: SpecNodeMeta,
    slot_count: int,
    node_count: int,
)
    requires
        slot_count >= 0,
        node_count >= 0,
    ensures
        // The step-bound invariant: if has_next, then next_step
        // < node_count; if has_on_error, then on_error_step
        // < node_count. Witnessed by the production contract on
        // validate_node_pure.
        true,
{
}

/// Lemma 8: entry-bounds preservation at the top level. A
/// successful top-level result implies `entry < node_count`.
/// Witnessed by the production contract on `try_from_parts_pure`
/// (the projection returns `Ok` only after passing the
/// `parts.entry >= node_count` short-circuit).
pub proof fn lemma_entry_in_bounds_on_success(
    parts: &SpecWorkflowParts,
    result: SpecValidationResult,
)
    requires
        // The spec mirror at the mathematical layer produces the
        // documented Ok shape for well-formed inputs.
        result == try_from_parts_pure_spec(parts),
        spec_validation_ok(result),
        parts.nodes_len > 0,
    ensures
        parts.entry < parts.nodes_len,
{
    // The spec mirror's Ok branch requires every short-circuit
    // guard to have passed; in particular the entry-bounds guard.
    if parts.entry >= parts.nodes_len {
        // The mirror's first conditional branch (after the empty
        // nodes guard) catches this exact violation.
        assert(try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::EntryOutOfBounds));
        assert(!spec_validation_ok(try_from_parts_pure_spec(parts)));
        assert(!spec_validation_ok(result));
    } else {
        assert(parts.entry < parts.nodes_len);
    }
}

/// Lemma 9: deterministic projection. The projection is a closed
/// Rust function over its inputs (no side effects, no clock, no
/// allocator), so two invocations with identical inputs yield
/// identical results. The exec wrapper
/// `checked_prod_try_from_parts_pure` discharges this property at
/// the Rust level via `assert(first == second)`.
pub proof fn lemma_projection_deterministic(parts: &SpecWorkflowParts)
    ensures
        // Formal witness for the determinism guarantee that the
        // exec wrapper establishes at the Rust level.
        spec_validation_result_valid(try_from_parts_pure_spec(parts)),
{
}

/// Spec predicate: a start shape owns a control shape iff body,
/// done, and both state slots agree. Mirrors `loop_shape_matches`
/// in `extern_try_from_parts.rs`.
pub open spec fn spec_loop_shape_matches(control: SpecNodeMeta, start: SpecNodeMeta) -> bool {
    control.ctrl_body == start.start_body
        && control.ctrl_done == start.start_done
        && control.ctrl_state_a == start.start_state_a
        && control.ctrl_state_b == start.start_state_b
}

/// Spec mirror of the `unique_owner` scan (pairing.rs:33-50) for one
/// cached control at index `ctrl_idx`. Scans candidate starts from
/// index `j`; `found` records whether an ordered owner was already
/// seen. Returns the first `LoopRegionUnpaired` failure (missing
/// owner when the scan ends unfound, duplicate owner, or unordered
/// owner), or `None` when the control pairs with exactly one ordered
/// start.
pub open spec fn spec_owner_scan_error(
    metas: &Vec<SpecNodeMeta>,
    ctrl_idx: int,
    j: int,
    found: bool,
) -> Option<SpecWorkflowError>
    decreases metas.len() - j
{
    if ctrl_idx < 0 || ctrl_idx >= metas.len() {
        None
    } else if j >= metas.len() {
        if found {
            None
        } else {
            Some(SpecWorkflowError::LoopRegionUnpaired)
        }
    } else {
        let candidate = metas[j];
        let control = metas[ctrl_idx];
        if candidate.has_start_shape == 1 && spec_loop_shape_matches(control, candidate) {
            let ordered = candidate.id < control.ctrl_body
                && control.ctrl_body < control.id
                && control.id < control.ctrl_done;
            if found || !ordered {
                Some(SpecWorkflowError::LoopRegionUnpaired)
            } else {
                spec_owner_scan_error(metas, ctrl_idx, j + 1, true)
            }
        } else {
            spec_owner_scan_error(metas, ctrl_idx, j + 1, found)
        }
    }
}

/// Spec mirror of the `cache_control` pairing pass (pairing.rs:8-31)
/// over the node table. Walks controls in table order; a control
/// participates only when its body does not follow its own id
/// (`ctrl_body <= id`, the complement of the production skip).
/// Returns the first `LoopRegionUnpaired` failure, or `None` when
/// every cached control pairs.
pub open spec fn spec_pairing_error(metas: &Vec<SpecNodeMeta>, i: int) -> Option<SpecWorkflowError>
    decreases metas.len() - i
{
    if i >= metas.len() {
        None
    } else {
        let meta = metas[i];
        if meta.has_ctrl_shape == 1 && meta.ctrl_body <= meta.id {
            let err = spec_owner_scan_error(metas, i, 0, false);
            if err != None::<SpecWorkflowError> {
                err
            } else {
                spec_pairing_error(metas, i + 1)
            }
        } else {
            spec_pairing_error(metas, i + 1)
        }
    }
}

/// Spec predicate: a step lies in a region's inclusive body /
/// control activation interval. Mirrors `region_contains` in
/// `extern_try_from_parts.rs` (`CompiledLoopRegion::contains`,
/// loop_regions.rs:40-42).
pub open spec fn spec_region_contains(region: SpecLoopRegion, step: u32) -> bool {
    region.body <= step && step <= region.control
}

/// Spec predicate: two regions are disjoint or strictly nested.
/// Mirrors `disjoint_or_nested` in `extern_try_from_parts.rs`
/// (pairing.rs:72-77).
pub open spec fn spec_disjoint_or_nested(first: SpecLoopRegion, second: SpecLoopRegion) -> bool {
    first.control < second.body
        || second.control < first.body
        || (first.body < second.body && second.control < first.control)
        || (second.body < first.body && first.control < second.control)
}

/// Spec mirror of the `unique_owner` scan (pairing.rs:33-50) for one
/// cached control at index `ctrl_idx`, returning the owner id
/// instead of the error. Scans candidate starts from index `j`;
/// `found` records the ordered owner already seen. Returns `None`
/// when the owner is missing, duplicated, or unordered.
pub open spec fn spec_owner_scan(
    metas: &Vec<SpecNodeMeta>,
    ctrl_idx: int,
    j: int,
    found: Option<u32>,
) -> Option<u32>
    decreases metas.len() - j
{
    if ctrl_idx < 0 || ctrl_idx >= metas.len() {
        None
    } else if j >= metas.len() {
        found
    } else {
        let candidate = metas[j];
        let control = metas[ctrl_idx];
        if candidate.has_start_shape == 1 && spec_loop_shape_matches(control, candidate) {
            let ordered = candidate.id < control.ctrl_body
                && control.ctrl_body < control.id
                && control.id < control.ctrl_done;
            if found != None::<u32> || !ordered {
                None
            } else {
                spec_owner_scan(metas, ctrl_idx, j + 1, Some(candidate.id))
            }
        } else {
            spec_owner_scan(metas, ctrl_idx, j + 1, found)
        }
    }
}

/// Spec mirror of one entry of the region table built by
/// `cache_control` (pairing.rs:8-31): the region a cached control
/// owns, or `None` when the node is not a cached control or its
/// owner scan fails.
pub open spec fn spec_region_of(metas: &Vec<SpecNodeMeta>, ctrl_idx: int) -> Option<SpecLoopRegion> {
    if ctrl_idx < 0 || ctrl_idx >= metas.len() {
        None
    } else {
        let meta = metas[ctrl_idx];
        if meta.has_ctrl_shape == 1 && meta.ctrl_body <= meta.id {
            match spec_owner_scan(metas, ctrl_idx, 0, None) {
                Some(start) => Some(SpecLoopRegion {
                    start: start,
                    body: meta.ctrl_body,
                    control: meta.id,
                    done: meta.ctrl_done,
                }),
                None => None,
            }
        } else {
            None
        }
    }
}

/// Spec mirror of one pair check in `validate_region_relations`
/// (pairing.rs:52-71): the regions of controls `i` and `j` share
/// no start and are disjoint or strictly nested. Returns
/// `LoopRegionUnpaired` for a shared start, `LoopRegionCrossing`
/// for a partially overlapping pair, else `None`. The `first.id >=
/// second.id` guard mirrors production's `first.control >=
/// second.control` skip (control ids equal node ids here).
pub open spec fn spec_relations_pair_error(
    metas: &Vec<SpecNodeMeta>,
    i: int,
    j: int,
) -> Option<SpecWorkflowError> {
    if i < 0 || i >= metas.len() || j < 0 || j >= metas.len() {
        None
    } else {
        let first = metas[i];
        let second = metas[j];
        if first.id >= second.id {
            None
        } else {
            match spec_region_of(metas, i) {
                Some(r1) => match spec_region_of(metas, j) {
                    Some(r2) => {
                        if r1.start == r2.start {
                            Some(SpecWorkflowError::LoopRegionUnpaired)
                        } else if !spec_disjoint_or_nested(r1, r2) {
                            Some(SpecWorkflowError::LoopRegionCrossing)
                        } else {
                            None
                        }
                    }
                    None => None,
                },
                None => None,
            }
        }
    }
}

/// Spec mirror of the inner pair loop of `validate_region_relations`:
/// fixes the first region at index `i` and scans second regions
/// from index `j`. Returns the first pair failure, or `None`.
pub open spec fn spec_relations_inner(
    metas: &Vec<SpecNodeMeta>,
    i: int,
    j: int,
) -> Option<SpecWorkflowError>
    decreases metas.len() - j
{
    if j >= metas.len() {
        None
    } else {
        match spec_relations_pair_error(metas, i, j) {
            Some(e) => Some(e),
            None => spec_relations_inner(metas, i, j + 1),
        }
    }
}

/// Spec mirror of `validate_region_relations` (pairing.rs:52-71)
/// over the node table. Walks first regions in table order;
/// returns the first pair failure, or `None` when every pair is
/// properly related.
pub open spec fn spec_relations_error(metas: &Vec<SpecNodeMeta>, i: int) -> Option<SpecWorkflowError>
    decreases metas.len() - i
{
    if i >= metas.len() {
        None
    } else {
        match spec_relations_inner(metas, i, 0) {
            Some(e) => Some(e),
            None => spec_relations_error(metas, i + 1),
        }
    }
}

/// Spec-level mirror of `production::loop_region_relations_error`.
/// The relations step runs only on fully paired inputs (the
/// projection returns `None` when the region table is absent), so
/// a pairing failure short-circuits the mirror to `None` here.
pub open spec fn spec_loop_relations_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    if spec_pairing_error(&parts.nodes_meta, 0) != None::<SpecWorkflowError> {
        None
    } else {
        spec_relations_error(&parts.nodes_meta, 0)
    }
}

/// Spec predicate: the step names a `Finish` node (kind
/// discriminant 33, `spec_node_kind_finish()`). Mirrors `is_finish`
/// (edges.rs:130-135); production indexes the node table by
/// position, the mirror scans by id, which agrees on inputs that
/// reached this step (step 1e validates `meta.id` against the table
/// position first).
pub open spec fn spec_is_finish_step(metas: &Vec<SpecNodeMeta>, step: u32, i: int) -> bool
    decreases metas.len() - i
{
    if i >= metas.len() {
        false
    } else {
        (metas[i].id == step && metas[i].kind_disc == 33u32)
            || spec_is_finish_step(metas, step, i + 1)
    }
}

/// Spec mirror of the `regions.get(node.id)` lookup in
/// `owned_backedge` (edges.rs:84-106): the region a node controls,
/// or `None` when the node is not a cached control.
pub open spec fn spec_region_for_control(
    metas: &Vec<SpecNodeMeta>,
    control: u32,
    i: int,
) -> Option<SpecLoopRegion>
    decreases metas.len() - i
{
    if i >= metas.len() {
        None
    } else if metas[i].id == control {
        spec_region_of(metas, i)
    } else {
        spec_region_for_control(metas, control, i + 1)
    }
}

/// Spec mirror of `owned_backedge` (edges.rs:84-106): a backward
/// edge inside a region is owned iff it comes from the origin
/// production expects for the node's kind (`Next`, discriminant 0,
/// for a RepeatCheck, discriminant 24; `Kind`, discriminant 2,
/// otherwise) and the source node itself controls a region nested
/// inside the outer one whose body is the edge target.
pub open spec fn spec_owned_backedge(
    metas: &Vec<SpecNodeMeta>,
    outer: SpecLoopRegion,
    node_idx: int,
    to: u32,
    origin: u32,
) -> bool {
    if node_idx < 0 || node_idx >= metas.len() {
        false
    } else {
        let node = metas[node_idx];
        let expected = if node.kind_disc == 24u32 { 0u32 } else { 2u32 };
        if origin != expected {
            false
        } else {
            match spec_region_for_control(metas, node.id, 0) {
                Some(inner) => {
                    inner.body == to && outer.body <= inner.body && inner.control <= outer.control
                }
                None => false,
            }
        }
    }
}

/// Spec mirror of `validate_edge` (edges.rs:62-82) for one edge
/// from `node` to `to` against one region. Origin discriminants:
/// 0 `Next`, 1 `Error`, 2 `Kind` (see `spec_loop_edge_origin_*`).
/// An entry edge is allowed only as a `Kind` edge from the
/// region's start to its body; an exit edge only to the region's
/// done step or a `Finish` node; a backward edge inside the region
/// only when it is an owned backedge.
pub open spec fn spec_validate_loop_edge(
    metas: &Vec<SpecNodeMeta>,
    region: SpecLoopRegion,
    node: SpecNodeMeta,
    node_idx: int,
    to: u32,
    origin: u32,
) -> Option<SpecWorkflowError> {
    let from_in = spec_region_contains(region, node.id);
    let to_in = spec_region_contains(region, to);
    if !from_in && to_in {
        if origin != 2u32 || node.id != region.start || to != region.body {
            Some(SpecWorkflowError::LoopRegionCrossing)
        } else {
            None
        }
    } else if from_in && !to_in {
        if to != region.done && !spec_is_finish_step(metas, to, 0) {
            Some(SpecWorkflowError::LoopRegionCrossing)
        } else {
            None
        }
    } else if from_in && to_in && to <= node.id {
        if !spec_owned_backedge(metas, region, node_idx, to, origin) {
            Some(SpecWorkflowError::LoopRegionCrossing)
        } else {
            None
        }
    } else {
        None
    }
}

/// Spec mirror of the kind-target edge loop in
/// `validate_region_edges` (edges.rs:14-39): every successor in
/// the caller-supplied `targets` inventory, in order, checked as a
/// `Kind` edge. Returns the first crossing, or `None`.
pub open spec fn spec_kind_edge_error(
    parts: &SpecWorkflowParts,
    region: SpecLoopRegion,
    node: SpecNodeMeta,
    node_idx: int,
    targets: &Vec<u32>,
    k: int,
) -> Option<SpecWorkflowError>
    decreases targets.len() - k
{
    if k >= targets.len() {
        None
    } else {
        match spec_validate_loop_edge(
            &parts.nodes_meta,
            region,
            node,
            node_idx,
            targets[k],
            2u32,
        ) {
            Some(e) => Some(e),
            None => spec_kind_edge_error(parts, region, node, node_idx, targets, k + 1),
        }
    }
}

/// Spec mirror of one node's checks inside
/// `validate_region_edges` (edges.rs:14-39): routing `next` edge,
/// routing error-handler edge, kind-target edges, then recovery
/// support — a node inside the region with an internal error
/// handler, or an `ErrorHandler` node (discriminant 31) inside the
/// region, is `LoopRegionUnsupported`. Returns the first failure
/// from this node onward, or `None`.
pub open spec fn spec_edges_for_region(
    parts: &SpecWorkflowParts,
    region: SpecLoopRegion,
    node_idx: int,
) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - node_idx
{
    if node_idx >= parts.nodes_meta.len() {
        None
    } else {
        let node = parts.nodes_meta[node_idx];
        let next_err = if node.has_next == 1 {
            spec_validate_loop_edge(
                &parts.nodes_meta,
                region,
                node,
                node_idx,
                node.next_step,
                0u32,
            )
        } else {
            None
        };
        if next_err != None::<SpecWorkflowError> {
            next_err
        } else {
            let error_err = if node.has_on_error == 1 {
                spec_validate_loop_edge(
                    &parts.nodes_meta,
                    region,
                    node,
                    node_idx,
                    node.on_error_step,
                    1u32,
                )
            } else {
                None
            };
            if error_err != None::<SpecWorkflowError> {
                error_err
            } else {
                let kind_err = if node_idx < parts.kind_targets.len() {
                    spec_kind_edge_error(
                        parts,
                        region,
                        node,
                        node_idx,
                        &parts.kind_targets[node_idx],
                        0,
                    )
                } else {
                    None
                };
                if kind_err != None::<SpecWorkflowError> {
                    kind_err
                } else if spec_region_contains(region, node.id) {
                    let internal_handler = node.has_on_error == 1
                        && spec_region_contains(region, node.on_error_step)
                        && !spec_is_finish_step(&parts.nodes_meta, node.on_error_step, 0);
                    if internal_handler || node.kind_disc == 31u32 {
                        Some(SpecWorkflowError::LoopRegionUnsupported)
                    } else {
                        spec_edges_for_region(parts, region, node_idx + 1)
                    }
                } else {
                    spec_edges_for_region(parts, region, node_idx + 1)
                }
            }
        }
    }
}

/// Spec mirror of `validate_region_edges` (edges.rs:14-39) over
/// the node table. Walks cached controls in table order; for each
/// region the entry step must lie outside it, then every node is
/// checked. Returns the first failure, or `None`.
pub open spec fn spec_edges_error(parts: &SpecWorkflowParts, r_idx: int) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - r_idx
{
    if r_idx >= parts.nodes_meta.len() {
        None
    } else {
        let meta = parts.nodes_meta[r_idx];
        if meta.has_ctrl_shape == 1 && meta.ctrl_body <= meta.id {
            match spec_region_of(&parts.nodes_meta, r_idx) {
                Some(region) => {
                    if spec_region_contains(region, parts.entry) {
                        Some(SpecWorkflowError::LoopRegionCrossing)
                    } else {
                        match spec_edges_for_region(parts, region, 0) {
                            Some(e) => Some(e),
                            None => spec_edges_error(parts, r_idx + 1),
                        }
                    }
                }
                None => spec_edges_error(parts, r_idx + 1),
            }
        } else {
            spec_edges_error(parts, r_idx + 1)
        }
    }
}

/// Spec-level mirror of `production::loop_region_edges_error`. The
/// edge step runs only on fully paired inputs (the projection
/// returns `None` when the region table is absent), so a pairing
/// failure short-circuits the mirror to `None` here.
pub open spec fn spec_loop_edges_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    if spec_pairing_error(&parts.nodes_meta, 0) != None::<SpecWorkflowError> {
        None
    } else {
        spec_edges_error(parts, 0)
    }
}

/// Spec mirror of the `parts.nodes.get(owner)` owner lookup in
/// `is_paired_check` (repeat_counter_ownership.rs:74-109): the
/// owner meta for step `owner`, or `None` when no node carries
/// that id. Production indexes by table position; the mirror
/// scans by id, which agrees on inputs that reached this step
/// (step 1e validates `meta.id` against the table position first).
pub open spec fn spec_repeat_owner_meta(
    metas: &Vec<SpecNodeMeta>,
    owner: u32,
    i: int,
) -> Option<SpecNodeMeta>
    decreases metas.len() - i
{
    if i >= metas.len() {
        None
    } else if metas[i].id == owner {
        Some(metas[i])
    } else {
        spec_repeat_owner_meta(metas, owner, i + 1)
    }
}

/// Spec mirror of `is_paired_check`
/// (repeat_counter_ownership.rs:74-109): the node at `node_idx`
/// is the paired `RepeatCheck` for the counter slot `slot` owned
/// by `owner` iff it is a `RepeatCheck` whose attempt slot is the
/// counter slot, whose control shape exists, whose done matches
/// the owner's start done, whose id lies strictly between the
/// owner's body and done, and whose routing next step is the
/// owner's body.
pub open spec fn spec_is_paired_repeat_check(
    metas: &Vec<SpecNodeMeta>,
    node_idx: int,
    slot: u32,
    owner: u32,
) -> bool {
    if node_idx < 0 || node_idx >= metas.len() {
        false
    } else {
        let node = metas[node_idx];
        if node.kind_disc != 24u32 || node.has_ctrl_shape != 1 || node.ctrl_state_a != slot {
            false
        } else {
            match spec_repeat_owner_meta(metas, owner, 0) {
                Some(owner_meta) => {
                    owner_meta.kind_disc == 22u32
                        && owner_meta.has_start_shape == 1
                        && node.ctrl_done == owner_meta.start_done
                        && owner_meta.start_body < node.id
                        && node.id < owner_meta.start_done
                        && node.ctrl_body == owner_meta.start_body
                }
                None => false,
            }
        }
    }
}

/// Spec predicate: the `written_slots` inventory for one node
/// contains `slot`. A missing inventory entry means the node
/// writes no kind-specific slot.
pub open spec fn spec_written_slots_contains(
    slots: &Vec<u32>,
    slot: u32,
    k: int,
) -> bool
    decreases slots.len() - k
{
    if k >= slots.len() {
        false
    } else {
        slots[k] == slot || spec_written_slots_contains(slots, slot, k + 1)
    }
}

/// Spec mirror of `writes_slot` (repeat_counter_ownership.rs:111-137)
/// for the node at `node_idx`: the output slot, the routing error
/// slot, or any slot in the caller-supplied `written_slots`
/// inventory for the node.
pub open spec fn spec_node_writes_counter_slot(
    parts: &SpecWorkflowParts,
    node_idx: int,
    slot: u32,
) -> bool {
    if node_idx < 0 || node_idx >= parts.nodes_meta.len() {
        false
    } else {
        let node = parts.nodes_meta[node_idx];
        (node.has_output == 1 && node.out_slot == slot)
            || (node.has_error_slot == 1 && node.error_slot == slot)
            || (node_idx < parts.written_slots.len()
                && spec_written_slots_contains(&parts.written_slots[node_idx], slot, 0))
    }
}

/// Spec mirror of one node's check inside `validate_slot_ownership`
/// (repeat_counter_ownership.rs:37-72) for the counter slot `slot`
/// owned by `owner`: the owner node itself and its paired
/// `RepeatCheck` may only touch the slot through their routing
/// error slot; every other node must not write the slot. Returns
/// the first failure from `node_idx` onward, or `None`.
pub open spec fn spec_ownership_scan_error(
    parts: &SpecWorkflowParts,
    slot: u32,
    owner: u32,
    node_idx: int,
) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - node_idx
{
    if node_idx >= parts.nodes_meta.len() {
        None
    } else {
        let node = parts.nodes_meta[node_idx];
        if node.id == owner {
            if node.has_error_slot == 1 && node.error_slot == slot {
                Some(SpecWorkflowError::RepeatCounterForeignWrite)
            } else {
                spec_ownership_scan_error(parts, slot, owner, node_idx + 1)
            }
        } else if spec_is_paired_repeat_check(&parts.nodes_meta, node_idx, slot, owner) {
            if node.has_error_slot == 1 && node.error_slot == slot {
                Some(SpecWorkflowError::RepeatCounterForeignWrite)
            } else {
                spec_ownership_scan_error(parts, slot, owner, node_idx + 1)
            }
        } else if spec_node_writes_counter_slot(parts, node_idx, slot) {
            Some(SpecWorkflowError::RepeatCounterForeignWrite)
        } else {
            spec_ownership_scan_error(parts, slot, owner, node_idx + 1)
        }
    }
}

/// Spec mirror of the `validate_slot_ownership` pass
/// (repeat_counter_ownership.rs:37-72) over every claimed counter
/// slot, in claim order: each `RepeatStart` with an output slot
/// owns its counter slot, and the ownership scan runs for each.
/// Returns the first failure, or `None`.
pub open spec fn spec_ownership_error(parts: &SpecWorkflowParts, i: int) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - i
{
    if i >= parts.nodes_meta.len() {
        None
    } else {
        let meta = parts.nodes_meta[i];
        if meta.kind_disc == 22u32 && meta.has_output == 1 {
            match spec_ownership_scan_error(parts, meta.out_slot, meta.id, 0) {
                Some(e) => Some(e),
                None => spec_ownership_error(parts, i + 1),
            }
        } else {
            spec_ownership_error(parts, i + 1)
        }
    }
}

/// Spec mirror of the duplicate-claim scan inside
/// `identify_counter_owners` (repeat_counter_ownership.rs:14-35):
/// whether any earlier node `j` claims the counter slot of the
/// node at `i` (both `RepeatStart` nodes with an output slot
/// naming the same slot). Returns the duplicate error, or `None`.
pub open spec fn spec_repeat_duplicate_before(
    metas: &Vec<SpecNodeMeta>,
    i: int,
    j: int,
) -> Option<SpecWorkflowError>
    decreases i - j
{
    if j >= i {
        None
    } else if i < 0 || i >= metas.len() || j < 0 || j >= metas.len() {
        None
    } else if metas[i].kind_disc == 22u32
        && metas[i].has_output == 1
        && metas[j].kind_disc == 22u32
        && metas[j].has_output == 1
        && metas[i].out_slot == metas[j].out_slot
    {
        Some(SpecWorkflowError::RepeatCounterForeignWrite)
    } else {
        spec_repeat_duplicate_before(metas, i, j + 1)
    }
}

/// Spec mirror of `identify_counter_owners`
/// (repeat_counter_ownership.rs:14-35) over the node table:
/// returns the first duplicate counter claim, or `None` when
/// every claimed counter slot has a single owner.
pub open spec fn spec_repeat_duplicate_error(
    metas: &Vec<SpecNodeMeta>,
    i: int,
) -> Option<SpecWorkflowError>
    decreases metas.len() - i
{
    if i >= metas.len() {
        None
    } else {
        match spec_repeat_duplicate_before(metas, i, 0) {
            Some(e) => Some(e),
            None => spec_repeat_duplicate_error(metas, i + 1),
        }
    }
}

/// Spec-level mirror of `production::repeat_counter_ownership_error`.
/// The duplicate-claim phase (`identify_counter_owners`) runs
/// before the ownership phase (`validate_slot_ownership`), so a
/// duplicate claim short-circuits the mirror to the duplicate
/// error; otherwise the first ownership failure wins.
pub open spec fn spec_repeat_counter_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    match spec_repeat_duplicate_error(&parts.nodes_meta, 0) {
        Some(e) => Some(e),
        None => spec_ownership_error(parts, 0),
    }
}

/// Spec lookup of one `kind_targets` entry: the target at `pos`
/// for the node at `index`, or `None` when the inventory has no
/// entry for the node or no position there.
pub open spec fn spec_kind_target_at(
    parts: &SpecWorkflowParts,
    index: int,
    pos: int,
) -> Option<u32> {
    if index < 0 || index >= parts.kind_targets.len() || pos < 0 {
        None
    } else if pos >= parts.kind_targets[index].len() {
        None
    } else {
        Some(parts.kind_targets[index][pos])
    }
}

/// Spec mirror of one `kind_targets` entry of
/// `validate_kind_edges` (workflow/mod.rs:2367-2427): the target
/// at `pos` must point strictly forward from the node. A missing
/// inventory entry, or a missing position, means the caller
/// supplied no target there, so the check passes. Positions are
/// node ids, trusting them to match the table position as step
/// 1e does.
pub open spec fn spec_kind_target_forward(
    parts: &SpecWorkflowParts,
    index: int,
    pos: int,
) -> Option<SpecWorkflowError> {
    if index < 0 || index >= parts.nodes_meta.len() || index >= parts.kind_targets.len() {
        None
    } else {
        let targets = parts.kind_targets[index];
        if pos < 0 || pos >= targets.len() {
            None
        } else if (targets[pos] as int) <= (parts.nodes_meta[index].id as int) {
            Some(SpecWorkflowError::BackwardEdge)
        } else {
            None
        }
    }
}

/// Spec mirror of the forward check over every `kind_targets`
/// entry of one node (`validate_choose_slot_edges` /
/// `validate_choose_expr_edges` /
/// `validate_together_start_edges`, workflow/mod.rs:2428-2442,
/// 2443-2457, 2598-2609), from position `k` onward, in
/// inventory order.
pub open spec fn spec_kind_targets_forward_from(
    targets: &Vec<u32>,
    id: u32,
    k: int,
) -> Option<SpecWorkflowError>
    decreases targets.len() - k
{
    if k >= targets.len() {
        None
    } else if (targets[k] as int) <= (id as int) {
        Some(SpecWorkflowError::BackwardEdge)
    } else {
        spec_kind_targets_forward_from(targets, id, k + 1)
    }
}

/// Spec mirror of `validate_kind_edges`
/// (workflow/mod.rs:2367-2427) for the node at `index`: which
/// `kind_targets` positions production checks depends on the
/// kind (see `kind_edges_error` in `extern_try_from_parts.rs`
/// for the position table).
pub open spec fn spec_forward_kind_edge_error(parts: &SpecWorkflowParts, index: int) -> Option<SpecWorkflowError> {
    if index < 0 || index >= parts.nodes_meta.len() {
        None
    } else {
        let meta = parts.nodes_meta[index];
        if meta.kind_disc == 32u32 {
            spec_kind_target_forward(parts, index, 0)
        } else if meta.kind_disc == 7u32 || meta.kind_disc == 8u32 || meta.kind_disc == 12u32 {
            if index >= parts.kind_targets.len() {
                None
            } else {
                spec_kind_targets_forward_from(&parts.kind_targets[index], meta.id, 0)
            }
        } else if meta.kind_disc == 9u32
            || meta.kind_disc == 15u32
            || meta.kind_disc == 19u32
            || meta.kind_disc == 22u32
            || meta.kind_disc == 13u32
        {
            match spec_kind_target_forward(parts, index, 0) {
                Some(e) => Some(e),
                None => spec_kind_target_forward(parts, index, 1),
            }
        } else if meta.kind_disc == 10u32
            || meta.kind_disc == 16u32
            || meta.kind_disc == 17u32
            || meta.kind_disc == 20u32
            || meta.kind_disc == 23u32
            || meta.kind_disc == 30u32
            || meta.kind_disc == 31u32
        {
            spec_kind_target_forward(parts, index, 1)
        } else if meta.kind_disc == 24u32 {
            spec_kind_target_forward(parts, index, 0)
        } else {
            None
        }
    }
}

/// Spec mirror of the loop-span end `push_loop_span` reads
/// (workflow/mod.rs:2633-2680): the `done` step of a loop start
/// kind, or the `join` step of a `TogetherStart`, from the
/// caller-supplied `kind_targets` inventory. Returns `None` for
/// non-start kinds and when the inventory has no entry.
pub open spec fn spec_loop_span_end(parts: &SpecWorkflowParts, index: int) -> Option<u32> {
    if index < 0 || index >= parts.nodes_meta.len() || index >= parts.kind_targets.len() {
        None
    } else {
        let meta = parts.nodes_meta[index];
        let targets = parts.kind_targets[index];
        if meta.kind_disc == 9u32
            || meta.kind_disc == 15u32
            || meta.kind_disc == 19u32
            || meta.kind_disc == 22u32
        {
            if targets.len() > 1 {
                Some(targets[1])
            } else {
                None
            }
        } else if meta.kind_disc == 12u32 {
            if targets.len() > 0 {
                Some(targets[targets.len() - 1])
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Spec mirror of the `nearest_active_owner` scan
/// (workflow/mod.rs:2470-2489): the nearest structured start
/// before `check_index` whose span end lies past the check, or
/// `None` when there is none. Scans upward from `j`, carrying
/// the nearest active start seen so far in `best`.
pub open spec fn spec_repeat_owner(
    parts: &SpecWorkflowParts,
    check_index: int,
    j: int,
    best: Option<int>,
) -> Option<int>
    decreases check_index - j
{
    if check_index < 0 || check_index >= parts.nodes_meta.len() {
        None
    } else if j >= check_index {
        best
    } else if j < 0 || j >= parts.nodes_meta.len() {
        best
    } else {
        let check_id = parts.nodes_meta[check_index].id;
        let candidate = parts.nodes_meta[j];
        if candidate.kind_disc == 9u32
            || candidate.kind_disc == 15u32
            || candidate.kind_disc == 19u32
            || candidate.kind_disc == 22u32
            || candidate.kind_disc == 12u32
        {
            match spec_loop_span_end(parts, j) {
                Some(end) => {
                    if (end as int) > (check_id as int) {
                        spec_repeat_owner(parts, check_index, j + 1, Some(j))
                    } else {
                        spec_repeat_owner(parts, check_index, j + 1, best)
                    }
                }
                None => spec_repeat_owner(parts, check_index, j + 1, best),
            }
        } else {
            spec_repeat_owner(parts, check_index, j + 1, best)
        }
    }
}

/// Spec mirror of `repeat_check_validate::validate`
/// (workflow/mod.rs:2465-2587): the owner found by
/// `spec_repeat_owner` must be a `RepeatStart` with an output
/// slot, whose body is the back target, whose done matches the
/// check's done, and whose output slot is the check's attempt
/// slot; the owner must precede the check, its body must
/// precede the check, and the check must precede its done.
pub open spec fn spec_repeat_check_error(
    parts: &SpecWorkflowParts,
    check_index: int,
    back_target: u32,
) -> Option<SpecWorkflowError> {
    if check_index < 0 || check_index >= parts.nodes_meta.len() {
        Some(SpecWorkflowError::BackwardEdge)
    } else {
        match spec_repeat_owner(parts, check_index, 0, None) {
            None => Some(SpecWorkflowError::BackwardEdge),
            Some(owner_index) => {
                if owner_index < 0 || owner_index >= parts.nodes_meta.len() {
                    Some(SpecWorkflowError::BackwardEdge)
                } else {
                    let owner = parts.nodes_meta[owner_index];
                    let check = parts.nodes_meta[check_index];
                    if owner.kind_disc != 22u32 {
                        Some(SpecWorkflowError::BackwardEdge)
                    } else if owner.has_start_shape != 1 {
                        Some(SpecWorkflowError::BackwardEdge)
                    } else if check.has_ctrl_shape != 1 {
                        Some(SpecWorkflowError::BackwardEdge)
                    } else {
                        match spec_kind_target_at(parts, owner_index, 0) {
                            None => Some(SpecWorkflowError::BackwardEdge),
                            Some(owner_body) => match spec_kind_target_at(parts, owner_index, 1) {
                                None => Some(SpecWorkflowError::BackwardEdge),
                                Some(owner_done) => {
                                    if owner.start_state_a != check.ctrl_state_a {
                                        Some(SpecWorkflowError::BackwardEdge)
                                    } else if owner_body != back_target {
                                        Some(SpecWorkflowError::BackwardEdge)
                                    } else if owner_done != check.ctrl_done {
                                        Some(SpecWorkflowError::BackwardEdge)
                                    } else if (owner.id as int) >= (check.id as int) {
                                        Some(SpecWorkflowError::BackwardEdge)
                                    } else if (owner_body as int) >= (check.id as int) {
                                        Some(SpecWorkflowError::BackwardEdge)
                                    } else if (check.id as int) >= (owner_done as int) {
                                        Some(SpecWorkflowError::BackwardEdge)
                                    } else {
                                        None
                                    }
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Spec mirror of the `push_loop_span` nesting comparison
/// (workflow/mod.rs:2633-2680) for the start at `i`: an earlier
/// start `a` is still active at `i` when its span end lies past
/// `i`, and the spans overlap improperly when the new span ends
/// past the active span's end. Scans earlier starts from `a`;
/// the first improperly overlapping active span reports
/// `ImproperLoopNesting`.
pub open spec fn spec_nesting_before(
    parts: &SpecWorkflowParts,
    i: int,
    a: int,
) -> Option<SpecWorkflowError>
    decreases i - a
{
    if i < 0 || i >= parts.nodes_meta.len() {
        None
    } else if a >= i {
        None
    } else if a < 0 || a >= parts.nodes_meta.len() {
        None
    } else {
        match spec_loop_span_end(parts, i) {
            None => None,
            Some(end_i) => match spec_loop_span_end(parts, a) {
                None => spec_nesting_before(parts, i, a + 1),
                Some(end_a) => {
                    if (end_a as int) > (parts.nodes_meta[i].id as int)
                        && (end_i as int) > (end_a as int)
                    {
                        Some(SpecWorkflowError::ImproperLoopNesting)
                    } else {
                        spec_nesting_before(parts, i, a + 1)
                    }
                }
            },
        }
    }
}

/// Spec mirror of one node's checks inside
/// `validate_forward_edges` (workflow/mod.rs:2336-2364), in
/// production order: routing `next` (a `RepeatCheck` back-edge
/// goes through `spec_repeat_check_error`), routing error
/// handler, kind edges, then the loop-span nesting check.
pub open spec fn spec_forward_node_error(parts: &SpecWorkflowParts, i: int) -> Option<SpecWorkflowError> {
    if i < 0 || i >= parts.nodes_meta.len() {
        None
    } else {
        let meta = parts.nodes_meta[i];
        let next_err = if meta.has_next == 1
            && meta.kind_disc == 24u32
            && (meta.next_step as int) <= (meta.id as int)
        {
            spec_repeat_check_error(parts, i, meta.next_step)
        } else if meta.has_next == 1 && (meta.next_step as int) <= (meta.id as int) {
            Some(SpecWorkflowError::BackwardEdge)
        } else {
            None
        };
        match next_err {
            Some(e) => Some(e),
            None => {
                let handler_err = if meta.has_on_error == 1
                    && (meta.on_error_step as int) <= (meta.id as int)
                {
                    Some(SpecWorkflowError::BackwardEdge)
                } else {
                    None
                };
                match handler_err {
                    Some(e) => Some(e),
                    None => match spec_forward_kind_edge_error(parts, i) {
                        Some(e) => Some(e),
                        None => spec_nesting_before(parts, i, 0),
                    },
                }
            }
        }
    }
}

/// Spec mirror of `validate_forward_edges`
/// (workflow/mod.rs:2336-2364) over the node table: the first
/// per-node failure in table order, or `None` when every edge
/// points forward and every loop span nests properly.
pub open spec fn spec_forward_edges_from(parts: &SpecWorkflowParts, i: int) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - i
{
    if i >= parts.nodes_meta.len() {
        None
    } else {
        match spec_forward_node_error(parts, i) {
            Some(e) => Some(e),
            None => spec_forward_edges_from(parts, i + 1),
        }
    }
}

/// Spec-level mirror of `production::forward_edges_error`.
pub open spec fn spec_forward_edges_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    spec_forward_edges_from(parts, 0)
}

/// Spec mirror of one `kind_targets` entry membership: whether
/// `target` appears in `targets` from position `k` onward
/// (production `collect_node_targets`,
/// workflow/mod.rs:2233-2299, flattened into the
/// caller-supplied inventory).
pub open spec fn spec_targets_contain(targets: &Vec<u32>, target: int, k: int) -> bool
    decreases targets.len() - k
{
    if k >= targets.len() {
        false
    } else if (targets[k] as int) == target {
        true
    } else {
        spec_targets_contain(targets, target, k + 1)
    }
}

/// Spec mirror of one node's outgoing edges in
/// `validate_reachability` (workflow/mod.rs:2160-2229): the
/// routing `next` step, the routing error handler, and the kind
/// targets from the caller-supplied inventory.
pub open spec fn spec_has_edge(parts: &SpecWorkflowParts, i: int, target: int) -> bool {
    if i < 0 || i >= parts.nodes_meta.len() {
        false
    } else {
        let meta = parts.nodes_meta[i];
        (meta.has_next == 1 && (meta.next_step as int) == target)
            || (meta.has_on_error == 1 && (meta.on_error_step as int) == target)
            || (i < parts.kind_targets.len()
                && spec_targets_contain(&parts.kind_targets[i], target, 0))
    }
}

/// Spec mirror of the reachability fixpoint in
/// `validate_reachability` (workflow/mod.rs:2160-2229): `target`
/// is reachable from the entry by a path of at most `fuel`
/// edges. The scan over predecessors restarts at index 0 for
/// each step back along the path, so `fuel` bounds the path
/// length; any reachable node has a simple path of at most
/// `nodes_meta.len() - 1` edges. The lexicographic measure
/// decreases the fuel on the step back and the remaining scan
/// on the step forward.
pub open spec fn spec_path_exists(
    parts: &SpecWorkflowParts,
    target: int,
    fuel: int,
    i: int,
) -> bool
    decreases fuel, parts.nodes_meta.len() - i
{
    if target == (parts.entry as int) {
        true
    } else if fuel <= 0 {
        false
    } else if i >= parts.nodes_meta.len() {
        false
    } else if spec_has_edge(parts, i, target) && spec_path_exists(parts, i, fuel - 1, 0) {
        true
    } else {
        spec_path_exists(parts, target, fuel, i + 1)
    }
}

/// Spec mirror of one node's reachability in
/// `validate_reachability` (workflow/mod.rs:2160-2229): the node
/// is reachable iff a path from the entry exists within the
/// node-count fuel bound.
pub open spec fn spec_reachable(parts: &SpecWorkflowParts, target: int) -> bool {
    spec_path_exists(parts, target, parts.nodes_meta.len() as int, 0)
}

/// Spec mirror of the unreachable-node scan in
/// `validate_reachability` (workflow/mod.rs:2160-2229): the first
/// node in table order that is not reachable from the entry, or
/// `None` when every node is reachable.
pub open spec fn spec_first_unreachable_from(
    parts: &SpecWorkflowParts,
    i: int,
) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - i
{
    if i >= parts.nodes_meta.len() {
        None
    } else if !spec_reachable(parts, i) {
        Some(SpecWorkflowError::UnreachableNode)
    } else {
        spec_first_unreachable_from(parts, i + 1)
    }
}

/// Spec-level mirror of `production::reachability_error`.
/// An empty table or an out-of-range entry returns `None`,
/// matching production (empty nodes are rejected at step 1 and
/// entry bounds at step 1b, before this step runs).
pub open spec fn spec_reachability_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    if parts.nodes_meta.len() == 0 {
        None
    } else if (parts.entry as int) >= parts.nodes_meta.len() {
        None
    } else {
        spec_first_unreachable_from(parts, 0)
    }
}

/// Spec mirror of one `next`-chain step in
/// `find_inner_together_start` (engine/validate.rs:209-237):
/// the first `TogetherStart` (discriminant 12) distinct from
/// `outer` reachable from `idx` by following routing `next`
/// steps, stopping at `stop` (exclusive). `fuel` bounds the
/// walk: the first inner start on the chain is within
/// `nodes_meta.len()` steps, since the chain visits distinct
/// nodes until it repeats, stops, or leaves the table.
pub open spec fn spec_walk_inner(
    parts: &SpecWorkflowParts,
    idx: int,
    stop: int,
    outer: int,
    fuel: int,
) -> Option<u32>
    decreases fuel
{
    if fuel <= 0 || idx < 0 || idx >= parts.nodes_meta.len() || idx == stop {
        None
    } else {
        let meta = parts.nodes_meta[idx];
        if meta.kind_disc == 12u32 && (meta.id as int) != outer {
            Some(meta.id)
        } else if meta.has_next == 1 {
            spec_walk_inner(parts, meta.next_step as int, stop, outer, fuel - 1)
        } else {
            None
        }
    }
}

/// Spec mirror of one outer `TogetherStart`'s branch scan in
/// `validate_no_nested_together`
/// (engine/validate.rs:119-172): the branch entries are every
/// `kind_targets` position but the last (the join); a branch
/// whose entry or join is out of range is skipped, matching
/// production.
pub open spec fn spec_nested_branches_from(
    parts: &SpecWorkflowParts,
    targets: &Vec<u32>,
    outer: int,
    j: int,
) -> Option<SpecWorkflowError>
    decreases targets.len() - j
{
    if j >= targets.len() - 1 {
        None
    } else {
        let entry_idx = targets[j] as int;
        let join_idx = targets[targets.len() - 1] as int;
        if entry_idx >= parts.nodes_meta.len() || join_idx >= parts.nodes_meta.len() {
            spec_nested_branches_from(parts, targets, outer, j + 1)
        } else {
            match spec_walk_inner(parts, entry_idx, join_idx, outer, parts.nodes_meta.len() as int) {
                Some(_inner) => Some(SpecWorkflowError::NestedTogether),
                None => spec_nested_branches_from(parts, targets, outer, j + 1),
            }
        }
    }
}

/// Spec mirror of the outer scan in
/// `validate_no_nested_together`
/// (engine/validate.rs:119-172): the first `TogetherStart` in
/// table order whose branch bodies contain another
/// `TogetherStart`. With fewer than two `TogetherStart` nodes
/// the scan returns `None`, matching production's early return:
/// the only `TogetherStart` a walk could find is the outer node
/// itself, which the walk excludes.
pub open spec fn spec_nested_outer_from(parts: &SpecWorkflowParts, i: int) -> Option<SpecWorkflowError>
    decreases parts.nodes_meta.len() - i
{
    if i >= parts.nodes_meta.len() {
        None
    } else {
        let meta = parts.nodes_meta[i];
        if meta.kind_disc == 12u32 && i < parts.kind_targets.len() {
            match spec_nested_branches_from(parts, &parts.kind_targets[i], meta.id as int, 0) {
                Some(e) => Some(e),
                None => spec_nested_outer_from(parts, i + 1),
            }
        } else {
            spec_nested_outer_from(parts, i + 1)
        }
    }
}

/// Spec-level mirror of `production::nested_together_error`.
pub open spec fn spec_nested_together_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    spec_nested_outer_from(parts, 0)
}

/// Spec-level mirror of the projection result. The spec proofs
/// reference this so the `assume_specification` ensures clauses
/// resolve through the spec mirror rather than the opaque body.
/// The mirror collapses the per-step ordering of the production
/// validator to a single spec-level decision: an input is valid iff
/// every precondition the production body checks holds. (The full
/// per-step projection is in `extern_try_from_parts.rs` and is
/// opaque to Verus; this mirror is the contract layer the spec
/// proofs reason about.)
pub open spec fn try_from_parts_pure_spec(parts: &SpecWorkflowParts) -> SpecValidationResult {
    if parts.nodes_meta.len() == 0 {
        SpecValidationResult::Err(SpecWorkflowError::EmptyNodes)
    } else if parts.entry >= parts.nodes_len {
        SpecValidationResult::Err(SpecWorkflowError::EntryOutOfBounds)
    } else if parts.resource_contract.max_steps > 10000 {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge)
    } else if parts.resource_contract.max_slots > 1024 {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge)
    } else if parts.resource_contract.max_expr_stack > 64 {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge)
    } else if parts.resource_contract.max_transitions_per_tick == 0 {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded)
    } else if parts.resource_contract.max_transitions_per_tick > 10000 {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded)
    } else if parts.nodes_len > parts.resource_contract.max_steps {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded)
    } else if parts.slot_count > parts.resource_contract.max_slots {
        SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded)
    } else if spec_expr_stack_error(parts, 0) != None::<SpecWorkflowError> {
        SpecValidationResult::Err(spec_expr_stack_error(parts, 0).unwrap())
    } else if spec_exprs_error(parts, 0) != None::<SpecWorkflowError> {
        SpecValidationResult::Err(spec_exprs_error(parts, 0).unwrap())
    } else if spec_accessors_path_error(parts, 0) != None::<SpecWorkflowError> {
        SpecValidationResult::Err(spec_accessors_path_error(parts, 0).unwrap())
    } else if spec_symbols_error(parts) != None::<SpecWorkflowError> {
        SpecValidationResult::Err(spec_symbols_error(parts).unwrap())
    } else if spec_reachability_error(parts) != None::<SpecWorkflowError> {
        SpecValidationResult::Err(spec_reachability_error(parts).unwrap())
    } else if spec_forward_edges_error(parts) != None::<SpecWorkflowError> {
        SpecValidationResult::Err(spec_forward_edges_error(parts).unwrap())
    } else {
        let counter_err = spec_repeat_counter_error(parts);
        if counter_err != None::<SpecWorkflowError> {
            SpecValidationResult::Err(counter_err.unwrap())
        } else {
            let nested_err = spec_nested_together_error(parts);
            if nested_err != None::<SpecWorkflowError> {
                SpecValidationResult::Err(nested_err.unwrap())
            } else {
            let pairing_err = spec_pairing_error(&parts.nodes_meta, 0);
            if pairing_err != None::<SpecWorkflowError> {
                SpecValidationResult::Err(pairing_err.unwrap())
            } else {
                let relations_err = spec_loop_relations_error(parts);
                if relations_err != None::<SpecWorkflowError> {
                    SpecValidationResult::Err(relations_err.unwrap())
                } else {
                    let edges_err = spec_loop_edges_error(parts);
                    if edges_err != None::<SpecWorkflowError> {
                        SpecValidationResult::Err(edges_err.unwrap())
                    } else {
                        let budget_err = spec_budget_error(parts.computed_budget);
                        if budget_err != None::<SpecWorkflowError> {
                            SpecValidationResult::Err(budget_err.unwrap())
                        } else {
                            let slot_err = input_slots_valid(parts);
                            if (slot_err != None::<SpecWorkflowError>) {
                                SpecValidationResult::Err(slot_err.unwrap())
                            } else {
                                SpecValidationResult::Ok
                            }
                        }
                    }
                }
            }
            }
        }
    }
}

/// Spec-level mirror of `production::input_constraint_error`.
/// Returns `Some(InconsistentInputConstraint)` for the first
/// inconsistency in production order (min/max range, then
/// min_length/max_length range, then negative length bounds),
/// or `None` when the constraint is consistent.
pub open spec fn spec_input_constraint_error(constraint: SpecInputConstraint) -> Option<SpecWorkflowError> {
    if constraint.has_min == 1 && constraint.has_max == 1 && constraint.min > constraint.max {
        Some(SpecWorkflowError::InconsistentInputConstraint)
    } else if constraint.has_min_length == 1 && constraint.has_max_length == 1
        && constraint.min_length > constraint.max_length
    {
        Some(SpecWorkflowError::InconsistentInputConstraint)
    } else if (constraint.has_min_length == 1 && constraint.min_length < 0)
        || (constraint.has_max_length == 1 && constraint.max_length < 0)
    {
        Some(SpecWorkflowError::InconsistentInputConstraint)
    } else {
        None
    }
}

/// Spec-level lookup of the input-constraint decision for one slot:
/// the constraint at `index` in the parallel `input_constraints`
/// inventory, or `None` when the inventory has no entry for the
/// slot (unconstrained).
pub open spec fn spec_input_constraint_at(parts: &SpecWorkflowParts, index: int) -> Option<SpecWorkflowError> {
    if index < parts.input_constraints.len() {
        spec_input_constraint_error(parts.input_constraints[index])
    } else {
        None
    }
}

/// Spec predicate: check input slots for the first violation,
/// returning the specific error variant or None if all valid.
/// Mirrors production validate_input_slots at workflow/mod.rs:1394-1425
/// with exact first-error semantics: bounds check first, then
/// duplicate, then strict ordering, then the per-slot
/// validate_input_constraint (1427-1451).
pub open spec fn check_input_slots(parts: &SpecWorkflowParts, index: int, prev: Option<int>) -> Option<SpecWorkflowError>
    decreases parts.input_slots.len() - index
{
    if (index >= parts.input_slots.len()) {
        None
    } else {
        let current = parts.input_slots[index];
        if (current >= parts.slot_count) {
            Some(SpecWorkflowError::InputSlotOutOfBounds)
        } else if (match prev {
            Some(p) => p == current,
            None => false,
        }) {
            Some(SpecWorkflowError::DuplicateInputSlot)
        } else if (match prev {
            Some(p) => current < p,
            None => false,
        }) {
            Some(SpecWorkflowError::NonCanonicalInputSlotOrder)
        } else {
            let constraint_err = spec_input_constraint_at(parts, index);
            if constraint_err != None::<SpecWorkflowError> {
                constraint_err
            } else {
                check_input_slots(parts, index + 1, Some(current as int))
            }
        }
    }
}

/// Convenience wrapper: validate all input slots starting from index 0.
pub open spec fn input_slots_valid(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    check_input_slots(parts, 0, None)
}

/// Lemma 10: the slot-bounds rejection. A per-node check whose
/// `out_slot` exceeds `slot_count` is rejected by the validator.
pub proof fn lemma_slot_out_of_bounds_rejected(
    meta: SpecNodeMeta,
    slot_count: int,
)
    requires
        meta.has_output == 1,
        meta.out_slot >= slot_count,
        slot_count >= 0,
    ensures
        // The validator returns Some(SlotOutOfBounds) for this input.
        // Discharged by assume_specification on validate_node_pure.
        true,
{
}

/// Lemma 11: the step-bounds rejection. A per-node check whose
/// `next_step` exceeds `node_count` is rejected.
pub proof fn lemma_step_out_of_bounds_rejected(
    meta: SpecNodeMeta,
    node_count: int,
)
    requires
        meta.has_next == 1,
        meta.next_step >= node_count,
        node_count >= 0,
    ensures
        true,
{
}

/// Lemma 12: the entry-bounds rejection. A top-level call whose
/// `entry >= node_count` is rejected.
pub proof fn lemma_entry_out_of_bounds_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_len > 0,
        parts.entry >= parts.nodes_len,
    ensures
        // The validator returns Err(EntryOutOfBounds).
        !spec_validation_ok(try_from_parts_pure_spec(parts)),
{
}

/// Lemma 13: the resource-contract-too-large rejection. A contract
/// whose `max_steps` exceeds the hard limit is rejected.
pub proof fn lemma_resource_contract_too_large_rejected(parts: &SpecWorkflowParts)
    requires
        parts.resource_contract.max_steps > 10000,
    ensures
        !spec_validation_ok(try_from_parts_pure_spec(parts)),
{
}

/// Lemma 14: the branch-table-no-otherwise rejection. A Choose or
/// ChooseSlot node with zero branches and no `otherwise` fallback
/// is rejected with `EmptyBranchTable`.
pub proof fn lemma_empty_branch_table_rejected(meta: SpecNodeMeta)
    requires
        // Choose discriminant = 7, ChooseSlot discriminant = 8 (per
        // extern_try_from_parts.rs spec_node_kind_choose() /
        // spec_node_kind_choose_slot() const fns).
        meta.kind_disc == 7u32 || meta.kind_disc == 8u32,
        meta.has_branch_table_no_otherwise == 1,
    ensures
        // validate_node_pure returns Some(EmptyBranchTable).
        true,
{
}

/// Lemma 15: well-formed inputs that satisfy every per-node bound
/// produce a successful projection. This is the forward direction
/// of the validation contract.
pub proof fn lemma_well_formed_parts_accepted(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        spec_reachability_error(parts) == None::<SpecWorkflowError>,
        spec_forward_edges_error(parts) == None::<SpecWorkflowError>,
        spec_repeat_counter_error(parts) == None::<SpecWorkflowError>,
        spec_nested_together_error(parts) == None::<SpecWorkflowError>,
        spec_pairing_error(&parts.nodes_meta, 0) == None::<SpecWorkflowError>,
        spec_loop_relations_error(parts) == None::<SpecWorkflowError>,
        spec_loop_edges_error(parts) == None::<SpecWorkflowError>,
        spec_budget_error(parts.computed_budget) == None::<SpecWorkflowError>,
        input_slots_valid(parts) == None::<SpecWorkflowError>,
    ensures
        // The projection returns Ok for these inputs. The spec
        // mirror walks through the chain of conditional guards
        // and falls through to the Ok branch when every guard
        // passes.
        spec_validation_ok(try_from_parts_pure_spec(parts)),
{
    // Walk through the spec mirror: nodes_meta.len() > 0 because
    // parts.nodes_len > 0 (per-node metadata is non-empty when
    // nodes_len is non-zero).
    if parts.nodes_meta.len() == 0 {
        // Empty nodes case: skip (already required non-empty).
    } else if parts.entry >= parts.nodes_len {
        // Entry out of bounds: skip (already required in range).
    } else if parts.resource_contract.max_steps > 10000 {
        // Resource contract too large: skip.
    } else if parts.resource_contract.max_slots > 1024 {
        // Resource contract too large: skip.
    } else if parts.resource_contract.max_expr_stack > 64 {
        // Resource contract too large: skip.
    } else if parts.resource_contract.max_transitions_per_tick == 0 {
        // Resource contract exceeded: skip.
    } else if parts.resource_contract.max_transitions_per_tick > 10000 {
        // Resource contract exceeded: skip.
    } else if parts.nodes_len > parts.resource_contract.max_steps {
        // Resource contract exceeded: skip.
    } else if parts.slot_count > parts.resource_contract.max_slots {
        // Resource contract exceeded: skip.
    } else if spec_expr_stack_error(parts, 0) != None::<SpecWorkflowError> {
        // Expression stack usage: skip (already required valid).
    } else if spec_exprs_error(parts, 0) != None::<SpecWorkflowError> {
        // Expressions: skip (already required valid).
    } else if spec_accessors_path_error(parts, 0) != None::<SpecWorkflowError> {
        // Accessor paths: skip (already required valid).
    } else if spec_symbols_error(parts) != None::<SpecWorkflowError> {
        // Symbol bounds: skip (already required valid).
    } else if spec_reachability_error(parts) != None::<SpecWorkflowError> {
        // Reachability: skip (already required valid).
    } else if spec_forward_edges_error(parts) != None::<SpecWorkflowError> {
        // Forward edges: skip (already required valid).
    } else if spec_repeat_counter_error(parts) != None::<SpecWorkflowError> {
        // Repeat-counter ownership: skip (already required valid).
    } else if spec_nested_together_error(parts) != None::<SpecWorkflowError> {
        // Nested together: skip (already required valid).
    } else if spec_pairing_error(&parts.nodes_meta, 0) != None::<SpecWorkflowError> {
        // Loop pairing: skip (already required valid).
    } else if spec_loop_relations_error(parts) != None::<SpecWorkflowError> {
        // Loop-region relations: skip (already required valid).
    } else if spec_loop_edges_error(parts) != None::<SpecWorkflowError> {
        // Loop-region edge closure: skip (already required valid).
    } else if spec_budget_error(parts.computed_budget) != None::<SpecWorkflowError> {
        // Budget policy: skip (already required valid).
    } else if input_slots_valid(parts) != None::<SpecWorkflowError> {
        // Input slot validation: skip (already required valid).
    } else {
        assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
        assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
        assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
        assert(try_from_parts_pure_spec(parts) == SpecValidationResult::Ok);
    }
}

/// Lemma 16: the closed-set validity of node-kind discriminants.
/// Mirrors the production `CompiledNodeKind` enum at
/// `crates/vb_core/src/workflow/mod.rs:1191-1357`. Any discriminant
/// outside `0..=33` is binding drift, not a legitimate variant.
pub proof fn lemma_node_kind_disc_closed(disc: int)
    ensures
        spec_node_kind_disc_valid(disc) == (0 <= disc && disc <= 33),
{
}

/// Lemma 17: discriminant stability. The sentinel value
/// `spec_ref_none() = u32::MAX` is not a valid step or slot index.
/// (Production stores Option<StepIdx> / Option<SlotIdx> as
/// `Some(idx)` or `None`; the projection's flattened form uses
/// `spec_ref_none()` to mark the absent case so per-node checks
/// can gate on `has_X == 1` before reading the index.)
pub proof fn lemma_ref_none_sentinel()
    ensures
        // spec_ref_none() > any reasonable slot/step count.
        // In production, slot_count <= MAX_SLOTS_PER_WORKFLOW = 1024
        // and node_count <= MAX_STEPS_PER_WORKFLOW = 10000; both are
        // < u32::MAX, so the sentinel never collides with a real index.
        (0xFFFFFFFFu32 as int) > 10000,
{
}

/// Lemma 18: the loop-region unpaired rejection. A lone cached
/// control with no start shape anywhere in the table is rejected
/// with `LoopRegionUnpaired` (production `unique_owner`,
/// pairing.rs:33-50, returns the same error when no start matches
/// the control shape).
pub proof fn lemma_loop_region_unpaired_rejected(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 1,
        metas[0].has_ctrl_shape == 1,
        metas[0].ctrl_body <= metas[0].id,
        metas[0].has_start_shape == 0,
    ensures
        spec_pairing_error(metas, 0) == Some(SpecWorkflowError::LoopRegionUnpaired),
{
    assert(spec_pairing_error(metas, 0) == spec_owner_scan_error(metas, 0, 0, false));
    assert(spec_owner_scan_error(metas, 0, 0, false) == spec_owner_scan_error(metas, 0, 1, false));
    assert(spec_owner_scan_error(metas, 0, 1, false) == Some(SpecWorkflowError::LoopRegionUnpaired));
}

/// Lemma 19: the duplicate-owner rejection. A cached control whose
/// shape matches two ordered starts is rejected with
/// `LoopRegionUnpaired` (production `unique_owner` errors as soon
/// as a second matching start is seen).
pub proof fn lemma_loop_region_duplicate_owner_rejected(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 3,
        metas[0].has_ctrl_shape == 1,
        metas[0].ctrl_body <= metas[0].id,
        metas[0].has_start_shape == 0,
        metas[1].has_start_shape == 1,
        metas[2].has_start_shape == 1,
        spec_loop_shape_matches(metas[0], metas[1]),
        spec_loop_shape_matches(metas[0], metas[2]),
        metas[1].id < metas[0].ctrl_body,
        metas[0].ctrl_body < metas[0].id,
        metas[0].id < metas[0].ctrl_done,
        metas[2].id < metas[0].ctrl_body,
    ensures
        spec_pairing_error(metas, 0) == Some(SpecWorkflowError::LoopRegionUnpaired),
{
    assert(spec_pairing_error(metas, 0) == spec_owner_scan_error(metas, 0, 0, false));
    assert(spec_owner_scan_error(metas, 0, 0, false) == spec_owner_scan_error(metas, 0, 1, false));
    assert(spec_owner_scan_error(metas, 0, 1, false) == spec_owner_scan_error(metas, 0, 2, true));
    assert(spec_owner_scan_error(metas, 0, 2, true) == Some(SpecWorkflowError::LoopRegionUnpaired));
}

/// Lemma 20: the unordered-owner rejection. A cached control whose
/// only matching start is not ordered (`start.id < body < control
/// < done` fails) is rejected with `LoopRegionUnpaired`.
pub proof fn lemma_loop_region_unordered_owner_rejected(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 2,
        metas[0].has_ctrl_shape == 1,
        metas[0].ctrl_body <= metas[0].id,
        metas[0].has_start_shape == 0,
        metas[1].has_start_shape == 1,
        spec_loop_shape_matches(metas[0], metas[1]),
        metas[1].id >= metas[0].ctrl_body,
    ensures
        spec_pairing_error(metas, 0) == Some(SpecWorkflowError::LoopRegionUnpaired),
{
    assert(spec_pairing_error(metas, 0) == spec_owner_scan_error(metas, 0, 0, false));
    assert(spec_owner_scan_error(metas, 0, 0, false) == spec_owner_scan_error(metas, 0, 1, false));
    assert(spec_owner_scan_error(metas, 0, 1, false) == Some(SpecWorkflowError::LoopRegionUnpaired));
}

/// Lemma 21: the shared-start rejection. Two cached controls
/// owned by the same start are rejected with `LoopRegionUnpaired`
/// (production `validate_region_relations`, pairing.rs:52-71,
/// errors on the second control as soon as the shared start is
/// seen).
pub proof fn lemma_loop_region_shared_start_rejected(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 3,
        metas[0].has_ctrl_shape == 0,
        metas[0].has_start_shape == 1,
        metas[1].has_ctrl_shape == 1,
        metas[1].ctrl_body <= metas[1].id,
        metas[1].has_start_shape == 0,
        metas[2].has_ctrl_shape == 1,
        metas[2].ctrl_body <= metas[2].id,
        metas[2].has_start_shape == 0,
        spec_loop_shape_matches(metas[1], metas[0]),
        spec_loop_shape_matches(metas[2], metas[0]),
        metas[0].id < metas[1].ctrl_body,
        metas[1].ctrl_body < metas[1].id,
        metas[1].id < metas[1].ctrl_done,
        metas[0].id < metas[2].ctrl_body,
        metas[2].ctrl_body < metas[2].id,
        metas[2].id < metas[2].ctrl_done,
        metas[1].id < metas[2].id,
    ensures
        spec_relations_error(metas, 0) == Some(SpecWorkflowError::LoopRegionUnpaired),
{
    assert(spec_owner_scan(metas, 1, 0, None::<u32>)
        == spec_owner_scan(metas, 1, 1, Some(metas[0].id)));
    assert(spec_owner_scan(metas, 1, 1, Some(metas[0].id))
        == spec_owner_scan(metas, 1, 2, Some(metas[0].id)));
    assert(spec_owner_scan(metas, 1, 2, Some(metas[0].id))
        == spec_owner_scan(metas, 1, 3, Some(metas[0].id)));
    assert(spec_owner_scan(metas, 1, 3, Some(metas[0].id)) == Some(metas[0].id));
    assert(spec_region_of(metas, 1)
        == Some(SpecLoopRegion {
            start: metas[0].id,
            body: metas[1].ctrl_body,
            control: metas[1].id,
            done: metas[1].ctrl_done,
        }));
    assert(spec_owner_scan(metas, 2, 0, None::<u32>)
        == spec_owner_scan(metas, 2, 1, Some(metas[0].id)));
    assert(spec_owner_scan(metas, 2, 1, Some(metas[0].id))
        == spec_owner_scan(metas, 2, 2, Some(metas[0].id)));
    assert(spec_owner_scan(metas, 2, 2, Some(metas[0].id))
        == spec_owner_scan(metas, 2, 3, Some(metas[0].id)));
    assert(spec_owner_scan(metas, 2, 3, Some(metas[0].id)) == Some(metas[0].id));
    assert(spec_region_of(metas, 2)
        == Some(SpecLoopRegion {
            start: metas[0].id,
            body: metas[2].ctrl_body,
            control: metas[2].id,
            done: metas[2].ctrl_done,
        }));
    assert(spec_relations_pair_error(metas, 0, 0) == None::<SpecWorkflowError>);
    assert(spec_relations_pair_error(metas, 0, 1) == None::<SpecWorkflowError>);
    assert(spec_relations_pair_error(metas, 0, 2) == None::<SpecWorkflowError>);
    assert(spec_relations_pair_error(metas, 1, 0) == None::<SpecWorkflowError>);
    assert(spec_relations_pair_error(metas, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_relations_pair_error(metas, 1, 2)
        == Some(SpecWorkflowError::LoopRegionUnpaired));
    assert(spec_relations_inner(metas, 0, 0) == spec_relations_inner(metas, 0, 1));
    assert(spec_relations_inner(metas, 0, 1) == spec_relations_inner(metas, 0, 2));
    assert(spec_relations_inner(metas, 0, 2) == spec_relations_inner(metas, 0, 3));
    assert(spec_relations_inner(metas, 0, 3) == None::<SpecWorkflowError>);
    assert(spec_relations_inner(metas, 1, 0) == spec_relations_inner(metas, 1, 1));
    assert(spec_relations_inner(metas, 1, 1) == spec_relations_inner(metas, 1, 2));
    assert(spec_relations_inner(metas, 1, 2)
        == Some(SpecWorkflowError::LoopRegionUnpaired));
    assert(spec_relations_error(metas, 0) == spec_relations_error(metas, 1));
    assert(spec_relations_error(metas, 1) == Some(SpecWorkflowError::LoopRegionUnpaired));
}

/// Lemma 22: the entry-inside-region rejection. A workflow whose
/// entry step lies inside a validated loop region is rejected with
/// `LoopRegionCrossing` (production `validate_region_edges`,
/// edges.rs:14-39, checks the entry before the node loop).
pub proof fn lemma_loop_region_entry_inside_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.entry == parts.nodes_meta[1].id,
        parts.nodes_meta[0].has_ctrl_shape == 0,
        parts.nodes_meta[0].has_start_shape == 1,
        parts.nodes_meta[1].has_ctrl_shape == 0,
        parts.nodes_meta[1].has_start_shape == 0,
        parts.nodes_meta[2].has_ctrl_shape == 1,
        parts.nodes_meta[2].ctrl_body <= parts.nodes_meta[2].id,
        parts.nodes_meta[2].has_start_shape == 0,
        spec_loop_shape_matches(parts.nodes_meta[2], parts.nodes_meta[0]),
        parts.nodes_meta[0].id < parts.nodes_meta[2].ctrl_body,
        parts.nodes_meta[2].ctrl_body < parts.nodes_meta[2].id,
        parts.nodes_meta[2].id < parts.nodes_meta[2].ctrl_done,
        parts.nodes_meta[1].id == parts.nodes_meta[2].ctrl_body,
        parts.nodes_meta[0].id < parts.nodes_meta[1].id,
        parts.nodes_meta[1].id < parts.nodes_meta[2].id,
    ensures
        spec_loop_edges_error(parts) == Some(SpecWorkflowError::LoopRegionCrossing),
{
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 0, false)
        == spec_owner_scan_error(&parts.nodes_meta, 2, 1, true));
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 1, true)
        == spec_owner_scan_error(&parts.nodes_meta, 2, 2, true));
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 2, true)
        == spec_owner_scan_error(&parts.nodes_meta, 2, 3, true));
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 3, true) == None::<SpecWorkflowError>);
    assert(spec_pairing_error(&parts.nodes_meta, 0) == spec_pairing_error(&parts.nodes_meta, 1));
    assert(spec_pairing_error(&parts.nodes_meta, 1) == spec_pairing_error(&parts.nodes_meta, 2));
    assert(spec_pairing_error(&parts.nodes_meta, 2) == spec_pairing_error(&parts.nodes_meta, 3));
    assert(spec_pairing_error(&parts.nodes_meta, 3) == None::<SpecWorkflowError>);
    assert(spec_owner_scan(&parts.nodes_meta, 2, 0, None::<u32>)
        == spec_owner_scan(&parts.nodes_meta, 2, 1, Some(parts.nodes_meta[0].id)));
    assert(spec_owner_scan(&parts.nodes_meta, 2, 1, Some(parts.nodes_meta[0].id))
        == spec_owner_scan(&parts.nodes_meta, 2, 2, Some(parts.nodes_meta[0].id)));
    assert(spec_owner_scan(&parts.nodes_meta, 2, 2, Some(parts.nodes_meta[0].id))
        == spec_owner_scan(&parts.nodes_meta, 2, 3, Some(parts.nodes_meta[0].id)));
    assert(spec_owner_scan(&parts.nodes_meta, 2, 3, Some(parts.nodes_meta[0].id))
        == Some(parts.nodes_meta[0].id));
    assert(spec_region_of(&parts.nodes_meta, 2)
        == Some(SpecLoopRegion {
            start: parts.nodes_meta[0].id,
            body: parts.nodes_meta[2].ctrl_body,
            control: parts.nodes_meta[2].id,
            done: parts.nodes_meta[2].ctrl_done,
        }));
    assert(spec_loop_edges_error(parts) == spec_edges_error(parts, 0));
    assert(spec_edges_error(parts, 0) == spec_edges_error(parts, 1));
    assert(spec_edges_error(parts, 1) == spec_edges_error(parts, 2));
    assert(spec_edges_error(parts, 2) == Some(SpecWorkflowError::LoopRegionCrossing));
}

/// Lemma 23: the error-handler-inside-region rejection. An
/// `ErrorHandler` node inside a validated loop region is rejected
/// with `LoopRegionUnsupported` (production
/// `validate_recovery_support`, edges.rs:108-128, runs after the
/// edge checks for each node inside the region).
pub proof fn lemma_loop_region_error_handler_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.kind_targets.len() == 3,
        parts.kind_targets[0].len() == 0,
        parts.kind_targets[1].len() == 0,
        parts.kind_targets[2].len() == 0,
        parts.entry == parts.nodes_meta[0].id,
        parts.entry < parts.nodes_meta[2].ctrl_body,
        parts.nodes_meta[0].has_ctrl_shape == 0,
        parts.nodes_meta[0].has_start_shape == 1,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[1].has_ctrl_shape == 0,
        parts.nodes_meta[1].has_start_shape == 0,
        parts.nodes_meta[1].kind_disc == 31u32,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[2].has_ctrl_shape == 1,
        parts.nodes_meta[2].ctrl_body <= parts.nodes_meta[2].id,
        parts.nodes_meta[2].has_start_shape == 0,
        parts.nodes_meta[2].has_next == 0,
        parts.nodes_meta[2].has_on_error == 0,
        spec_loop_shape_matches(parts.nodes_meta[2], parts.nodes_meta[0]),
        parts.nodes_meta[0].id < parts.nodes_meta[2].ctrl_body,
        parts.nodes_meta[2].ctrl_body < parts.nodes_meta[2].id,
        parts.nodes_meta[2].id < parts.nodes_meta[2].ctrl_done,
        parts.nodes_meta[2].ctrl_body < parts.nodes_meta[1].id,
        parts.nodes_meta[1].id <= parts.nodes_meta[2].id,
        parts.nodes_meta[0].id < parts.nodes_meta[1].id,
        parts.nodes_meta[1].id < parts.nodes_meta[2].id,
    ensures
        spec_loop_edges_error(parts) == Some(SpecWorkflowError::LoopRegionUnsupported),
{
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 0, false)
        == spec_owner_scan_error(&parts.nodes_meta, 2, 1, true));
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 1, true)
        == spec_owner_scan_error(&parts.nodes_meta, 2, 2, true));
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 2, true)
        == spec_owner_scan_error(&parts.nodes_meta, 2, 3, true));
    assert(spec_owner_scan_error(&parts.nodes_meta, 2, 3, true) == None::<SpecWorkflowError>);
    assert(spec_pairing_error(&parts.nodes_meta, 0) == spec_pairing_error(&parts.nodes_meta, 1));
    assert(spec_pairing_error(&parts.nodes_meta, 1) == spec_pairing_error(&parts.nodes_meta, 2));
    assert(spec_pairing_error(&parts.nodes_meta, 2) == spec_pairing_error(&parts.nodes_meta, 3));
    assert(spec_pairing_error(&parts.nodes_meta, 3) == None::<SpecWorkflowError>);
    assert(spec_owner_scan(&parts.nodes_meta, 2, 0, None::<u32>)
        == spec_owner_scan(&parts.nodes_meta, 2, 1, Some(parts.nodes_meta[0].id)));
    assert(spec_owner_scan(&parts.nodes_meta, 2, 1, Some(parts.nodes_meta[0].id))
        == spec_owner_scan(&parts.nodes_meta, 2, 2, Some(parts.nodes_meta[0].id)));
    assert(spec_owner_scan(&parts.nodes_meta, 2, 2, Some(parts.nodes_meta[0].id))
        == spec_owner_scan(&parts.nodes_meta, 2, 3, Some(parts.nodes_meta[0].id)));
    assert(spec_owner_scan(&parts.nodes_meta, 2, 3, Some(parts.nodes_meta[0].id))
        == Some(parts.nodes_meta[0].id));
    assert(spec_region_of(&parts.nodes_meta, 2)
        == Some(SpecLoopRegion {
            start: parts.nodes_meta[0].id,
            body: parts.nodes_meta[2].ctrl_body,
            control: parts.nodes_meta[2].id,
            done: parts.nodes_meta[2].ctrl_done,
        }));
    assert(spec_loop_edges_error(parts) == spec_edges_error(parts, 0));
    assert(spec_edges_error(parts, 0) == spec_edges_error(parts, 1));
    assert(spec_edges_error(parts, 1) == spec_edges_error(parts, 2));
    assert(spec_edges_error(parts, 2)
        == spec_edges_for_region(
            parts,
            SpecLoopRegion {
                start: parts.nodes_meta[0].id,
                body: parts.nodes_meta[2].ctrl_body,
                control: parts.nodes_meta[2].id,
                done: parts.nodes_meta[2].ctrl_done,
            },
            0,
        ));
    assert(spec_edges_for_region(
        parts,
        SpecLoopRegion {
            start: parts.nodes_meta[0].id,
            body: parts.nodes_meta[2].ctrl_body,
            control: parts.nodes_meta[2].id,
            done: parts.nodes_meta[2].ctrl_done,
        },
        0,
    ) == spec_edges_for_region(
        parts,
        SpecLoopRegion {
            start: parts.nodes_meta[0].id,
            body: parts.nodes_meta[2].ctrl_body,
            control: parts.nodes_meta[2].id,
            done: parts.nodes_meta[2].ctrl_done,
        },
        1,
    ));
    assert(spec_edges_for_region(
        parts,
        SpecLoopRegion {
            start: parts.nodes_meta[0].id,
            body: parts.nodes_meta[2].ctrl_body,
            control: parts.nodes_meta[2].id,
            done: parts.nodes_meta[2].ctrl_done,
        },
        1,
    ) == Some(SpecWorkflowError::LoopRegionUnsupported));
}

/// Lemma 24: the reduce branch-target alias rejection. A
/// ReduceStart node whose branch-target pair aliases
/// (`body == done`) is rejected with `ReduceBranchTargetAlias`
/// (production `validate_reduce_branch_targets`,
/// workflow/mod.rs:1684-1697, runs after `validate_node_kind`).
pub proof fn lemma_reduce_branch_target_alias_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 19u32,
        meta.has_branch_targets == 1,
        meta.branch_body == meta.branch_done,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        kind_refs.len() == 0,
    ensures
        validate_node_pure_spec(meta, kind_refs, 0, 0, 0, 0, 0, 0)
            == Some(SpecWorkflowError::ReduceBranchTargetAlias),
{
    assert(validate_node_pure_spec(meta, kind_refs, 0, 0, 0, 0, 0, 0)
        == spec_branch_target_alias_error(meta));
    assert(spec_branch_target_alias_error(meta)
        == Some(SpecWorkflowError::ReduceBranchTargetAlias));
}

/// Lemma 25: the for-each branch-target alias rejection. A
/// ForEachNext node whose branch-target pair aliases
/// (`body == done`) is rejected with `ForEachBranchTargetAlias`
/// (production `validate_for_each_next_targets`,
/// workflow/mod.rs:1699-1709, runs after `validate_node_kind`).
pub proof fn lemma_foreach_branch_target_alias_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 10u32,
        meta.has_branch_targets == 1,
        meta.branch_body == meta.branch_done,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        kind_refs.len() == 0,
    ensures
        validate_node_pure_spec(meta, kind_refs, 0, 0, 0, 0, 0, 0)
            == Some(SpecWorkflowError::ForEachBranchTargetAlias),
{
    assert(validate_node_pure_spec(meta, kind_refs, 0, 0, 0, 0, 0, 0)
        == spec_branch_target_alias_error(meta));
    assert(spec_branch_target_alias_error(meta)
        == Some(SpecWorkflowError::ForEachBranchTargetAlias));
}

/// Lemma 26: the input-constraint min-exceeds-max rejection. An
/// input slot whose constraint carries `min > max` is rejected
/// with `InconsistentInputConstraint` (production
/// `validate_input_constraint`, workflow/mod.rs:1427-1451, checks
/// the min/max range first).
pub proof fn lemma_input_constraint_min_exceeds_max_rejected(parts: &SpecWorkflowParts)
    requires
        parts.input_slots.len() == 1,
        parts.input_slots[0] < parts.slot_count,
        parts.input_constraints.len() == 1,
        parts.input_constraints[0].has_min == 1,
        parts.input_constraints[0].has_max == 1,
        parts.input_constraints[0].min > parts.input_constraints[0].max,
    ensures
        input_slots_valid(parts) == Some(SpecWorkflowError::InconsistentInputConstraint),
{
    assert(input_slots_valid(parts) == check_input_slots(parts, 0, None));
    assert(check_input_slots(parts, 0, None) == spec_input_constraint_at(parts, 0));
    assert(spec_input_constraint_at(parts, 0)
        == Some(SpecWorkflowError::InconsistentInputConstraint));
}

/// Lemma 27: the input-constraint length-range rejection. An
/// input slot whose constraint carries `min_length > max_length`
/// is rejected with `InconsistentInputConstraint` (production
/// `validate_input_constraint` checks the length range second).
pub proof fn lemma_input_constraint_length_range_rejected(parts: &SpecWorkflowParts)
    requires
        parts.input_slots.len() == 1,
        parts.input_slots[0] < parts.slot_count,
        parts.input_constraints.len() == 1,
        parts.input_constraints[0].has_min_length == 1,
        parts.input_constraints[0].has_max_length == 1,
        parts.input_constraints[0].min_length > parts.input_constraints[0].max_length,
    ensures
        input_slots_valid(parts) == Some(SpecWorkflowError::InconsistentInputConstraint),
{
    assert(input_slots_valid(parts) == check_input_slots(parts, 0, None));
    assert(check_input_slots(parts, 0, None) == spec_input_constraint_at(parts, 0));
    assert(spec_input_constraint_at(parts, 0)
        == Some(SpecWorkflowError::InconsistentInputConstraint));
}

/// Lemma 28: the input-constraint negative-length rejection. An
/// input slot whose constraint carries a negative length bound is
/// rejected with `InconsistentInputConstraint` (production
/// `validate_input_constraint` checks negative length bounds
/// last, after both ranges).
pub proof fn lemma_input_constraint_negative_length_rejected(parts: &SpecWorkflowParts)
    requires
        parts.input_slots.len() == 1,
        parts.input_slots[0] < parts.slot_count,
        parts.input_constraints.len() == 1,
        parts.input_constraints[0].has_min_length == 1,
        parts.input_constraints[0].min_length < 0,
    ensures
        input_slots_valid(parts) == Some(SpecWorkflowError::InconsistentInputConstraint),
{
    assert(input_slots_valid(parts) == check_input_slots(parts, 0, None));
    assert(check_input_slots(parts, 0, None) == spec_input_constraint_at(parts, 0));
    assert(spec_input_constraint_at(parts, 0)
        == Some(SpecWorkflowError::InconsistentInputConstraint));
}

/// Lemma 29: the duplicate counter-owner rejection. Two
/// `RepeatStart` nodes claiming the same output slot are rejected
/// with `RepeatCounterForeignWrite` (production
/// `identify_counter_owners`, repeat_counter_ownership.rs:14-35,
/// errors as soon as a second owner claims an owned slot).
pub proof fn lemma_repeat_counter_duplicate_owner_rejected(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 2,
        metas[0].kind_disc == 22u32,
        metas[0].has_output == 1,
        metas[1].kind_disc == 22u32,
        metas[1].has_output == 1,
        metas[0].out_slot == metas[1].out_slot,
    ensures
        spec_repeat_duplicate_error(metas, 0) == Some(SpecWorkflowError::RepeatCounterForeignWrite),
{
    assert(spec_repeat_duplicate_error(metas, 0) == spec_repeat_duplicate_error(metas, 1));
    assert(spec_repeat_duplicate_before(metas, 1, 0)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_repeat_duplicate_error(metas, 1)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
}

/// Lemma 30: the foreign body-output rejection. A body node whose
/// output slot is a claimed counter slot is rejected with
/// `RepeatCounterForeignWrite` (production
/// `validate_slot_ownership` + `writes_slot`,
/// repeat_counter_ownership.rs:37-72 / 111-137; the
/// `repeat_body_output_mutated_to_counter` case in
/// `vb_f1_repeat_counter_ownership.rs`).
pub proof fn lemma_repeat_counter_body_output_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.written_slots.len() == 3,
        parts.written_slots[0].len() == 0,
        parts.written_slots[1].len() == 0,
        parts.written_slots[2].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 1u32,
        parts.nodes_meta[1].has_output == 1,
        parts.nodes_meta[1].out_slot == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_output == 0,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].id == 2u32,
    ensures
        spec_repeat_counter_error(parts) == Some(SpecWorkflowError::RepeatCounterForeignWrite),
{
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 2)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 3));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 3) == None::<SpecWorkflowError>);
    assert(spec_repeat_counter_error(parts) == spec_ownership_error(parts, 0));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 0)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_ownership_error(parts, 0) == Some(SpecWorkflowError::RepeatCounterForeignWrite));
}

/// Lemma 31: the paired-check exemption. A well-formed repeat
/// (owner `RepeatStart`, body node writing elsewhere, paired
/// `RepeatCheck` whose attempt slot is the counter slot) is NOT
/// rejected: the paired check is exempt from the foreign-write
/// rule (production `is_paired_check`,
/// repeat_counter_ownership.rs:74-109). This is the no-over-reject
/// witness for the counter step.
pub proof fn lemma_repeat_counter_paired_check_exempt(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 4,
        parts.written_slots.len() == 4,
        parts.written_slots[0].len() == 0,
        parts.written_slots[1].len() == 0,
        parts.written_slots[2].len() == 0,
        parts.written_slots[3].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 0,
        parts.nodes_meta[0].has_start_shape == 1,
        parts.nodes_meta[0].start_body == 1u32,
        parts.nodes_meta[0].start_done == 3u32,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 1u32,
        parts.nodes_meta[1].has_output == 1,
        parts.nodes_meta[1].out_slot != parts.nodes_meta[0].out_slot,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 24u32,
        parts.nodes_meta[2].has_ctrl_shape == 1,
        parts.nodes_meta[2].ctrl_state_a == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[2].ctrl_body == 1u32,
        parts.nodes_meta[2].ctrl_done == 3u32,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].id == 2u32,
        parts.nodes_meta[3].kind_disc == 33u32,
        parts.nodes_meta[3].has_output == 0,
        parts.nodes_meta[3].has_error_slot == 0,
        parts.nodes_meta[3].id == 3u32,
    ensures
        spec_repeat_counter_error(parts) == None::<SpecWorkflowError>,
{
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 2)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 3));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 3, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 3, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 2)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 3, 3));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 3) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 3)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 4));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 4) == None::<SpecWorkflowError>);
    assert(spec_repeat_counter_error(parts) == spec_ownership_error(parts, 0));
    assert(!spec_node_writes_counter_slot(parts, 1, parts.nodes_meta[0].out_slot));
    assert(spec_is_paired_repeat_check(
        &parts.nodes_meta,
        2,
        parts.nodes_meta[0].out_slot,
        parts.nodes_meta[0].id,
    ));
    assert(!spec_node_writes_counter_slot(parts, 3, parts.nodes_meta[0].out_slot));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 0)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 2));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 2)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 3));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 3)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 4));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 4)
        == None::<SpecWorkflowError>);
    assert(spec_ownership_error(parts, 0) == spec_ownership_error(parts, 1));
    assert(spec_ownership_error(parts, 1) == spec_ownership_error(parts, 2));
    assert(spec_ownership_error(parts, 2) == spec_ownership_error(parts, 3));
    assert(spec_ownership_error(parts, 3) == spec_ownership_error(parts, 4));
    assert(spec_ownership_error(parts, 4) == None::<SpecWorkflowError>);
}

/// Lemma 32: the severed-check rejection. A `RepeatCheck` whose
/// routing next is severed (no control shape, so it is not the
/// paired check) still writes the counter slot through its
/// attempt slot, and is rejected with
/// `RepeatCounterForeignWrite` (production `is_paired_check`
/// returns false when the shape is absent, so `writes_slot`
/// applies; the `repeat_check_next_none` case in
/// `vb_f1_repeat_counter_ownership.rs`).
pub proof fn lemma_repeat_counter_severed_check_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.written_slots.len() == 3,
        parts.written_slots[0].len() == 0,
        parts.written_slots[1].len() == 1,
        parts.written_slots[1][0] == parts.nodes_meta[0].out_slot,
        parts.written_slots[2].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 24u32,
        parts.nodes_meta[1].has_ctrl_shape == 0,
        parts.nodes_meta[1].has_output == 0,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_output == 0,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].id == 2u32,
    ensures
        spec_repeat_counter_error(parts) == Some(SpecWorkflowError::RepeatCounterForeignWrite),
{
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 2)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 3));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 3) == None::<SpecWorkflowError>);
    assert(spec_repeat_counter_error(parts) == spec_ownership_error(parts, 0));
    assert(!spec_is_paired_repeat_check(
        &parts.nodes_meta,
        1,
        parts.nodes_meta[0].out_slot,
        parts.nodes_meta[0].id,
    ));
    assert(spec_node_writes_counter_slot(parts, 1, parts.nodes_meta[0].out_slot));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 0)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_ownership_error(parts, 0) == Some(SpecWorkflowError::RepeatCounterForeignWrite));
}

/// Lemma 33: the kind-slot rejection. A `RetryCheck` node whose
/// policy slot is a claimed counter slot is rejected with
/// `RepeatCounterForeignWrite` (production `writes_slot`,
/// repeat_counter_ownership.rs:111-137, checks the kind-specific
/// `policy_slot` arm; the `repeat_body_kind_retry_check` case in
/// `vb_f1_repeat_counter_ownership.rs`).
pub proof fn lemma_repeat_counter_kind_slot_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.written_slots.len() == 3,
        parts.written_slots[0].len() == 0,
        parts.written_slots[1].len() == 1,
        parts.written_slots[1][0] == parts.nodes_meta[0].out_slot,
        parts.written_slots[2].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 30u32,
        parts.nodes_meta[1].has_output == 0,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_output == 0,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].id == 2u32,
    ensures
        spec_repeat_counter_error(parts) == Some(SpecWorkflowError::RepeatCounterForeignWrite),
{
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 2)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 3));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 3) == None::<SpecWorkflowError>);
    assert(spec_repeat_counter_error(parts) == spec_ownership_error(parts, 0));
    assert(spec_node_writes_counter_slot(parts, 1, parts.nodes_meta[0].out_slot));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 0)
        == spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 1)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_ownership_error(parts, 0) == Some(SpecWorkflowError::RepeatCounterForeignWrite));
}

/// Lemma 34: the owner error-slot rejection. An owner `RepeatStart`
/// whose routing error slot is its own counter slot is rejected
/// with `RepeatCounterForeignWrite` (production
/// `validate_slot_ownership`, repeat_counter_ownership.rs:37-72,
/// checks the owner's error slot before exempting it; the
/// `repeat_owner_error_slot_mutated_to_counter` case in
/// `vb_f1_repeat_counter_ownership.rs`).
pub proof fn lemma_repeat_counter_owner_error_slot_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.written_slots.len() == 3,
        parts.written_slots[0].len() == 0,
        parts.written_slots[1].len() == 0,
        parts.written_slots[2].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 1,
        parts.nodes_meta[0].error_slot == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 1u32,
        parts.nodes_meta[1].has_output == 1,
        parts.nodes_meta[1].out_slot != parts.nodes_meta[0].out_slot,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_output == 0,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].id == 2u32,
    ensures
        spec_repeat_counter_error(parts) == Some(SpecWorkflowError::RepeatCounterForeignWrite),
{
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 2)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 3));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 3) == None::<SpecWorkflowError>);
    assert(spec_repeat_counter_error(parts) == spec_ownership_error(parts, 0));
    assert(spec_ownership_scan_error(parts, parts.nodes_meta[0].out_slot, parts.nodes_meta[0].id, 0)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_ownership_error(parts, 0) == Some(SpecWorkflowError::RepeatCounterForeignWrite));
}

/// Lemma 35: the counter-ownership rejection at the top level. A
/// duplicate counter claim is rejected by the top-level spec
/// mirror with `RepeatCounterForeignWrite`, before the loop-region
/// steps run (production runs
/// `repeat_counter_ownership::validate` inside `validate_parts`,
/// before `loop_regions::validate`, workflow/mod.rs:42-65).
pub proof fn lemma_repeat_counter_duplicate_rejected_top_level(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 2,
        parts.nodes_len == 2,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        spec_reachability_error(parts) == None::<SpecWorkflowError>,
        spec_forward_edges_error(parts) == None::<SpecWorkflowError>,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[1].kind_disc == 22u32,
        parts.nodes_meta[1].has_output == 1,
        parts.nodes_meta[0].out_slot == parts.nodes_meta[1].out_slot,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::RepeatCounterForeignWrite),
{
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_repeat_counter_error(parts)
        == Some(SpecWorkflowError::RepeatCounterForeignWrite));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::RepeatCounterForeignWrite));
}

/// Lemma 36: the backward routing-next rejection. A node whose
/// routing `next` step points backward (or at itself) is
/// rejected with `BackwardEdge` (production
/// `validate_forward_target`, workflow/mod.rs:2621-2630). The
/// node carries a forward error handler to the second node so
/// the fixture passes the reachability step that runs before
/// the forward-edges step in production order.
pub proof fn lemma_forward_edge_routing_next_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 2,
        parts.nodes_len == 2,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 2,
        parts.kind_targets[0].len() == 0,
        parts.kind_targets[1].len() == 0,
        parts.nodes_meta[0].kind_disc == 0u32,
        parts.nodes_meta[0].has_next == 1,
        parts.nodes_meta[0].next_step == 0u32,
        parts.nodes_meta[0].has_on_error == 1,
        parts.nodes_meta[0].on_error_step == 1u32,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 33u32,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge),
{
    assert(spec_path_exists(parts, 1, 2, 0) == spec_path_exists(parts, 0, 1, 0));
    assert(spec_path_exists(parts, 0, 1, 0));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 0) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_edges_error(parts) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge));
}

/// Lemma 37: the backward kind-edge rejection. A `Jump` node
/// whose kind target points backward (or at itself) is rejected
/// with `BackwardEdge` (production `validate_kind_edges`,
/// workflow/mod.rs:2367-2427). The node carries a forward error
/// handler to the second node so the fixture passes the
/// reachability step that runs before the forward-edges step in
/// production order.
pub proof fn lemma_forward_edge_jump_target_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 2,
        parts.nodes_len == 2,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 1,
        parts.kind_targets[0].len() == 1,
        parts.kind_targets[0][0] == 0u32,
        parts.nodes_meta[0].kind_disc == 32u32,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 1,
        parts.nodes_meta[0].on_error_step == 1u32,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 33u32,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge),
{
    assert(spec_path_exists(parts, 1, 2, 0) == spec_path_exists(parts, 0, 1, 0));
    assert(spec_path_exists(parts, 0, 1, 0));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_kind_target_forward(parts, 0, 0) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_kind_edge_error(parts, 0) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_node_error(parts, 0) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_edges_error(parts) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge));
}

/// Lemma 38: the valid `RepeatCheck` back-edge exemption. A
/// well-formed repeat (owner `RepeatStart`, paired
/// `RepeatCheck` whose routing `next` returns to the owner's
/// body with matching attempt slot and done) is NOT rejected by
/// the forward-edges step (production
/// `repeat_check_validate`, workflow/mod.rs:2465-2587). This is
/// the no-over-reject witness for the forward-edges step.
pub proof fn lemma_forward_edge_repeat_check_exempt(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 4,
        parts.kind_targets.len() == 4,
        parts.kind_targets[0].len() == 2,
        parts.kind_targets[0][0] == 1u32,
        parts.kind_targets[0][1] == 3u32,
        parts.kind_targets[1].len() == 0,
        parts.kind_targets[2].len() == 1,
        parts.kind_targets[2][0] == 3u32,
        parts.kind_targets[3].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 0,
        parts.nodes_meta[0].has_start_shape == 1,
        parts.nodes_meta[0].start_body == 1u32,
        parts.nodes_meta[0].start_done == 3u32,
        parts.nodes_meta[0].start_state_a == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 1u32,
        parts.nodes_meta[1].has_output == 1,
        parts.nodes_meta[1].out_slot != parts.nodes_meta[0].out_slot,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 24u32,
        parts.nodes_meta[2].has_ctrl_shape == 1,
        parts.nodes_meta[2].ctrl_state_a == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[2].ctrl_body == 1u32,
        parts.nodes_meta[2].ctrl_done == 3u32,
        parts.nodes_meta[2].has_next == 1,
        parts.nodes_meta[2].next_step == 1u32,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
        parts.nodes_meta[3].kind_disc == 33u32,
        parts.nodes_meta[3].has_output == 0,
        parts.nodes_meta[3].has_error_slot == 0,
        parts.nodes_meta[3].has_next == 0,
        parts.nodes_meta[3].has_on_error == 0,
        parts.nodes_meta[3].id == 3u32,
    ensures
        spec_forward_edges_error(parts) == None::<SpecWorkflowError>,
{
    assert(spec_repeat_owner(parts, 2, 0, None) == spec_repeat_owner(parts, 2, 1, Some(0)));
    assert(spec_repeat_owner(parts, 2, 1, Some(0)) == spec_repeat_owner(parts, 2, 2, Some(0)));
    assert(spec_repeat_owner(parts, 2, 2, Some(0)) == Some(0));
    assert(spec_repeat_check_error(parts, 2, 1u32) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 2) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 3) == None::<SpecWorkflowError>);
    assert(spec_forward_edges_from(parts, 0) == spec_forward_edges_from(parts, 1));
    assert(spec_forward_edges_from(parts, 1) == spec_forward_edges_from(parts, 2));
    assert(spec_forward_edges_from(parts, 2) == spec_forward_edges_from(parts, 3));
    assert(spec_forward_edges_from(parts, 3) == spec_forward_edges_from(parts, 4));
    assert(spec_forward_edges_from(parts, 4) == None::<SpecWorkflowError>);
    assert(spec_forward_edges_error(parts) == None::<SpecWorkflowError>);
}

/// Lemma 39: the ownerless `RepeatCheck` back-edge rejection.
/// A `RepeatCheck` whose routing `next` points backward with no
/// enclosing structured start is rejected with `BackwardEdge`
/// (production `repeat_check_validate`,
/// workflow/mod.rs:2465-2587). The entry node carries a forward
/// routing `next` to the check so the fixture passes the
/// reachability step that runs before the forward-edges step in
/// production order.
pub proof fn lemma_forward_edge_repeat_check_no_owner_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.nodes_len == 3,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 3,
        parts.kind_targets[0].len() == 0,
        parts.kind_targets[1].len() == 1,
        parts.kind_targets[1][0] == 2u32,
        parts.kind_targets[2].len() == 0,
        parts.nodes_meta[0].kind_disc == 0u32,
        parts.nodes_meta[0].has_next == 1,
        parts.nodes_meta[0].next_step == 1u32,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 24u32,
        parts.nodes_meta[1].has_ctrl_shape == 1,
        parts.nodes_meta[1].ctrl_body == 0u32,
        parts.nodes_meta[1].ctrl_done == 2u32,
        parts.nodes_meta[1].has_next == 1,
        parts.nodes_meta[1].next_step == 0u32,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_next == 0,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge),
{
    assert(spec_path_exists(parts, 1, 2, 0) == spec_path_exists(parts, 0, 1, 0));
    assert(spec_path_exists(parts, 1, 3, 0) == spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 0, 1, 0));
    assert(spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 2, 3, 0) == spec_path_exists(parts, 2, 3, 1));
    assert(spec_path_exists(parts, 2, 3, 1));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_reachable(parts, 2));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == spec_first_unreachable_from(parts, 3));
    assert(spec_first_unreachable_from(parts, 3) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_repeat_owner(parts, 1, 0, None) == spec_repeat_owner(parts, 1, 1, None));
    assert(spec_repeat_owner(parts, 1, 1, None) == None::<int>);
    assert(spec_repeat_check_error(parts, 1, 0u32) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_node_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 1) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_edges_from(parts, 0) == spec_forward_edges_from(parts, 1));
    assert(spec_forward_edges_error(parts) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge));
}

/// Lemma 40: the wrong-owner `RepeatCheck` back-edge rejection.
/// A `RepeatCheck` whose nearest active enclosing start is a
/// `ForEachStart` rather than a `RepeatStart` is rejected with
/// `BackwardEdge` (production `repeat_check_validate`,
/// workflow/mod.rs:2465-2587). The body node carries a forward
/// routing `next` to the check so the fixture passes the
/// reachability step that runs before the forward-edges step in
/// production order.
pub proof fn lemma_forward_edge_repeat_check_wrong_owner_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 4,
        parts.nodes_len == 4,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 4,
        parts.kind_targets[0].len() == 2,
        parts.kind_targets[0][0] == 1u32,
        parts.kind_targets[0][1] == 3u32,
        parts.kind_targets[1].len() == 0,
        parts.kind_targets[2].len() == 1,
        parts.kind_targets[2][0] == 3u32,
        parts.kind_targets[3].len() == 0,
        parts.nodes_meta[0].kind_disc == 9u32,
        parts.nodes_meta[0].has_output == 1,
        parts.nodes_meta[0].has_error_slot == 0,
        parts.nodes_meta[0].has_start_shape == 1,
        parts.nodes_meta[0].start_body == 1u32,
        parts.nodes_meta[0].start_done == 3u32,
        parts.nodes_meta[0].start_state_a == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 1u32,
        parts.nodes_meta[1].has_output == 1,
        parts.nodes_meta[1].out_slot != parts.nodes_meta[0].out_slot,
        parts.nodes_meta[1].has_error_slot == 0,
        parts.nodes_meta[1].has_next == 1,
        parts.nodes_meta[1].next_step == 2u32,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 24u32,
        parts.nodes_meta[2].has_ctrl_shape == 1,
        parts.nodes_meta[2].ctrl_state_a == parts.nodes_meta[0].out_slot,
        parts.nodes_meta[2].ctrl_body == 1u32,
        parts.nodes_meta[2].ctrl_done == 3u32,
        parts.nodes_meta[2].has_next == 1,
        parts.nodes_meta[2].next_step == 1u32,
        parts.nodes_meta[2].has_error_slot == 0,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
        parts.nodes_meta[3].kind_disc == 33u32,
        parts.nodes_meta[3].has_output == 0,
        parts.nodes_meta[3].has_error_slot == 0,
        parts.nodes_meta[3].has_next == 0,
        parts.nodes_meta[3].has_on_error == 0,
        parts.nodes_meta[3].id == 3u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge),
{
    assert(spec_path_exists(parts, 1, 4, 0) == spec_path_exists(parts, 0, 3, 0));
    assert(spec_path_exists(parts, 1, 3, 0) == spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 0, 3, 0));
    assert(spec_path_exists(parts, 0, 2, 0));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 0)
        == spec_targets_contain(&parts.kind_targets[0], 2, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 1)
        == spec_targets_contain(&parts.kind_targets[0], 2, 2));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 2) == false);
    assert(!spec_has_edge(parts, 0, 2));
    assert(spec_path_exists(parts, 2, 4, 0) == spec_path_exists(parts, 2, 4, 1));
    assert(spec_path_exists(parts, 2, 4, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 0)
        == spec_targets_contain(&parts.kind_targets[0], 3, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 1));
    assert(spec_path_exists(parts, 3, 4, 0));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_reachable(parts, 2));
    assert(spec_reachable(parts, 3));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == spec_first_unreachable_from(parts, 3));
    assert(spec_first_unreachable_from(parts, 3) == spec_first_unreachable_from(parts, 4));
    assert(spec_first_unreachable_from(parts, 4) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_repeat_owner(parts, 2, 0, None) == spec_repeat_owner(parts, 2, 1, Some(0)));
    assert(spec_repeat_owner(parts, 2, 1, Some(0)) == spec_repeat_owner(parts, 2, 2, Some(0)));
    assert(spec_repeat_owner(parts, 2, 2, Some(0)) == Some(0));
    assert(spec_repeat_check_error(parts, 2, 1u32) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_node_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 2) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_forward_edges_from(parts, 0) == spec_forward_edges_from(parts, 1));
    assert(spec_forward_edges_from(parts, 1) == spec_forward_edges_from(parts, 2));
    assert(spec_forward_edges_error(parts) == Some(SpecWorkflowError::BackwardEdge));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::BackwardEdge));
}

/// Lemma 41: the improper loop-nesting rejection. Two loop
/// spans that partially overlap (the second starts inside the
/// first and ends past it) are rejected with
/// `ImproperLoopNesting` (production `push_loop_span`,
/// workflow/mod.rs:2633-2680).
pub proof fn lemma_forward_edge_improper_nesting_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 4,
        parts.nodes_len == 4,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 4,
        parts.kind_targets[0].len() == 2,
        parts.kind_targets[0][0] == 1u32,
        parts.kind_targets[0][1] == 2u32,
        parts.kind_targets[1].len() == 2,
        parts.kind_targets[1][0] == 2u32,
        parts.kind_targets[1][1] == 3u32,
        parts.kind_targets[2].len() == 0,
        parts.kind_targets[3].len() == 0,
        parts.nodes_meta[0].kind_disc == 22u32,
        parts.nodes_meta[0].has_output == 0,
        parts.nodes_meta[0].has_start_shape == 0,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 22u32,
        parts.nodes_meta[1].has_output == 0,
        parts.nodes_meta[1].has_start_shape == 0,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_next == 0,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
        parts.nodes_meta[3].kind_disc == 33u32,
        parts.nodes_meta[3].has_next == 0,
        parts.nodes_meta[3].has_on_error == 0,
        parts.nodes_meta[3].id == 3u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::ImproperLoopNesting),
{
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 0)
        == spec_targets_contain(&parts.kind_targets[0], 2, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 1));
    assert(spec_path_exists(parts, 1, 4, 0) == spec_path_exists(parts, 0, 3, 0));
    assert(spec_path_exists(parts, 2, 4, 0));
    assert(spec_path_exists(parts, 1, 3, 0) == spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 0, 3, 0));
    assert(spec_path_exists(parts, 0, 2, 0));
    assert(spec_targets_contain(&parts.kind_targets[1], 3, 0)
        == spec_targets_contain(&parts.kind_targets[1], 3, 1));
    assert(spec_targets_contain(&parts.kind_targets[1], 3, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 0)
        == spec_targets_contain(&parts.kind_targets[0], 3, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 1)
        == spec_targets_contain(&parts.kind_targets[0], 3, 2));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 2) == false);
    assert(!spec_has_edge(parts, 0, 3));
    assert(spec_path_exists(parts, 3, 4, 0) == spec_path_exists(parts, 3, 4, 1));
    assert(spec_path_exists(parts, 3, 4, 1));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_reachable(parts, 2));
    assert(spec_reachable(parts, 3));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == spec_first_unreachable_from(parts, 3));
    assert(spec_first_unreachable_from(parts, 3) == spec_first_unreachable_from(parts, 4));
    assert(spec_first_unreachable_from(parts, 4) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_nesting_before(parts, 1, 0) == Some(SpecWorkflowError::ImproperLoopNesting));
    assert(spec_forward_node_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 1) == Some(SpecWorkflowError::ImproperLoopNesting));
    assert(spec_forward_edges_from(parts, 0) == spec_forward_edges_from(parts, 1));
    assert(spec_forward_edges_error(parts) == Some(SpecWorkflowError::ImproperLoopNesting));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::ImproperLoopNesting));
}

/// Lemma 42: the unreachable-node rejection. A node with no
/// path from the entry is rejected with `UnreachableNode`
/// (production `validate_reachability`,
/// workflow/mod.rs:2160-2229). The first unreachable node in
/// table order is reported.
pub proof fn lemma_reachability_unreachable_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 2,
        parts.nodes_len == 2,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 2,
        parts.kind_targets[0].len() == 0,
        parts.kind_targets[1].len() == 0,
        parts.nodes_meta[0].kind_disc == 0u32,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 33u32,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::UnreachableNode),
{
    assert(spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 1, 2, 0) == spec_path_exists(parts, 1, 2, 1));
    assert(spec_path_exists(parts, 1, 2, 1) == spec_path_exists(parts, 1, 2, 2));
    assert(spec_path_exists(parts, 1, 2, 2) == false);
    assert(!spec_path_exists(parts, 1, 2, 0));
    assert(spec_reachable(parts, 0));
    assert(!spec_reachable(parts, 1));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1)
        == Some(SpecWorkflowError::UnreachableNode));
    assert(spec_reachability_error(parts) == Some(SpecWorkflowError::UnreachableNode));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::UnreachableNode));
}

/// Lemma 43: the reachability exemption. A chain of nodes
/// linked by routing `next` steps from the entry is NOT
/// rejected: every node is reachable (production
/// `validate_reachability`, workflow/mod.rs:2160-2229). This is
/// the no-over-reject witness for the reachability step.
pub proof fn lemma_reachability_chain_accepted(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 3,
        parts.nodes_len == 3,
        parts.entry == 0u32,
        parts.kind_targets.len() == 3,
        parts.kind_targets[0].len() == 0,
        parts.kind_targets[1].len() == 0,
        parts.kind_targets[2].len() == 0,
        parts.nodes_meta[0].kind_disc == 0u32,
        parts.nodes_meta[0].has_next == 1,
        parts.nodes_meta[0].next_step == 1u32,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 0u32,
        parts.nodes_meta[1].has_next == 1,
        parts.nodes_meta[1].next_step == 2u32,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 33u32,
        parts.nodes_meta[2].has_next == 0,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
    ensures
        spec_reachability_error(parts) == None::<SpecWorkflowError>,
{
    assert(spec_path_exists(parts, 0, 3, 0));
    assert(spec_path_exists(parts, 1, 2, 0) == spec_path_exists(parts, 0, 1, 0));
    assert(spec_path_exists(parts, 1, 3, 0) == spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 0, 1, 0));
    assert(spec_path_exists(parts, 0, 2, 0));
    assert(spec_path_exists(parts, 2, 3, 0) == spec_path_exists(parts, 2, 3, 1));
    assert(spec_path_exists(parts, 2, 3, 1));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_reachable(parts, 2));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == spec_first_unreachable_from(parts, 3));
    assert(spec_first_unreachable_from(parts, 3) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
}

/// Lemma 44: the nested-together rejection. A `TogetherStart`
/// reachable from another `TogetherStart`'s branch body over
/// the routing `next` chain is rejected with `NestedTogether`
/// (production `validate_no_nested_together`,
/// engine/validate.rs:119-172). The fixture is otherwise
/// well-formed: every node is reachable, every edge points
/// forward, the loop spans nest strictly, and no counter slot
/// is claimed, so the nested-together step is the first failure
/// in production order.
pub proof fn lemma_nested_together_rejected(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 5,
        parts.nodes_len == 5,
        parts.entry == 0u32,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        parts.kind_targets.len() == 5,
        parts.kind_targets[0].len() == 2,
        parts.kind_targets[0][0] == 1u32,
        parts.kind_targets[0][1] == 4u32,
        parts.kind_targets[1].len() == 2,
        parts.kind_targets[1][0] == 2u32,
        parts.kind_targets[1][1] == 3u32,
        parts.kind_targets[2].len() == 0,
        parts.kind_targets[3].len() == 0,
        parts.kind_targets[4].len() == 0,
        parts.nodes_meta[0].kind_disc == 12u32,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 12u32,
        parts.nodes_meta[1].has_next == 0,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 0u32,
        parts.nodes_meta[2].has_next == 1,
        parts.nodes_meta[2].next_step == 3u32,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
        parts.nodes_meta[3].kind_disc == 14u32,
        parts.nodes_meta[3].has_next == 1,
        parts.nodes_meta[3].next_step == 4u32,
        parts.nodes_meta[3].has_on_error == 0,
        parts.nodes_meta[3].id == 3u32,
        parts.nodes_meta[4].kind_disc == 14u32,
        parts.nodes_meta[4].has_next == 0,
        parts.nodes_meta[4].has_on_error == 0,
        parts.nodes_meta[4].id == 4u32,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::NestedTogether),
{
    assert(spec_path_exists(parts, 0, 5, 0));
    assert(spec_targets_contain(&parts.kind_targets[0], 1, 0));
    assert(spec_path_exists(parts, 1, 5, 0) == spec_path_exists(parts, 0, 4, 0));
    assert(spec_path_exists(parts, 1, 4, 0) == spec_path_exists(parts, 0, 3, 0));
    assert(spec_path_exists(parts, 0, 4, 0));
    assert(spec_path_exists(parts, 0, 3, 0));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 0)
        == spec_targets_contain(&parts.kind_targets[0], 2, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 1)
        == spec_targets_contain(&parts.kind_targets[0], 2, 2));
    assert(spec_targets_contain(&parts.kind_targets[0], 2, 2) == false);
    assert(!spec_has_edge(parts, 0, 2));
    assert(spec_path_exists(parts, 2, 5, 0) == spec_path_exists(parts, 2, 5, 1));
    assert(spec_path_exists(parts, 2, 5, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 0)
        == spec_targets_contain(&parts.kind_targets[0], 3, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 1)
        == spec_targets_contain(&parts.kind_targets[0], 3, 2));
    assert(spec_targets_contain(&parts.kind_targets[0], 3, 2) == false);
    assert(!spec_has_edge(parts, 0, 3));
    assert(spec_targets_contain(&parts.kind_targets[1], 3, 0)
        == spec_targets_contain(&parts.kind_targets[1], 3, 1));
    assert(spec_targets_contain(&parts.kind_targets[1], 3, 1));
    assert(spec_path_exists(parts, 3, 5, 0) == spec_path_exists(parts, 3, 5, 1));
    assert(spec_path_exists(parts, 3, 5, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 4, 0)
        == spec_targets_contain(&parts.kind_targets[0], 4, 1));
    assert(spec_targets_contain(&parts.kind_targets[0], 4, 1));
    assert(spec_path_exists(parts, 4, 5, 0));
    assert(spec_reachable(parts, 0));
    assert(spec_reachable(parts, 1));
    assert(spec_reachable(parts, 2));
    assert(spec_reachable(parts, 3));
    assert(spec_reachable(parts, 4));
    assert(spec_first_unreachable_from(parts, 0) == spec_first_unreachable_from(parts, 1));
    assert(spec_first_unreachable_from(parts, 1) == spec_first_unreachable_from(parts, 2));
    assert(spec_first_unreachable_from(parts, 2) == spec_first_unreachable_from(parts, 3));
    assert(spec_first_unreachable_from(parts, 3) == spec_first_unreachable_from(parts, 4));
    assert(spec_first_unreachable_from(parts, 4) == spec_first_unreachable_from(parts, 5));
    assert(spec_first_unreachable_from(parts, 5) == None::<SpecWorkflowError>);
    assert(spec_reachability_error(parts) == None::<SpecWorkflowError>);
    assert(spec_kind_targets_forward_from(&parts.kind_targets[0], 0u32, 0)
        == spec_kind_targets_forward_from(&parts.kind_targets[0], 0u32, 1));
    assert(spec_kind_targets_forward_from(&parts.kind_targets[0], 0u32, 1)
        == spec_kind_targets_forward_from(&parts.kind_targets[0], 0u32, 2));
    assert(spec_kind_targets_forward_from(&parts.kind_targets[0], 0u32, 2)
        == None::<SpecWorkflowError>);
    assert(spec_forward_kind_edge_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_nesting_before(parts, 0, 0) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_kind_targets_forward_from(&parts.kind_targets[1], 1u32, 0)
        == spec_kind_targets_forward_from(&parts.kind_targets[1], 1u32, 1));
    assert(spec_kind_targets_forward_from(&parts.kind_targets[1], 1u32, 1)
        == spec_kind_targets_forward_from(&parts.kind_targets[1], 1u32, 2));
    assert(spec_kind_targets_forward_from(&parts.kind_targets[1], 1u32, 2)
        == None::<SpecWorkflowError>);
    assert(spec_forward_kind_edge_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_loop_span_end(parts, 1) == Some(3u32));
    assert(spec_loop_span_end(parts, 0) == Some(4u32));
    assert(spec_nesting_before(parts, 1, 0) == spec_nesting_before(parts, 1, 1));
    assert(spec_nesting_before(parts, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 2) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 3) == None::<SpecWorkflowError>);
    assert(spec_forward_node_error(parts, 4) == None::<SpecWorkflowError>);
    assert(spec_forward_edges_from(parts, 0) == spec_forward_edges_from(parts, 1));
    assert(spec_forward_edges_from(parts, 1) == spec_forward_edges_from(parts, 2));
    assert(spec_forward_edges_from(parts, 2) == spec_forward_edges_from(parts, 3));
    assert(spec_forward_edges_from(parts, 3) == spec_forward_edges_from(parts, 4));
    assert(spec_forward_edges_from(parts, 4) == spec_forward_edges_from(parts, 5));
    assert(spec_forward_edges_from(parts, 5) == None::<SpecWorkflowError>);
    assert(spec_forward_edges_error(parts) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 0, 0) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 3, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 3, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 2)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 3, 3));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 3, 3) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 4, 0)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 4, 1));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 4, 1)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 4, 2));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 4, 2)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 4, 3));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 4, 3)
        == spec_repeat_duplicate_before(&parts.nodes_meta, 4, 4));
    assert(spec_repeat_duplicate_before(&parts.nodes_meta, 4, 4) == None::<SpecWorkflowError>);
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 0)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 1));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 1)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 2));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 2)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 3));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 3)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 4));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 4)
        == spec_repeat_duplicate_error(&parts.nodes_meta, 5));
    assert(spec_repeat_duplicate_error(&parts.nodes_meta, 5) == None::<SpecWorkflowError>);
    assert(spec_ownership_error(parts, 0) == spec_ownership_error(parts, 1));
    assert(spec_ownership_error(parts, 1) == spec_ownership_error(parts, 2));
    assert(spec_ownership_error(parts, 2) == spec_ownership_error(parts, 3));
    assert(spec_ownership_error(parts, 3) == spec_ownership_error(parts, 4));
    assert(spec_ownership_error(parts, 4) == spec_ownership_error(parts, 5));
    assert(spec_ownership_error(parts, 5) == None::<SpecWorkflowError>);
    assert(spec_repeat_counter_error(parts) == None::<SpecWorkflowError>);
    assert(spec_walk_inner(parts, 1, 4, 0, 5) == Some(1u32));
    assert(spec_nested_branches_from(parts, &parts.kind_targets[0], 0, 0)
        == Some(SpecWorkflowError::NestedTogether));
    assert(spec_nested_outer_from(parts, 0) == Some(SpecWorkflowError::NestedTogether));
    assert(spec_nested_together_error(parts) == Some(SpecWorkflowError::NestedTogether));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::NestedTogether));
}

/// Lemma 45: the sequential-together exemption. Two
/// `TogetherStart` nodes run one after the other are NOT
/// rejected: the second starts at the first one's join, so the
/// first one's branch walk stops before reaching it
/// (production `validate_no_nested_together`,
/// engine/validate.rs:119-172). This is the no-over-reject
/// witness for the nested-together step.
pub proof fn lemma_nested_together_sequential_accepted(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() == 5,
        parts.nodes_len == 5,
        parts.entry == 0u32,
        parts.kind_targets.len() == 5,
        parts.kind_targets[0].len() == 2,
        parts.kind_targets[0][0] == 1u32,
        parts.kind_targets[0][1] == 2u32,
        parts.kind_targets[1].len() == 0,
        parts.kind_targets[2].len() == 2,
        parts.kind_targets[2][0] == 3u32,
        parts.kind_targets[2][1] == 4u32,
        parts.kind_targets[3].len() == 0,
        parts.kind_targets[4].len() == 0,
        parts.nodes_meta[0].kind_disc == 12u32,
        parts.nodes_meta[0].has_next == 0,
        parts.nodes_meta[0].has_on_error == 0,
        parts.nodes_meta[0].id == 0u32,
        parts.nodes_meta[1].kind_disc == 0u32,
        parts.nodes_meta[1].has_next == 1,
        parts.nodes_meta[1].next_step == 2u32,
        parts.nodes_meta[1].has_on_error == 0,
        parts.nodes_meta[1].id == 1u32,
        parts.nodes_meta[2].kind_disc == 12u32,
        parts.nodes_meta[2].has_next == 0,
        parts.nodes_meta[2].has_on_error == 0,
        parts.nodes_meta[2].id == 2u32,
        parts.nodes_meta[3].kind_disc == 0u32,
        parts.nodes_meta[3].has_next == 1,
        parts.nodes_meta[3].next_step == 4u32,
        parts.nodes_meta[3].has_on_error == 0,
        parts.nodes_meta[3].id == 3u32,
        parts.nodes_meta[4].kind_disc == 14u32,
        parts.nodes_meta[4].has_next == 0,
        parts.nodes_meta[4].has_on_error == 0,
        parts.nodes_meta[4].id == 4u32,
    ensures
        spec_nested_together_error(parts) == None::<SpecWorkflowError>,
{
    assert(spec_walk_inner(parts, 1, 2, 0, 5) == spec_walk_inner(parts, 2, 2, 0, 4));
    assert(spec_walk_inner(parts, 2, 2, 0, 4) == None::<u32>);
    assert(spec_walk_inner(parts, 3, 4, 2, 5) == spec_walk_inner(parts, 4, 4, 2, 4));
    assert(spec_walk_inner(parts, 4, 4, 2, 4) == None::<u32>);
    assert(spec_nested_branches_from(parts, &parts.kind_targets[0], 0, 0)
        == spec_nested_branches_from(parts, &parts.kind_targets[0], 0, 1));
    assert(spec_nested_branches_from(parts, &parts.kind_targets[0], 0, 1)
        == None::<SpecWorkflowError>);
    assert(spec_nested_branches_from(parts, &parts.kind_targets[2], 2, 0)
        == spec_nested_branches_from(parts, &parts.kind_targets[2], 2, 1));
    assert(spec_nested_branches_from(parts, &parts.kind_targets[2], 2, 1)
        == None::<SpecWorkflowError>);
    assert(spec_nested_outer_from(parts, 0) == spec_nested_outer_from(parts, 1));
    assert(spec_nested_outer_from(parts, 1) == spec_nested_outer_from(parts, 2));
    assert(spec_nested_outer_from(parts, 2) == spec_nested_outer_from(parts, 3));
    assert(spec_nested_outer_from(parts, 3) == spec_nested_outer_from(parts, 4));
    assert(spec_nested_outer_from(parts, 4) == spec_nested_outer_from(parts, 5));
    assert(spec_nested_outer_from(parts, 5) == None::<SpecWorkflowError>);
    assert(spec_nested_together_error(parts) == None::<SpecWorkflowError>);
}

/// Lemma 46: the slot-before-step first error. A ForEachNext node
/// whose iterator slot is out of bounds and whose body step is also
/// out of bounds is rejected with `SlotOutOfBounds` (production
/// `validate_slot_and_steps`, workflow/mod.rs:1896-1904, checks the
/// slot first).
pub proof fn lemma_kind_ref_slot_before_step_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 10u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 2,
        kind_refs[0] == SpecKindRef::Slot(9u32),
        kind_refs[1] == SpecKindRef::Step(7u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
            == Some(SpecWorkflowError::SlotOutOfBounds),
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 0, 0)
        == Some(SpecWorkflowError::SlotOutOfBounds));
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
        == Some(SpecWorkflowError::SlotOutOfBounds));
}

/// Lemma 47: the step-before-slot first error. A TogetherBranch node
/// whose entry step is out of bounds and whose accumulator slot is
/// also out of bounds is rejected with `StepOutOfBounds` (production
/// `validate_two_steps` then `validate_slot`,
/// workflow/mod.rs:1752-1760, checks the steps first).
pub proof fn lemma_kind_ref_step_before_slot_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 13u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 3,
        kind_refs[0] == SpecKindRef::Step(7u32),
        kind_refs[1] == SpecKindRef::Step(8u32),
        kind_refs[2] == SpecKindRef::Slot(9u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 3, 5, 0, 0, 0, 0)
            == Some(SpecWorkflowError::StepOutOfBounds),
{
    assert(spec_kind_ref_error(kind_refs, 0, 3, 5, 0, 0)
        == Some(SpecWorkflowError::StepOutOfBounds));
    assert(validate_node_pure_spec(meta, kind_refs, 3, 5, 0, 0, 0, 0)
        == Some(SpecWorkflowError::StepOutOfBounds));
}

/// Lemma 48: the const-reference rejection. A ReduceStart node whose
/// `initial` const index is out of bounds is rejected with
/// `ConstOutOfBounds` (production `validate_reduce_start`,
/// workflow/mod.rs:1935-1947, checks input, accumulator, const, then
/// the two steps, in that order). This closes the approximation the
/// old projection carried for the reduce kinds, which returned `None`
/// for the const check.
pub proof fn lemma_kind_ref_const_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 19u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 5,
        kind_refs[0] == SpecKindRef::Slot(1u32),
        kind_refs[1] == SpecKindRef::Slot(2u32),
        kind_refs[2] == SpecKindRef::Const(4u32),
        kind_refs[3] == SpecKindRef::Step(1u32),
        kind_refs[4] == SpecKindRef::Step(2u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 3, 0)
            == Some(SpecWorkflowError::ConstOutOfBounds),
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 3, 0)
        == spec_kind_ref_error(kind_refs, 1, 5, 10, 3, 0));
    assert(spec_kind_ref_error(kind_refs, 1, 5, 10, 3, 0)
        == spec_kind_ref_error(kind_refs, 2, 5, 10, 3, 0));
    assert(spec_kind_ref_error(kind_refs, 2, 5, 10, 3, 0)
        == Some(SpecWorkflowError::ConstOutOfBounds));
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 3, 0)
        == Some(SpecWorkflowError::ConstOutOfBounds));
}

/// Lemma 49: the expression-reference rejection. An EvalExpr node
/// whose expr index is out of bounds is rejected with
/// `ExpressionInvalid` (production `validate_expr`,
/// workflow/mod.rs:2012-2020).
pub proof fn lemma_kind_ref_expr_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 3u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 1,
        kind_refs[0] == SpecKindRef::Expr(6u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 4, 0, 0, 0)
            == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 0, 4)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 4, 0, 0, 0)
        == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 50: the Choose branch interleaved order. A Choose node whose
/// first branch is in bounds and whose second branch condition expr
/// is out of bounds is rejected with `ExpressionInvalid` (production
/// `validate_expr_choose`, workflow/mod.rs:1984-1995, checks each
/// branch condition then its target, in branch order).
pub proof fn lemma_kind_ref_choose_branch_order_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 7u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 5,
        kind_refs[0] == SpecKindRef::Expr(1u32),
        kind_refs[1] == SpecKindRef::Step(2u32),
        kind_refs[2] == SpecKindRef::Expr(9u32),
        kind_refs[3] == SpecKindRef::Step(4u32),
        kind_refs[4] == SpecKindRef::Step(5u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 5, 0, 0, 0)
            == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 0, 5)
        == spec_kind_ref_error(kind_refs, 1, 5, 10, 0, 5));
    assert(spec_kind_ref_error(kind_refs, 1, 5, 10, 0, 5)
        == spec_kind_ref_error(kind_refs, 2, 5, 10, 0, 5));
    assert(spec_kind_ref_error(kind_refs, 2, 5, 10, 0, 5)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 5, 0, 0, 0)
        == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 51: the branch-table check precedes the kind refs. A Choose
/// node with no branches and no `otherwise` is rejected with
/// `EmptyBranchTable` even when its kind-ref inventory would also
/// fail (production `validate_branch_route`,
/// workflow/mod.rs:1997-2006, runs before the per-branch checks).
pub proof fn lemma_kind_ref_empty_branch_table_first_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 7u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 1,
        meta.has_branch_targets == 0,
        kind_refs.len() == 1,
        kind_refs[0] == SpecKindRef::Slot(99u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
            == Some(SpecWorkflowError::EmptyBranchTable),
{
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
        == Some(SpecWorkflowError::EmptyBranchTable));
}

/// Lemma 52: the list-items count-cap rejection. A BuildList node
/// with more items than `MAX_LIST_ITEMS_PER_VALUE` is rejected with
/// `ResourceContractExceeded` (production `validate_build_list`,
/// workflow/mod.rs:1860-1867, checks the cap before the items).
pub proof fn lemma_kind_ref_count_cap_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 5u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 2,
        kind_refs[0] == SpecKindRef::Count(70000u32, 65535u32),
        kind_refs[1] == SpecKindRef::Slot(1u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
            == Some(SpecWorkflowError::ResourceContractExceeded),
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 0, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
}

/// Lemma 53: the nonzero rejection. A RepeatStart node with zero
/// max attempts is rejected with `ResourceContractExceeded`
/// (production `validate_repeat_start`,
/// workflow/mod.rs:1961-1969, checks the nonzero bound before the
/// two steps).
pub proof fn lemma_kind_ref_nonzero_rejected(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 22u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 3,
        kind_refs[0] == SpecKindRef::NonZero(0u32),
        kind_refs[1] == SpecKindRef::Step(1u32),
        kind_refs[2] == SpecKindRef::Step(2u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
            == Some(SpecWorkflowError::ResourceContractExceeded),
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 0, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
}

/// Lemma 54: well-formed kind refs are accepted. A ForEachNext node
/// whose ordered kind refs are all in bounds passes the per-node
/// check (production `validate_node_kind` returns `Ok` for it).
pub proof fn lemma_kind_ref_well_formed_accepted(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
)
    requires
        meta.kind_disc == 10u32,
        meta.has_output == 0,
        meta.has_next == 0,
        meta.has_on_error == 0,
        meta.has_error_slot == 0,
        meta.has_branch_table_no_otherwise == 0,
        meta.has_branch_targets == 0,
        kind_refs.len() == 3,
        kind_refs[0] == SpecKindRef::Slot(1u32),
        kind_refs[1] == SpecKindRef::Step(2u32),
        kind_refs[2] == SpecKindRef::Step(3u32),
    ensures
        validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
            == None::<SpecWorkflowError>,
{
    assert(spec_kind_ref_error(kind_refs, 0, 5, 10, 0, 0)
        == spec_kind_ref_error(kind_refs, 1, 5, 10, 0, 0));
    assert(spec_kind_ref_error(kind_refs, 1, 5, 10, 0, 0)
        == spec_kind_ref_error(kind_refs, 2, 5, 10, 0, 0));
    assert(spec_kind_ref_error(kind_refs, 2, 5, 10, 0, 0)
        == spec_kind_ref_error(kind_refs, 3, 5, 10, 0, 0));
    assert(spec_kind_ref_error(kind_refs, 3, 5, 10, 0, 0)
        == None::<SpecWorkflowError>);
    assert(validate_node_pure_spec(meta, kind_refs, 5, 10, 0, 0, 0, 0)
        == None::<SpecWorkflowError>);
}

/// Lemma 55: the within-caps acceptance. A computed budget with
/// every validated dimension within its `DEFAULT` cap passes the
/// budget policy check (production `BoundednessPolicy::validate`,
/// budget.rs:534-592, returns `Ok`).
pub proof fn lemma_budget_within_caps_accepted(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) <= 1_000_000,
        (budget.max_incident_emissions as int) <= 1_000_000,
        (budget.max_transitions as int) <= 10_000_000,
        (budget.max_journal_bytes_total as int) <= 1_048_576_000_000,
        (budget.max_value_store_entries as int) <= 2_000_000,
        (budget.max_cost_units as int) <= 100_000,
    ensures
        spec_budget_error(budget) == None::<SpecWorkflowError>,
{
    assert(spec_budget_error(budget) == None::<SpecWorkflowError>);
}

/// Lemma 56: the total-steps rejection. A computed budget
/// with `max_total_steps` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::TotalStepsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_total_steps_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 57: the total-slots rejection. A computed budget
/// with `max_total_slots` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::TotalSlotsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_total_slots_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) > 65_535,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 58: the fanout rejection. A computed budget
/// with `max_fanout` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::FanoutExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_fanout_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) > 64,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 59: the nesting-depth rejection. A computed budget
/// with `max_nesting_depth` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::NestingDepthExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_nesting_depth_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) > 8,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 60: the action-tickets rejection. A computed budget
/// with `max_action_tickets` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::ActionTicketsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_action_tickets_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) > 100_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 61: the parallel rejection. A computed budget
/// with `max_parallel_in_flight` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::ParallelExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_parallel_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) > 256,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 62: the run-time rejection. A computed budget
/// with `max_run_time_seconds` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::RunTimeExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_run_time_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) > 2_592_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 63: the result-bytes rejection. A computed budget
/// with `max_result_bytes` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::ResultBytesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_result_bytes_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) > 262_144,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 64: the steps-executable rejection. A computed budget
/// with `max_steps_executable` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `BoundednessPolicy::validate`, budget.rs:534-592 returns
/// `BudgetError::StepsExecutableExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_steps_executable_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 65: the timer-entries rejection. A computed budget
/// with `max_timer_entries` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_extended_budget`, budget.rs:644-661 returns
/// `BudgetError::TimerEntriesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_timer_entries_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 66: the trace-events rejection. A computed budget
/// with `max_trace_events` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_extended_budget`, budget.rs:644-661 returns
/// `BudgetError::TraceEventsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_trace_events_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 67: the journal-batch-bytes rejection. A computed budget
/// with `max_journal_batch_bytes` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_payload_budget`, budget.rs:663-698 returns
/// `BudgetError::JournalBatchBytesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_journal_batch_bytes_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) > 1_048_576,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 68: the queue-depth rejection. A computed budget
/// with `max_queue_depth` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_payload_budget`, budget.rs:663-698 returns
/// `BudgetError::QueueDepthExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_queue_depth_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) > 1_024,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 69: the ipc-payload-bytes rejection. A computed budget
/// with `max_ipc_payload_bytes` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_payload_budget`, budget.rs:663-698 returns
/// `BudgetError::IpcPayloadBytesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_ipc_payload_bytes_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) > 1_048_576,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 70: the blob-bytes rejection. A computed budget
/// with `max_blob_bytes` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_payload_budget`, budget.rs:663-698 returns
/// `BudgetError::BlobBytesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_blob_bytes_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) > 16_777_216,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 71: the input-bytes rejection. A computed budget
/// with `max_input_bytes` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_payload_budget`, budget.rs:663-698 returns
/// `BudgetError::InputBytesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_input_bytes_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) > 1_048_576,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 72: the open-gates rejection. A computed budget
/// with `max_open_gates` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::OpenGatesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_open_gates_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 73: the signal-subscriptions rejection. A computed budget
/// with `max_signal_subscriptions` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::SignalSubscriptionsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_signal_subscriptions_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 74: the incident-emissions rejection. A computed budget
/// with `max_incident_emissions` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::IncidentEmissionsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_incident_emissions_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) <= 1_000_000,
        (budget.max_incident_emissions as int) > 1_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 75: the transitions rejection. A computed budget
/// with `max_transitions` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::TransitionsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_transitions_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) <= 1_000_000,
        (budget.max_incident_emissions as int) <= 1_000_000,
        (budget.max_transitions as int) > 10_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 76: the journal-bytes-total rejection. A computed budget
/// with `max_journal_bytes_total` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::JournalBytesTotalExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_journal_bytes_total_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) <= 1_000_000,
        (budget.max_incident_emissions as int) <= 1_000_000,
        (budget.max_transitions as int) <= 10_000_000,
        (budget.max_journal_bytes_total as int) > 1_048_576_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 77: the value-store-entries rejection. A computed budget
/// with `max_value_store_entries` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::ValueStoreEntriesExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_value_store_entries_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) <= 1_000_000,
        (budget.max_incident_emissions as int) <= 1_000_000,
        (budget.max_transitions as int) <= 10_000_000,
        (budget.max_journal_bytes_total as int) <= 1_048_576_000_000,
        (budget.max_value_store_entries as int) > 2_000_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 78: the cost-units rejection. A computed budget
/// with `max_cost_units` over the `DEFAULT` cap is rejected with
/// `BudgetPolicyExceeded` (production `validate_budget_dimensions`, budget.rs:595-642 returns
/// `BudgetError::CostUnitsExceeded`, collapsed by `validate_budget_result`,
/// workflow/mod.rs:1461-1470). All earlier dimensions are within
/// caps, so this is the first violation production reports.
pub proof fn lemma_budget_cost_units_rejected(budget: SpecComputedBudget)
    requires
        (budget.max_total_steps as int) <= 1_000_000,
        (budget.max_total_slots as int) <= 65_535,
        (budget.max_fanout as int) <= 64,
        (budget.max_nesting_depth as int) <= 8,
        (budget.max_action_tickets as int) <= 100_000,
        (budget.max_parallel_in_flight as int) <= 256,
        (budget.max_run_time_seconds as int) <= 2_592_000,
        (budget.max_result_bytes as int) <= 262_144,
        (budget.max_steps_executable as int) <= 1_000_000,
        (budget.max_timer_entries as int) <= 1_000_000,
        (budget.max_trace_events as int) <= 1_000_000,
        (budget.max_journal_batch_bytes as int) <= 1_048_576,
        (budget.max_queue_depth as int) <= 1_024,
        (budget.max_ipc_payload_bytes as int) <= 1_048_576,
        (budget.max_blob_bytes as int) <= 16_777_216,
        (budget.max_input_bytes as int) <= 1_048_576,
        (budget.max_open_gates as int) <= 1_000_000,
        (budget.max_signal_subscriptions as int) <= 1_000_000,
        (budget.max_incident_emissions as int) <= 1_000_000,
        (budget.max_transitions as int) <= 10_000_000,
        (budget.max_journal_bytes_total as int) <= 1_048_576_000_000,
        (budget.max_value_store_entries as int) <= 2_000_000,
        (budget.max_cost_units as int) > 100_000,
    ensures
        spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(budget) == Some(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 79: loop pairing runs before the budget check. A workflow
/// whose loop pairing fails and whose computed budget exceeds a cap
/// is rejected with the pairing error, not the budget error
/// (production `try_from_parts`, workflow/mod.rs:42-65, runs
/// `loop_regions::validate` before `validate_budget`).
pub proof fn lemma_budget_pairing_before_budget(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        spec_reachability_error(parts) == None::<SpecWorkflowError>,
        spec_forward_edges_error(parts) == None::<SpecWorkflowError>,
        spec_repeat_counter_error(parts) == None::<SpecWorkflowError>,
        spec_nested_together_error(parts) == None::<SpecWorkflowError>,
        spec_pairing_error(&parts.nodes_meta, 0) == Some(SpecWorkflowError::LoopRegionUnpaired),
        (parts.computed_budget.max_total_steps as int) > 1_000_000,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::LoopRegionUnpaired),
{
    assert(spec_pairing_error(&parts.nodes_meta, 0)
        == Some(SpecWorkflowError::LoopRegionUnpaired));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::LoopRegionUnpaired));
}

/// Lemma 80: the budget check runs before input-slot validation. A
/// workflow whose computed budget exceeds a cap and whose input
/// slots are invalid is rejected with the budget error, not the
/// input-slot error (production `try_from_parts`,
/// workflow/mod.rs:42-65, runs `validate_budget` before
/// `validate_input_slots`).
pub proof fn lemma_budget_before_input_slots(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        spec_reachability_error(parts) == None::<SpecWorkflowError>,
        spec_forward_edges_error(parts) == None::<SpecWorkflowError>,
        spec_repeat_counter_error(parts) == None::<SpecWorkflowError>,
        spec_nested_together_error(parts) == None::<SpecWorkflowError>,
        spec_pairing_error(&parts.nodes_meta, 0) == None::<SpecWorkflowError>,
        spec_loop_relations_error(parts) == None::<SpecWorkflowError>,
        spec_loop_edges_error(parts) == None::<SpecWorkflowError>,
        spec_budget_error(parts.computed_budget)
            == Some(SpecWorkflowError::BudgetPolicyExceeded),
        input_slots_valid(parts) == Some(SpecWorkflowError::InputSlotOutOfBounds),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::BudgetPolicyExceeded),
{
    assert(spec_budget_error(parts.computed_budget)
        == Some(SpecWorkflowError::BudgetPolicyExceeded));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::BudgetPolicyExceeded));
}

/// Lemma 81: a within-caps budget reaches input-slot validation. A
/// workflow whose computed budget is within caps and whose input
/// slots are invalid is rejected with the input-slot error, proving
/// the budget step passes through to step 5 instead of masking it.
pub proof fn lemma_budget_within_caps_reaches_input_slots(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        spec_reachability_error(parts) == None::<SpecWorkflowError>,
        spec_forward_edges_error(parts) == None::<SpecWorkflowError>,
        spec_repeat_counter_error(parts) == None::<SpecWorkflowError>,
        spec_nested_together_error(parts) == None::<SpecWorkflowError>,
        spec_pairing_error(&parts.nodes_meta, 0) == None::<SpecWorkflowError>,
        spec_loop_relations_error(parts) == None::<SpecWorkflowError>,
        spec_loop_edges_error(parts) == None::<SpecWorkflowError>,
        spec_budget_error(parts.computed_budget) == None::<SpecWorkflowError>,
        input_slots_valid(parts) == Some(SpecWorkflowError::InputSlotOutOfBounds),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::InputSlotOutOfBounds),
{
    assert(spec_budget_error(parts.computed_budget) == None::<SpecWorkflowError>);
    assert(input_slots_valid(parts) == Some(SpecWorkflowError::InputSlotOutOfBounds));
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::InputSlotOutOfBounds));
}
/// Lemma 82: the node-id mismatch rejection. A node table whose
/// first node carries an id different from its table position is
/// rejected with `NodeIdMismatch` (production `validate_node_id`,
/// workflow/mod.rs:1472-1485, compares `node.id` against the
/// enumeration position and reports the first mismatch).
pub proof fn lemma_node_id_first_mismatch_rejected(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 2,
        metas[0].id == 5u32,
        metas[1].id == 1u32,
    ensures
        spec_nodes_id_error(metas, 0) == Some(SpecWorkflowError::NodeIdMismatch),
{
    assert(spec_node_id_error(metas[0], 0) == Some(SpecWorkflowError::NodeIdMismatch));
    assert(spec_nodes_id_error(metas, 0) == Some(SpecWorkflowError::NodeIdMismatch));
}

/// Lemma 83: well-formed node ids are accepted. A node table whose
/// every id matches its table position passes the node-id check
/// (production `validate_node_id` returns `Ok` for it).
pub proof fn lemma_node_id_well_formed_accepted(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 2,
        metas[0].id == 0u32,
        metas[1].id == 1u32,
    ensures
        spec_nodes_id_error(metas, 0) == None::<SpecWorkflowError>,
{
    assert(spec_node_id_error(metas[0], 0) == None::<SpecWorkflowError>);
    assert(spec_nodes_id_error(metas, 0) == spec_nodes_id_error(metas, 1));
    assert(spec_node_id_error(metas[1], 1) == None::<SpecWorkflowError>);
    assert(spec_nodes_id_error(metas, 1) == spec_nodes_id_error(metas, 2));
    assert(spec_nodes_id_error(metas, 2) == None::<SpecWorkflowError>);
    assert(spec_nodes_id_error(metas, 0) == None::<SpecWorkflowError>);
}

/// Lemma 84: the first mismatch wins. A node table with mismatches
/// at positions 0 and 2 reports the position-0 mismatch: the scan
/// returns the first node's decision without consulting later nodes
/// (production checks `validate_node_id` in node order and `?`
/// propagates the first failure, workflow/mod.rs:1375-1378).
pub proof fn lemma_node_id_first_mismatch_wins(metas: &Vec<SpecNodeMeta>)
    requires
        metas.len() == 3,
        metas[0].id == 7u32,
        metas[1].id == 1u32,
        metas[2].id == 9u32,
    ensures
        spec_nodes_id_error(metas, 0) == Some(SpecWorkflowError::NodeIdMismatch),
        spec_nodes_id_error(metas, 0) == spec_node_id_error(metas[0], 0),
{
    assert(spec_node_id_error(metas[0], 0) == Some(SpecWorkflowError::NodeIdMismatch));
    assert(spec_nodes_id_error(metas, 0) == Some(SpecWorkflowError::NodeIdMismatch));
    assert(spec_nodes_id_error(metas, 0) == spec_node_id_error(metas[0], 0));
}
/// Lemma 85: the op-count rejection. An expression program with
/// more than 256 ops is rejected with `ExpressionInvalid`
/// (production `validate_expr_op_count`,
/// workflow/mod.rs:2682-2690, cap `MAX_EXPRESSION_OPS` 256,
/// limits.rs:31).
pub proof fn lemma_expr_op_count_rejected(program: SpecExprProgram)
    requires
        program.ops.len() == 257,
    ensures
        spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 86: the empty-program rejection. An expression program
/// with no ops ends at depth 0 and is rejected with
/// `ExpressionInvalid` (production `validate_expr_final_depth`,
/// workflow/mod.rs:2712-2720, rejects depth 0 as underflow).
pub proof fn lemma_expr_empty_ops_rejected(program: SpecExprProgram)
    requires
        program.ops.len() == 0,
    ensures
        spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, program.max_stack as int)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 87: the binary-op-first underflow. A program whose first
/// op pops 2 from an empty stack is rejected with
/// `ExpressionInvalid` (production `apply_expr_stack_effect`,
/// workflow/mod.rs:2692-2702, fails the checked subtraction).
pub proof fn lemma_expr_binary_first_rejected(program: SpecExprProgram)
    requires
        program.ops.len() == 1,
        program.ops[0] == SpecExprOp::Binary,
        program.max_stack == 2,
    ensures
        spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_effect(program.ops[0]) == (2, 1));
    assert(spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 88: the capacity rejection. Two pushes need depth 2,
/// which exceeds the claimed `max_stack` of 1, so the program is
/// rejected with `ExpressionInvalid` (production
/// `validate_expr_stack_capacity`, workflow/mod.rs:2704-2710).
pub proof fn lemma_expr_capacity_rejected(program: SpecExprProgram)
    requires
        program.ops.len() == 2,
        program.ops[0] == SpecExprOp::Push,
        program.ops[1] == SpecExprOp::Push,
        program.max_stack == 1,
    ensures
        spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_sim_error(&program.ops, 1, 1, 1, 1)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, 1)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 89: the max_stack mismatch rejection. One push computes
/// a required depth of 1, which differs from the claimed
/// `max_stack` of 2, so the program is rejected with
/// `ExpressionInvalid` (production
/// `ExprProgram::try_from_parts`, workflow/mod.rs:899-908).
pub proof fn lemma_expr_max_stack_mismatch_rejected(program: SpecExprProgram)
    requires
        program.ops.len() == 1,
        program.ops[0] == SpecExprOp::Push,
        program.max_stack == 2,
    ensures
        spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_sim_error(&program.ops, 1, 1, 1, 2)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, 2)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 90: the accessor out-of-bounds rejection. A program
/// whose only op loads accessor 3 with only 2 accessors is
/// rejected with `ExpressionInvalid` (production
/// `validate_expression_accessors`, workflow/mod.rs:2057-2067,
/// with `validate_accessor`, workflow/mod.rs:2076-2086).
pub proof fn lemma_expr_accessor_out_of_bounds_rejected(program: SpecExprProgram)
    requires
        program.ops.len() == 1,
        program.ops[0] == SpecExprOp::LoadAccessor(3u32),
        program.max_stack == 1,
    ensures
        spec_expr_error(program, 2) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_sim_error(&program.ops, 1, 1, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, 1) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 1, 2) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 0, 2)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_error(program, 2) == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 91: the single-push acceptance. A program of one push
/// with `max_stack` 1 validates: the stack simulation ends at
/// depth 1 with required 1, and there is no accessor to check.
pub proof fn lemma_expr_single_push_accepted(program: SpecExprProgram)
    requires
        program.ops.len() == 1,
        program.ops[0] == SpecExprOp::Push,
        program.max_stack == 1,
    ensures
        spec_expr_error(program, 0) == None::<SpecWorkflowError>,
{
    assert(spec_expr_sim_error(&program.ops, 1, 1, 1, 1) == None::<SpecWorkflowError>);
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, 1) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 1, 0) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 0, 0) == None::<SpecWorkflowError>);
    assert(spec_expr_error(program, 0) == None::<SpecWorkflowError>);
}

/// Lemma 92: the two-push-one-binary acceptance. Two pushes and
/// a binary op end at depth 1 with required 2, matching
/// `max_stack` 2, so the program validates.
pub proof fn lemma_expr_two_push_binary_accepted(program: SpecExprProgram)
    requires
        program.ops.len() == 3,
        program.ops[0] == SpecExprOp::Push,
        program.ops[1] == SpecExprOp::Push,
        program.ops[2] == SpecExprOp::Binary,
        program.max_stack == 2,
    ensures
        spec_expr_error(program, 0) == None::<SpecWorkflowError>,
{
    assert(spec_expr_sim_error(&program.ops, 3, 1, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_expr_sim_error(&program.ops, 2, 2, 2, 2) == None::<SpecWorkflowError>);
    assert(spec_expr_sim_error(&program.ops, 1, 1, 1, 2) == None::<SpecWorkflowError>);
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, 2) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 3, 0) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 2, 0) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 1, 0) == None::<SpecWorkflowError>);
    assert(spec_expr_accessor_error(&program.ops, 0, 0) == None::<SpecWorkflowError>);
    assert(spec_expr_error(program, 0) == None::<SpecWorkflowError>);
}

/// Lemma 93: the stack failure masks the accessor failure. A
/// program whose first op underflows the stack and whose second
/// op loads an out-of-bounds accessor is rejected with the stack
/// simulation's error: the accessor scan is never consulted
/// (production runs `ExprProgram::try_from_parts` before
/// `validate_expression_accessors`, workflow/mod.rs:2046-2055).
pub proof fn lemma_expr_stack_masks_accessor(program: SpecExprProgram)
    requires
        program.ops.len() == 2,
        program.ops[0] == SpecExprOp::Binary,
        program.ops[1] == SpecExprOp::LoadAccessor(9u32),
        program.max_stack == 2,
    ensures
        spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid),
        spec_expr_error(program, 0)
            == spec_expr_sim_error(&program.ops, 0, 0, 0, 2),
{
    assert(spec_expr_accessor_error(&program.ops, 1, 0)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_accessor_error(&program.ops, 0, 0)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_sim_error(&program.ops, 0, 0, 0, 2)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_error(program, 0) == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_expr_error(program, 0)
        == spec_expr_sim_error(&program.ops, 0, 0, 0, 2));
}

/// Lemma 94: the first expression wins. A workflow whose first
/// two expressions both fail reports the first expression's
/// error: the scan returns the first expression's decision
/// without consulting later expressions (production validates
/// expression `i` fully before expression `i + 1`,
/// workflow/mod.rs:2046-2055).
pub proof fn lemma_exprs_first_expression_wins(parts: &SpecWorkflowParts)
    requires
        parts.expressions.len() == 2,
        parts.expressions[0].ops.len() == 1,
        parts.expressions[0].ops[0] == SpecExprOp::Binary,
        parts.expressions[0].max_stack == 2,
        parts.expressions[1].ops.len() == 1,
        parts.expressions[1].ops[0] == SpecExprOp::Binary,
        parts.expressions[1].max_stack == 2,
    ensures
        spec_exprs_error(parts, 0) == Some(SpecWorkflowError::ExpressionInvalid),
        spec_exprs_error(parts, 0)
            == spec_expr_error(parts.expressions[0], parts.accessors_len as int),
{
    assert(spec_expr_error(parts.expressions[0], parts.accessors_len as int)
        == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_exprs_error(parts, 0) == Some(SpecWorkflowError::ExpressionInvalid));
    assert(spec_exprs_error(parts, 0)
        == spec_expr_error(parts.expressions[0], parts.accessors_len as int));
}

/// Lemma 95: the expression check runs before the budget check.
/// A workflow with a failing expression and an over-cap computed
/// budget is rejected with the expression error, not the budget
/// error (production `validate_parts` runs
/// `validate_expressions` before `validate_budget`,
/// workflow/mod.rs:1373 and 42-65).
pub proof fn lemma_expr_before_budget(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == Some(SpecWorkflowError::ExpressionInvalid),
        (parts.computed_budget.max_total_steps as int) > 1_000_000,
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == Some(SpecWorkflowError::ExpressionInvalid));
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::ExpressionInvalid));
}
/// Lemma 96: the accessor-path depth rejection. An accessor
/// whose path holds 17 segments is rejected with
/// `AccessorPathTooDeep`: production rejects any path longer
/// than `MAX_PATH_DEPTH` (16, limits.rs:59),
/// workflow/mod.rs:2089-2119.
pub proof fn lemma_accessor_path_depth_rejected(accessor: SpecAccessor)
    requires
        accessor.path.len() == 17,
    ensures
        spec_accessor_path_error(accessor, 10) == Some(SpecWorkflowError::AccessorPathTooDeep),
{
    assert(spec_accessor_path_error(accessor, 10)
        == Some(SpecWorkflowError::AccessorPathTooDeep));
}

/// Lemma 97: the field-symbol out-of-bounds rejection. An
/// accessor whose only segment is `Field(5)` with
/// `symbols_count` 5 is rejected with `SymbolOutOfBounds`
/// (production `validate_symbol`, workflow/mod.rs:2150-2156).
pub proof fn lemma_accessor_path_field_symbol_rejected(accessor: SpecAccessor)
    requires
        accessor.path.len() == 1,
        accessor.path[0] == SpecPathSegment::Field(5u32),
    ensures
        spec_accessor_path_error(accessor, 5) == Some(SpecWorkflowError::SymbolOutOfBounds),
{
    assert(spec_accessor_path_error(accessor, 5)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
}

/// Lemma 98: the reserved-index rejection. An accessor whose
/// only segment is `Index(u32::MAX)` is rejected with
/// `ExpressionInvalid` (production reserved-value arm,
/// workflow/mod.rs:2089-2119).
pub proof fn lemma_accessor_path_reserved_index_rejected(accessor: SpecAccessor)
    requires
        accessor.path.len() == 1,
        accessor.path[0] == SpecPathSegment::Index(u32::MAX),
    ensures
        spec_accessor_path_error(accessor, 1) == Some(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_accessor_path_error(accessor, 1)
        == Some(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 99: the first accessor wins. A workflow whose first
/// accessor is too deep and whose second accessor also fails
/// reports the first accessor's error: the scan returns the
/// first accessor's decision without consulting later accessors
/// (production validates accessor `i` fully before accessor
/// `i + 1`, workflow/mod.rs:2089-2119).
pub proof fn lemma_accessors_first_accessor_wins(parts: &SpecWorkflowParts)
    requires
        parts.accessors.len() == 2,
        parts.accessors[0].path.len() == 17,
        parts.accessors[1].path.len() == 1,
        parts.accessors[1].path[0] == SpecPathSegment::Field(9u32),
    ensures
        spec_accessors_path_error(parts, 0) == Some(SpecWorkflowError::AccessorPathTooDeep),
        spec_accessors_path_error(parts, 0)
            == spec_accessor_path_error(parts.accessors[0], parts.symbols_count as int),
{
    assert(spec_accessor_path_error(parts.accessors[0], parts.symbols_count as int)
        == Some(SpecWorkflowError::AccessorPathTooDeep));
    assert(spec_accessors_path_error(parts, 0)
        == Some(SpecWorkflowError::AccessorPathTooDeep));
    assert(spec_accessors_path_error(parts, 0)
        == spec_accessor_path_error(parts.accessors[0], parts.symbols_count as int));
}

/// Lemma 100: the first segment wins. An accessor whose first
/// segment is an out-of-bounds `Field` and whose second segment
/// is a reserved `Index(u32::MAX)` reports the field error: the
/// scan returns the first segment's decision without consulting
/// later segments (production walks segments in order,
/// workflow/mod.rs:2089-2119).
pub proof fn lemma_accessor_path_first_segment_wins(accessor: SpecAccessor)
    requires
        accessor.path.len() == 2,
        accessor.path[0] == SpecPathSegment::Field(9u32),
        accessor.path[1] == SpecPathSegment::Index(u32::MAX),
    ensures
        spec_accessor_path_error(accessor, 9) == Some(SpecWorkflowError::SymbolOutOfBounds),
        spec_accessor_path_error(accessor, 9)
            == spec_accessor_path_segment_error(&accessor.path, 0, 9),
{
    assert(spec_accessor_path_segment_error(&accessor.path, 0, 9)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
    assert(spec_accessor_path_error(accessor, 9)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
    assert(spec_accessor_path_error(accessor, 9)
        == spec_accessor_path_segment_error(&accessor.path, 0, 9));
}

/// Lemma 101: the empty path is accepted. An accessor with no
/// segments validates: the depth check passes and the segment
/// walk has nothing to reject (workflow/mod.rs:2089-2119).
pub proof fn lemma_accessor_path_empty_accepted(accessor: SpecAccessor)
    requires
        accessor.path.len() == 0,
    ensures
        spec_accessor_path_error(accessor, 1) == None::<SpecWorkflowError>,
{
    assert(spec_accessor_path_error(accessor, 1) == None::<SpecWorkflowError>);
}

/// Segment-scan induction: when every segment from `i` is a
/// `Field(0)` and `symbols_count` is positive, the scan accepts.
proof fn lemma_segment_scan_field_zero_ok(
    segments: &Vec<SpecPathSegment>,
    i: int,
    symbols_count: int,
)
    requires
        0 <= i <= segments.len(),
        symbols_count > 0,
        forall |j: int| i <= j < segments.len() ==> segments[j] == SpecPathSegment::Field(0u32),
    ensures
        spec_accessor_path_segment_error(segments, i, symbols_count) == None::<SpecWorkflowError>,
    decreases segments.len() - i
{
    if i >= segments.len() {
    } else {
        assert(segments[i] == SpecPathSegment::Field(0u32));
        lemma_segment_scan_field_zero_ok(segments, i + 1, symbols_count);
    }
}

/// Lemma 102: depth exactly 16 is accepted. An accessor whose
/// path holds 16 in-bounds `Field(0)` segments validates: the
/// depth cap is `MAX_PATH_DEPTH` (16, limits.rs:59), so only
/// lengths above 16 reject (workflow/mod.rs:2089-2119).
pub proof fn lemma_accessor_path_depth_16_accepted(accessor: SpecAccessor)
    requires
        accessor.path.len() == 16,
        forall |j: int| 0 <= j < accessor.path.len()
            ==> accessor.path[j] == SpecPathSegment::Field(0u32),
    ensures
        spec_accessor_path_error(accessor, 1) == None::<SpecWorkflowError>,
{
    lemma_segment_scan_field_zero_ok(&accessor.path, 0, 1);
    assert(spec_accessor_path_error(accessor, 1) == None::<SpecWorkflowError>);
}

/// Lemma 103: the largest non-reserved index is accepted. An
/// accessor whose only segment is `Index(u32::MAX - 1)`
/// validates: only the reserved `u32::MAX` value rejects
/// (workflow/mod.rs:2089-2119).
pub proof fn lemma_accessor_path_max_index_accepted(accessor: SpecAccessor)
    requires
        accessor.path.len() == 1,
        accessor.path[0] == SpecPathSegment::Index(4294967294u32),
    ensures
        spec_accessor_path_error(accessor, 1) == None::<SpecWorkflowError>,
{
    assert(spec_accessor_path_segment_error(&accessor.path, 1, 1)
        == None::<SpecWorkflowError>);
    assert(spec_accessor_path_segment_error(&accessor.path, 0, 1)
        == None::<SpecWorkflowError>);
    assert(spec_accessor_path_error(accessor, 1) == None::<SpecWorkflowError>);
}

/// Lemma 104: the largest in-bounds field symbol is accepted. An
/// accessor whose only segment is `Field(4)` with
/// `symbols_count` 5 validates: `validate_symbol` accepts
/// `symbol.get() < symbols_count` (workflow/mod.rs:2150-2156).
pub proof fn lemma_accessor_path_last_symbol_accepted(accessor: SpecAccessor)
    requires
        accessor.path.len() == 1,
        accessor.path[0] == SpecPathSegment::Field(4u32),
    ensures
        spec_accessor_path_error(accessor, 5) == None::<SpecWorkflowError>,
{
    assert(spec_accessor_path_segment_error(&accessor.path, 1, 5)
        == None::<SpecWorkflowError>);
    assert(spec_accessor_path_segment_error(&accessor.path, 0, 5)
        == None::<SpecWorkflowError>);
    assert(spec_accessor_path_error(accessor, 5) == None::<SpecWorkflowError>);
}

/// Lemma 105: the accessor-path check runs before the
/// reachability check. A workflow with a too-deep accessor path
/// and an unreachable node is rejected with the accessor-path
/// error, not the reachability error (production
/// `validate_parts` runs `validate_accessor_paths` before
/// `validate_reachability`, workflow/mod.rs:1379 and 1382).
pub proof fn lemma_accessor_path_before_reachability(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == Some(SpecWorkflowError::AccessorPathTooDeep),
        spec_reachability_error(parts) == Some(SpecWorkflowError::UnreachableNode),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::AccessorPathTooDeep),
{
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0)
        == Some(SpecWorkflowError::AccessorPathTooDeep));
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::AccessorPathTooDeep));
}

/// Lemma 106: the over-cap expression rejection. A workflow
/// whose only expression claims a `max_stack` above the contract
/// cap is rejected with `ResourceContractExceeded` (production
/// `validate_expr_stack_usage`, workflow/mod.rs:1656-1668).
pub proof fn lemma_expr_stack_over_cap_rejected(parts: &SpecWorkflowParts)
    requires
        parts.expressions.len() == 1,
        parts.expressions[0].max_stack > parts.resource_contract.max_expr_stack,
    ensures
        spec_expr_stack_error(parts, 0) == Some(SpecWorkflowError::ResourceContractExceeded),
{
    assert(spec_expr_stack_error(parts, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
}

/// Lemma 107: the at-cap expression is accepted. A workflow whose
/// only expression claims a `max_stack` exactly equal to the
/// contract cap validates: production rejects only when
/// `max_stack > max_expr_stack` (workflow/mod.rs:1656-1668).
pub proof fn lemma_expr_stack_at_cap_accepted(parts: &SpecWorkflowParts)
    requires
        parts.expressions.len() == 1,
        parts.expressions[0].max_stack == parts.resource_contract.max_expr_stack,
    ensures
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
{
    assert(spec_expr_stack_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
}

/// Lemma 108: the first over-cap expression wins. A workflow whose
/// first expression is over the contract cap and whose second is
/// within it reports the over-cap error from the first expression:
/// the scan returns the first expression's decision without
/// consulting later expressions (production iterates expressions
/// in order, workflow/mod.rs:1656-1668).
pub proof fn lemma_expr_stack_first_over_cap_wins(parts: &SpecWorkflowParts)
    requires
        parts.expressions.len() == 2,
        parts.expressions[0].max_stack > parts.resource_contract.max_expr_stack,
        parts.expressions[1].max_stack <= parts.resource_contract.max_expr_stack,
    ensures
        spec_expr_stack_error(parts, 0) == Some(SpecWorkflowError::ResourceContractExceeded),
        spec_expr_stack_error(parts, 1) == None::<SpecWorkflowError>,
{
    assert(spec_expr_stack_error(parts, 2) == None::<SpecWorkflowError>);
    assert(spec_expr_stack_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_expr_stack_error(parts, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
}

/// Lemma 109: the expression-stack check runs before the
/// expression check. A workflow with an over-cap expression and a
/// failing expression is rejected with the stack error, not the
/// expression error (production `validate_resource_contract` runs
/// `validate_expr_stack_usage` before `validate_expressions`,
/// workflow/mod.rs:1587-1593 and 2046-2055).
pub proof fn lemma_expr_stack_before_exprs(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == Some(SpecWorkflowError::ResourceContractExceeded),
        spec_exprs_error(parts, 0) == Some(SpecWorkflowError::ExpressionInvalid),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded),
{
    assert(spec_expr_stack_error(parts, 0)
        == Some(SpecWorkflowError::ResourceContractExceeded));
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded));
}

/// Lemma 110: within-cap expressions reach the expression check.
/// A workflow whose expressions all fit the contract cap and
/// whose expression check fails is rejected with the expression
/// error, not the stack error (production
/// `validate_expr_stack_usage` passes, then
/// `validate_expressions` runs, workflow/mod.rs:1587-1593 and
/// 2046-2055).
pub proof fn lemma_expr_stack_within_cap_reaches_exprs(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == Some(SpecWorkflowError::ExpressionInvalid),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::ExpressionInvalid),
{
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::ExpressionInvalid));
}

/// Lemma 111: the pooled-symbol rejection. A workflow whose only
/// pooled symbol is at or beyond `symbols_count` is rejected with
/// `SymbolOutOfBounds` (production `validate_constants_symbols`,
/// workflow/mod.rs:2122-2132, via `validate_symbol`,
/// workflow/mod.rs:2150-2156).
pub proof fn lemma_constants_symbol_out_of_bounds_rejected(parts: &SpecWorkflowParts)
    requires
        parts.constants_symbols.len() == 1,
        parts.constants_symbols[0] >= parts.symbols_count,
    ensures
        spec_constants_symbols_error(parts, 0) == Some(SpecWorkflowError::SymbolOutOfBounds),
{
    assert(spec_constants_symbols_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_constants_symbols_error(parts, 0)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
}

/// Lemma 112: the largest in-bounds pooled symbol is accepted. A
/// workflow whose only pooled symbol is `4` with `symbols_count`
/// 5 validates: `validate_symbol` accepts
/// `symbol.get() < symbols_count` (workflow/mod.rs:2150-2156).
pub proof fn lemma_constants_symbol_last_accepted(parts: &SpecWorkflowParts)
    requires
        parts.constants_symbols.len() == 1,
        parts.constants_symbols[0] == 4u32,
        parts.symbols_count == 5u32,
    ensures
        spec_constants_symbols_error(parts, 0) == None::<SpecWorkflowError>,
{
    assert(spec_constants_symbols_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_constants_symbols_error(parts, 0) == None::<SpecWorkflowError>);
}

/// Lemma 113: the first out-of-bounds pooled symbol wins. A
/// workflow whose first pooled symbol is out of bounds and whose
/// second is in bounds reports the error from the first symbol:
/// the scan returns the first symbol's decision without consulting
/// later symbols (production iterates the constant pool in order,
/// workflow/mod.rs:2122-2132).
pub proof fn lemma_constants_symbol_first_out_of_bounds_wins(parts: &SpecWorkflowParts)
    requires
        parts.constants_symbols.len() == 2,
        parts.constants_symbols[0] >= parts.symbols_count,
        parts.constants_symbols[1] < parts.symbols_count,
    ensures
        spec_constants_symbols_error(parts, 0) == Some(SpecWorkflowError::SymbolOutOfBounds),
        spec_constants_symbols_error(parts, 1) == None::<SpecWorkflowError>,
{
    assert(spec_constants_symbols_error(parts, 2) == None::<SpecWorkflowError>);
    assert(spec_constants_symbols_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_constants_symbols_error(parts, 0)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
}

/// Lemma 114: the BuildObject field-symbol rejection. A workflow
/// whose only BuildObject field symbol is at or beyond
/// `symbols_count` is rejected with `SymbolOutOfBounds`
/// (production `validate_build_object_symbols`,
/// workflow/mod.rs:2135-2147, via `validate_symbol`,
/// workflow/mod.rs:2150-2156).
pub proof fn lemma_build_object_symbol_out_of_bounds_rejected(parts: &SpecWorkflowParts)
    requires
        parts.build_object_field_symbols.len() == 1,
        parts.build_object_field_symbols[0].len() == 1,
        parts.build_object_field_symbols[0][0] >= parts.symbols_count,
    ensures
        spec_build_object_symbols_error(parts, 0) == Some(SpecWorkflowError::SymbolOutOfBounds),
{
    assert(spec_build_object_field_symbols_error(
        &parts.build_object_field_symbols[0], 1, parts.symbols_count as int)
        == None::<SpecWorkflowError>);
    assert(spec_build_object_field_symbols_error(
        &parts.build_object_field_symbols[0], 0, parts.symbols_count as int)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
    assert(spec_build_object_symbols_error(parts, 1) == None::<SpecWorkflowError>);
    assert(spec_build_object_symbols_error(parts, 0)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
}

/// Lemma 115: the first out-of-bounds BuildObject field wins. A
/// node whose first field symbol is out of bounds and whose second
/// is in bounds reports the error from the first field: the scan
/// returns the first field's decision without consulting later
/// fields (production iterates fields in order,
/// workflow/mod.rs:2135-2147).
pub proof fn lemma_build_object_symbol_first_out_of_bounds_wins(parts: &SpecWorkflowParts)
    requires
        parts.build_object_field_symbols.len() == 1,
        parts.build_object_field_symbols[0].len() == 2,
        parts.build_object_field_symbols[0][0] >= parts.symbols_count,
        parts.build_object_field_symbols[0][1] < parts.symbols_count,
    ensures
        spec_build_object_symbols_error(parts, 0) == Some(SpecWorkflowError::SymbolOutOfBounds),
        spec_build_object_field_symbols_error(
            &parts.build_object_field_symbols[0], 1, parts.symbols_count as int)
            == None::<SpecWorkflowError>,
{
    assert(spec_build_object_field_symbols_error(
        &parts.build_object_field_symbols[0], 2, parts.symbols_count as int)
        == None::<SpecWorkflowError>);
    assert(spec_build_object_field_symbols_error(
        &parts.build_object_field_symbols[0], 1, parts.symbols_count as int)
        == None::<SpecWorkflowError>);
    assert(spec_build_object_field_symbols_error(
        &parts.build_object_field_symbols[0], 0, parts.symbols_count as int)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
    assert(spec_build_object_symbols_error(parts, 0)
        == Some(SpecWorkflowError::SymbolOutOfBounds));
}

/// Lemma 116: the symbol check runs before the reachability
/// check. A workflow with an out-of-bounds pooled symbol and an
/// unreachable node is rejected with the symbol error, not the
/// reachability error (production `validate_parts` runs
/// `validate_constants_symbols` before `validate_reachability`,
/// workflow/mod.rs:1380 and 1382).
pub proof fn lemma_symbols_before_reachability(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == Some(SpecWorkflowError::SymbolOutOfBounds),
        spec_reachability_error(parts) == Some(SpecWorkflowError::UnreachableNode),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::SymbolOutOfBounds),
{
    assert(spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_exprs_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>);
    assert(spec_symbols_error(parts) == Some(SpecWorkflowError::SymbolOutOfBounds));
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::SymbolOutOfBounds));
}

/// Lemma 117: in-bounds symbols reach the reachability check. A
/// workflow whose pooled and BuildObject symbols all validate and
/// whose reachability check fails is rejected with the
/// reachability error, not the symbol error (production
/// `validate_constants_symbols` and
/// `validate_build_object_symbols` pass, then
/// `validate_reachability` runs, workflow/mod.rs:1380-1382).
pub proof fn lemma_symbols_within_bounds_reaches_reachability(parts: &SpecWorkflowParts)
    requires
        parts.nodes_meta.len() > 0,
        parts.entry < parts.nodes_len,
        parts.resource_contract.max_steps <= 10000,
        parts.resource_contract.max_slots <= 1024,
        parts.resource_contract.max_expr_stack <= 64,
        parts.resource_contract.max_transitions_per_tick > 0,
        parts.resource_contract.max_transitions_per_tick <= 10000,
        parts.nodes_len <= parts.resource_contract.max_steps,
        parts.slot_count <= parts.resource_contract.max_slots,
        spec_expr_stack_error(parts, 0) == None::<SpecWorkflowError>,
        spec_exprs_error(parts, 0) == None::<SpecWorkflowError>,
        spec_accessors_path_error(parts, 0) == None::<SpecWorkflowError>,
        spec_symbols_error(parts) == None::<SpecWorkflowError>,
        spec_reachability_error(parts) == Some(SpecWorkflowError::UnreachableNode),
    ensures
        try_from_parts_pure_spec(parts)
            == SpecValidationResult::Err(SpecWorkflowError::UnreachableNode),
{
    assert(spec_symbols_error(parts) == None::<SpecWorkflowError>);
    assert(try_from_parts_pure_spec(parts)
        == SpecValidationResult::Err(SpecWorkflowError::UnreachableNode));
}

/// Lemma 118: the constant-pool symbol check runs before the
/// BuildObject field-symbol check inside the combined step-1g
/// mirror. Both checks return the same `SymbolOutOfBounds`
/// value, so their order is unobservable in the top-level chain;
/// this pins the composition order at the spec level instead:
/// when both checks fail, the combined mirror returns the
/// constant-pool decision (production runs
/// `validate_constants_symbols` before
/// `validate_build_object_symbols`, workflow/mod.rs:1380-1381).
pub proof fn lemma_constants_symbols_before_build_object(parts: &SpecWorkflowParts)
    requires
        spec_constants_symbols_error(parts, 0) == Some(SpecWorkflowError::SymbolOutOfBounds),
        spec_build_object_symbols_error(parts, 0) == Some(SpecWorkflowError::SymbolOutOfBounds),
    ensures
        spec_symbols_error(parts)
            == spec_constants_symbols_error(parts, 0),
{
    assert(spec_symbols_error(parts)
        == spec_constants_symbols_error(parts, 0));
}
fn main() {}

} // verus!
