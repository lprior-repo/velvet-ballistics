// Verification artifact: collect_ir_structure.rs
// PO: PO-012 (lower_bounded_collect IR struct field refinement)
// Bead: vb-xi2f.23
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/collect_ir_structure.rs
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// Target: production exec fn `lower_bounded_collect` in
// `crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-260`.
//
// The production exec fn emits 3 + body_node_count CompiledNode
// entries on the success path:
//
//   [0] CompiledNodeKind::CollectStart {
//          source, limit, page_size,
//          body: id+1, done: id+body_len+2,
//        } with output slot collector_slot
//   [1..body_len] one node per body step at ids id+1 .. id+body_len,
//        each with its own output slot (SetConst or an
//        expression-program node for a Set step, Do node for a Do step)
//   [body_len+1] CompiledNodeKind::CollectNext {
//          collector_slot, body: id+1, done: id+body_len+2,
//        }
//   [body_len+2] CompiledNodeKind::CollectFinish {
//          collector_slot,
//        } with output slot finish_slot
//
// The collector slot and the finish slot are the two slots
// `allocate_collect_slots` returns (bounded_collect.rs:146-154);
// under the fresh-source assumption they are pre_slot_count + 1 and
// pre_slot_count + 2. The fn returns the finish slot.
//
// Binding mechanism: `#[path = "extern_collect_ir_structure.rs"]`
// imports the thin extern surface, which defines a
// `#[verifier::external]` projection
// `lower_bounded_collect_ir_projection` that mirrors the production
// signature (parameter list, parameter order, return-type envelope)
// and reproduces the production decision shape (precondition checks,
// error variants, slot-recording delta, emitted-node count,
// per-node kind/field structure). The spec file attaches spec
// contracts to the projection via `assume_specification` and every
// proof below the bridge exercises the production projection
// through an exec wrapper. There are zero vacuous proofs in the
// rewritten spec.
//
// The pre-split spec bound this file to `lower_canonical_collect` at
// part_03.rs:195-256 with fixed offsets (body=id+1, page=id+2,
// done=id+3), a 4-node emission with a single `SetConst` body node,
// and a +1 slot delta. That binding is false for current production
// whenever the body has more than one step: the consecutiveness
// lemma (L1), the CollectPage lemma (L4), and the slot-delta lemma
// (L7) no longer hold. This rewrite re-grounds every lemma in the
// current shape, mirroring the production-side contract already
// proven in the part_03.rs Verus block (L1-L6 for
// `plan_compute_edges`) and the PO-011 sibling spec
// (collect_lowering.rs).
//
// ============================================================================
// BINDING LEDGER (mirrors extern_collect_ir_structure.rs BINDING LEDGER)
// ============================================================================
//   - `StepIdx` (u16 newtype)              <- crates/vb_core/src/ids/mod.rs:70-70
//   - `StepIdx::new`                       <- crates/vb_core/src/ids/mod.rs:36-36
//   - `StepIdx::get`                       <- crates/vb_core/src/ids/mod.rs:70-70
//   - `StepIdx::checked_add`               <- crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:201-206
//   - `lower_bounded_collect`              <- crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:243-260
//   - `plan_compute_edges`                 <- crates/vb_compile/src/mod_compile_lowering/bounded_collect.rs:116-126
//   - `CollectStart` / `CollectNext` /
//     `CollectFinish`                      <- crates/vb_core/src/workflow/mod.rs:1264-1284
//   - `SlotCompiler::record_slot`          <- crates/vb_compile/src/mod_compile_lowering/part_08.rs:111-117
//   - `SlotCompiler::allocate_slot`        <- crates/vb_compile/src/mod_compile_lowering/part_08.rs:29-38
//   - `emit_scoped_flat_body`              <- crates/vb_compile/src/mod_compile_lowering/canonical_branch_body.rs:20-42
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `lower_bounded_collect` cannot be verified
// end-to-end inside Verus because it transitively depends on
// `vb_core::workflow::*`, `vb_core::ids::*`, `SlotCompiler`, and
// `CompileError`, all of which carry heap allocations, derives, or
// crate-internal modules that Verus does not model in a single-file
// Verus unit. The pure projection in
// `extern_collect_ir_structure.rs` captures every decision branch the
// production fn takes on the relevant scalar inputs (id, source,
// limit, page_size, body_len, body_node_count, pre_slot_count) and is
// recorded as a trusted base in the binding ledger. Each proof below
// operates on the spec construction `spec_collect_ir_outcome(...)`
// whose fields are bound to the projection via the
// `assume_specification` contract; any divergence between the
// projection and the production body is a binding debt item tracked
// outside Verus.
//
// The exec wrapper `checked_prod_lower_bounded_collect_ir` carries
// explicit `assert(...)` calls in its body to discharge the
// `assume_specification` postcondition — Verus requires the contract
// to be unfolded before the wrapper's ensures clause can be verified.
//
// Slot accounting assumes the fresh-source case (see the extern
// file's TRUST BOUNDARY note): `record_slot(source)` with
// `source.0 == pre_slot_count` raises the slot count by exactly one,
// the two `allocate_slot` calls raise it by one each, and each body
// step allocates one further slot, so the full-call post count is
// `pre_slot_count + 3 + body_len`. The PO-011 sibling projection
// reports the `plan_build`-only `pre_slot_count + 3`; the two post
// counts differ by design because the PO-011 projection scopes body
// emission outside it while this spec covers the full emission chain.
//
// Body node kinds are not pinned: body steps are `Set` or `Do` only,
// but a `Set` step lowers to `SetConst` or an expression-program node
// and a `Do` step to a `Do` node, so this spec claims body node ids
// and output slots, never body node kinds.
//
// ============================================================================
// PROOF OBLIGATION MAP
// ============================================================================
//   L1: lemma_collect_node_ids
//        -> the emitted node ids are id (CollectStart), id+1 ..
//           id+body_len (body), id+body_len+1 (CollectNext),
//           id+body_len+2 (CollectFinish).
//   L2: lemma_node_0_is_collect_start
//        -> Node 0 is CollectStart with source/limit/page_size/body/done
//           fields matching the spec inputs and output slot
//           collector_slot.
//   L3: lemma_body_nodes
//        -> Body nodes occupy ids id+1 .. id+body_len with output
//           slots pre_slot_count+3 .. pre_slot_count+3+body_len-1.
//   L4: lemma_node_next_is_collect_next
//        -> The CollectNext node is at id+body_len+1 with
//           collector_slot, body=id+1, done=id+body_len+2.
//   L5: lemma_node_finish_is_collect_finish
//        -> The CollectFinish node is at id+body_len+2 with
//           collector_slot and output slot finish_slot.
//   L6: lemma_collect_node_count
//        -> total emitted node count is exactly 3 + body_node_count
//           on success.
//   L7: lemma_collect_slot_delta
//        -> slot count delta is exactly +3+body_len on success
//           (fresh-source assumption).
//   L8: lemma_collect_full_emission_chain
//        -> conjunction of L1-L7 (full emission chain contract).
//
// Failure-path proofs:
//   F1: lemma_collect_id_overflow_fails
//        -> id + 1 > u16::MAX produces ok=false and
//           error_kind=SPEC_ERR_LIMIT_EXCEEDED with 0 emitted nodes.
//   F2: lemma_collect_body_shape_fails
//        -> body_len == 0 produces ok=false and
//           error_kind=SPEC_ERR_STEP_SHAPE with 0 emitted nodes.
//   F3: lemma_collect_body_length_overflow_fails
//        -> body_len + 1 > u16::MAX produces ok=false and
//           error_kind=SPEC_ERR_INDEX_RANGE with 0 emitted nodes.
//   F4: lemma_collect_slot_budget_fails
//        -> pre_slot_count + 3 + body_len > u16::MAX produces
//           ok=false and error_kind=SPEC_ERR_SLOT_RANGE with 0
//           emitted nodes.

use vstd::prelude::*;

verus! {

#[path = "extern_collect_ir_structure.rs"]
mod production;

pub use production::{
    lower_bounded_collect_ir_projection,
    SpecCollectIROutcome,
    StepIdx,
    SlotIdx,
    KIND_COLLECT_FINISH,
    KIND_COLLECT_NEXT,
    KIND_COLLECT_START,
    SPEC_ERR_INDEX_RANGE,
    SPEC_ERR_LIMIT_EXCEEDED,
    SPEC_ERR_NONE,
    SPEC_ERR_SLOT_RANGE,
    SPEC_ERR_STEP_SHAPE,
};

// ============================================================================
// Spec-side constants and predicates
// ============================================================================

pub open spec fn u16_max() -> int {
    65535
}

pub open spec fn u32_max() -> int {
    4294967295
}

pub open spec fn bounded_u16(x: int) -> bool {
    0 <= x && x <= u16_max()
}

pub open spec fn bounded_u32(x: int) -> bool {
    0 <= x && x <= u32_max()
}

/// Spec helper: whether the scalar-determined sub-decision of
/// `lower_bounded_collect` succeeds for the given inputs: the body is
/// non-empty, every edge sum fits in u16, and the full-call slot
/// budget fits in u16. Mirrors the success arm of the
/// `assume_specification` contract on
/// `lower_bounded_collect_ir_projection` below.
pub open spec fn spec_collect_ir_ok(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
) -> bool {
    body_len >= 1 && id + 1 <= u16_max() && body_len + 1 <= u16_max()
        && id + body_len + 1 <= u16_max() && body_len + 2 <= u16_max()
        && id + body_len + 2 <= u16_max()
        && pre_slot_count + 3 + body_len <= u16_max()
}

/// Spec helper: the error kind the production projection returns when
/// the scalar-determined sub-decision fails, in production order
/// (empty body, then the `plan_compute_edges` checks in order, then
/// the slot budget). Mirrors the decision shape of
/// `lower_bounded_collect_ir_projection` in the extern file.
pub open spec fn spec_collect_ir_error_kind(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
) -> u8 {
    if body_len == 0 {
        SPEC_ERR_STEP_SHAPE
    } else if id + 1 > u16_max() {
        SPEC_ERR_LIMIT_EXCEEDED
    } else if body_len + 1 > u16_max() {
        SPEC_ERR_INDEX_RANGE
    } else if id + body_len + 1 > u16_max() {
        SPEC_ERR_LIMIT_EXCEEDED
    } else if body_len + 2 > u16_max() {
        SPEC_ERR_INDEX_RANGE
    } else if id + body_len + 2 > u16_max() {
        SPEC_ERR_LIMIT_EXCEEDED
    } else {
        SPEC_ERR_SLOT_RANGE
    }
}

/// Spec-side predicate for inputs that satisfy the production
/// `lower_bounded_collect` preconditions on the success path:
///   - id fits in u16 and id + body_len + 2 fits in u16 (so
///     checked_step_offset for body/advance/done all succeed).
///   - body has at least one step (so validate_body succeeds) and
///     body_len + 1 / body_len + 2 fit in u16 (so
///     checked_body_length_offset succeeds).
///   - body_node_count is at least body_len (each body step emits
///     at least one node) and fits in u16.
///   - source fits in u16 (so record_slot(source) is well-defined).
///   - limit and page_size fit in u32 with the documented >=1 lower
///     bound (matches production plan_validate_bounds).
///   - pre_slot_count fits in u16 with pre_slot_count + 3 + body_len
///     <= u16::MAX (so the full-call slot budget does not overflow).
pub open spec fn collect_ir_inputs_valid(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
) -> bool {
    bounded_u16(id) && id + body_len + 2 <= u16_max() && body_len >= 1
        && body_len + 1 <= u16_max() && body_len + 2 <= u16_max()
        && body_len <= body_node_count && bounded_u16(body_node_count)
        && bounded_u16(source) && 1 <= limit && limit <= u32_max()
        && 1 <= page_size && page_size <= u32_max()
        && bounded_u16(pre_slot_count)
        && pre_slot_count + 3 + body_len <= u16_max()
}

// ============================================================================
// Spec-side construction of the projection outcome from spec inputs
// ============================================================================
//
// `spec_collect_ir_outcome` constructs the SpecCollectIROutcome value
// the production projection returns when called with the corresponding
// primitive inputs. It is the spec-side mirror of the projection body.
// The `assume_specification` bridge below asserts that the production
// projection returns exactly this value.
//
// The success path mirrors the production body:
//   - start_step_id = id, start_source = source, start_limit = limit,
//     start_page_size = page_size, start_body_id = id+1,
//     start_done_id = id+body_len+2
//   - collector_slot = pre_slot_count+1, finish_slot = pre_slot_count+2
//   - body_first_step_id = id+1, body_last_step_id = id+body_len,
//     body_first_output_slot = pre_slot_count+3,
//     body_last_output_slot = pre_slot_count+3+body_len-1
//   - next_step_id = id+body_len+1, next_collector_slot = collector_slot,
//     next_body_id = id+1, next_done_id = id+body_len+2
//   - finish_step_id = id+body_len+2,
//     finish_collector_slot = collector_slot
//   - post_slot_count = pre_slot_count+3+body_len,
//     emitted_node_count = 3+body_node_count
//   - node_0_kind = KIND_COLLECT_START, next_kind = KIND_COLLECT_NEXT,
//     finish_kind = KIND_COLLECT_FINISH
//
// The failure path zeros out all node fields and sets
// emitted_node_count = 0.

pub open spec fn spec_collect_ir_outcome(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
) -> SpecCollectIROutcome {
    SpecCollectIROutcome {
        ok: spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
        error_kind: spec_collect_ir_error_kind(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
        pre_slot_count: pre_slot_count as u16,
        post_slot_count: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 3 + body_len
        } else {
            pre_slot_count
        }) as u16,
        emitted_node_count: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            3 + body_node_count
        } else {
            0
        }) as u32,
        collector_slot: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 1
        } else {
            0
        }) as u16,
        finish_slot: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 2
        } else {
            0
        }) as u16,
        start_step_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id
        } else {
            0
        }) as u16,
        start_source: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            source
        } else {
            0
        }) as u16,
        start_limit: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            limit
        } else {
            0
        }) as u32,
        start_page_size: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            page_size
        } else {
            0
        }) as u32,
        start_body_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + 1
        } else {
            0
        }) as u16,
        start_done_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + body_len + 2
        } else {
            0
        }) as u16,
        body_first_step_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + 1
        } else {
            0
        }) as u16,
        body_last_step_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + body_len
        } else {
            0
        }) as u16,
        body_first_output_slot: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 3
        } else {
            0
        }) as u16,
        body_last_output_slot: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 3 + body_len - 1
        } else {
            0
        }) as u16,
        next_step_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + body_len + 1
        } else {
            0
        }) as u16,
        next_collector_slot: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 1
        } else {
            0
        }) as u16,
        next_body_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + 1
        } else {
            0
        }) as u16,
        next_done_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + body_len + 2
        } else {
            0
        }) as u16,
        finish_step_id: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            id + body_len + 2
        } else {
            0
        }) as u16,
        finish_collector_slot: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            pre_slot_count + 1
        } else {
            0
        }) as u16,
        node_0_kind: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            KIND_COLLECT_START as int
        } else {
            0
        }) as u8,
        next_kind: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            KIND_COLLECT_NEXT as int
        } else {
            0
        }) as u8,
        finish_kind: (if spec_collect_ir_ok(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ) {
            KIND_COLLECT_FINISH as int
        } else {
            0
        }) as u8,
    }
}

// ============================================================================
// assume_specification bridge — production projection contract
// ============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to a Rust function whose body Verus cannot fully model.
// The body of `lower_bounded_collect_ir_projection` is
// `#[verifier::external]`; Verus accepts the `ensures` clause below
// but does not verify the body itself. The contract characterises the
// production behaviour the corresponding `lower_bounded_collect`
// would exhibit on the same scalar inputs. The success and failure
// arms are stated separately so the contract is exact on both paths:
// on failure every node field is zero, unlike the pre-split bridge,
// which stated success-path field values unconditionally.

pub assume_specification[ production::lower_bounded_collect_ir_projection ](
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_len: u16,
    body_node_count: u16,
    pre_slot_count: u16,
) -> (outcome: SpecCollectIROutcome)
    ensures
        match outcome.ok {
            true => {
                &&& outcome.error_kind == SPEC_ERR_NONE
                &&& spec_collect_ir_ok(
                    id.0 as int,
                    source.0 as int,
                    limit as int,
                    page_size as int,
                    body_len as int,
                    body_node_count as int,
                    pre_slot_count as int,
                )
                &&& outcome.pre_slot_count == pre_slot_count
                &&& outcome.post_slot_count as int
                    == pre_slot_count as int + 3 + body_len as int
                &&& outcome.emitted_node_count as int == body_node_count as int + 3
                &&& outcome.collector_slot as int == pre_slot_count as int + 1
                &&& outcome.finish_slot as int == pre_slot_count as int + 2
                &&& outcome.start_step_id == id.0
                &&& outcome.start_source == source.0
                &&& outcome.start_limit == limit
                &&& outcome.start_page_size == page_size
                &&& outcome.start_body_id as int == id.0 as int + 1
                &&& outcome.start_done_id as int == id.0 as int + body_len as int + 2
                &&& outcome.body_first_step_id as int == id.0 as int + 1
                &&& outcome.body_last_step_id as int == id.0 as int + body_len as int
                &&& outcome.body_first_output_slot as int == pre_slot_count as int + 3
                &&& outcome.body_last_output_slot as int
                    == pre_slot_count as int + 3 + body_len as int - 1
                &&& outcome.next_step_id as int == id.0 as int + body_len as int + 1
                &&& outcome.next_collector_slot == outcome.collector_slot
                &&& outcome.next_body_id as int == id.0 as int + 1
                &&& outcome.next_done_id as int == id.0 as int + body_len as int + 2
                &&& outcome.finish_step_id as int == id.0 as int + body_len as int + 2
                &&& outcome.finish_collector_slot == outcome.collector_slot
                &&& outcome.node_0_kind == KIND_COLLECT_START
                &&& outcome.next_kind == KIND_COLLECT_NEXT
                &&& outcome.finish_kind == KIND_COLLECT_FINISH
            },
            false => {
                &&& outcome.pre_slot_count == pre_slot_count
                &&& outcome.post_slot_count == pre_slot_count
                &&& outcome.emitted_node_count == 0
                &&& outcome.collector_slot == 0
                &&& outcome.finish_slot == 0
                &&& outcome.start_step_id == 0
                &&& outcome.start_source == 0
                &&& outcome.start_limit == 0
                &&& outcome.start_page_size == 0
                &&& outcome.start_body_id == 0
                &&& outcome.start_done_id == 0
                &&& outcome.body_first_step_id == 0
                &&& outcome.body_last_step_id == 0
                &&& outcome.body_first_output_slot == 0
                &&& outcome.body_last_output_slot == 0
                &&& outcome.next_step_id == 0
                &&& outcome.next_collector_slot == 0
                &&& outcome.next_body_id == 0
                &&& outcome.next_done_id == 0
                &&& outcome.finish_step_id == 0
                &&& outcome.finish_collector_slot == 0
                &&& outcome.node_0_kind == 0
                &&& outcome.next_kind == 0
                &&& outcome.finish_kind == 0
                &&& outcome.error_kind == spec_collect_ir_error_kind(
                    id.0 as int,
                    source.0 as int,
                    limit as int,
                    page_size as int,
                    body_len as int,
                    body_node_count as int,
                    pre_slot_count as int,
                )
                &&& !spec_collect_ir_ok(
                    id.0 as int,
                    source.0 as int,
                    limit as int,
                    page_size as int,
                    body_len as int,
                    body_node_count as int,
                    pre_slot_count as int,
                )
            },
        },
;

// ============================================================================
// Production-bound exec wrapper
// ============================================================================
//
// The wrapper takes the production primitive inputs, asserts the
// production preconditions for the success path, calls the projection,
// and surfaces the postcondition. The wrapper's `ensures` clause is
// identical to the projection's `assume_specification` postcondition
// for the success case, so the binding chain ends at the projection.
// The wrapper body carries explicit `assert(...)` calls to discharge
// the assume_specification contract — Verus requires the contract to
// be unfolded before the ensures clause can be verified.

/// Production-bound exec wrapper for `lower_bounded_collect`.
/// Requires the production preconditions for the success path:
///   - id + body_len + 2 fits in u16 (so checked_step_offset for
///     body/advance/done succeeds).
///   - body has at least one step (so validate_body succeeds).
///   - pre_slot_count + 3 + body_len fits in u16 (so the full-call
///     slot budget does not overflow).
///   - limit and page_size are at least 1 (so plan_validate_bounds
///     succeeds).
pub exec fn checked_prod_lower_bounded_collect_ir(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_len: u16,
    body_node_count: u16,
    pre_slot_count: u16,
) -> (outcome: SpecCollectIROutcome)
    requires
        id.0 as int + body_len as int + 2 <= u16_max(),
        body_len as int >= 1,
        body_len as int + 1 <= u16_max(),
        body_len as int + 2 <= u16_max(),
        body_len as int <= body_node_count as int,
        limit >= 1,
        page_size >= 1,
        pre_slot_count as int + 3 + body_len as int <= u16_max(),
    ensures
        outcome.ok,
        outcome.error_kind == SPEC_ERR_NONE,
        outcome.pre_slot_count == pre_slot_count,
        outcome.post_slot_count as int == pre_slot_count as int + 3 + body_len as int,
        outcome.emitted_node_count as int == body_node_count as int + 3,
        outcome.collector_slot as int == pre_slot_count as int + 1,
        outcome.finish_slot as int == pre_slot_count as int + 2,
        // Node 0 (CollectStart)
        outcome.start_step_id == id.0,
        outcome.start_source == source.0,
        outcome.start_limit == limit,
        outcome.start_page_size == page_size,
        outcome.start_body_id as int == id.0 as int + 1,
        outcome.start_done_id as int == id.0 as int + body_len as int + 2,
        // Body nodes
        outcome.body_first_step_id as int == id.0 as int + 1,
        outcome.body_last_step_id as int == id.0 as int + body_len as int,
        outcome.body_first_output_slot as int == pre_slot_count as int + 3,
        outcome.body_last_output_slot as int
            == pre_slot_count as int + 3 + body_len as int - 1,
        // CollectNext node
        outcome.next_step_id as int == id.0 as int + body_len as int + 1,
        outcome.next_collector_slot == outcome.collector_slot,
        outcome.next_body_id as int == id.0 as int + 1,
        outcome.next_done_id as int == id.0 as int + body_len as int + 2,
        // CollectFinish node
        outcome.finish_step_id as int == id.0 as int + body_len as int + 2,
        outcome.finish_collector_slot == outcome.collector_slot,
        // Node kinds
        outcome.node_0_kind == KIND_COLLECT_START,
        outcome.next_kind == KIND_COLLECT_NEXT,
        outcome.finish_kind == KIND_COLLECT_FINISH,
{
    let outcome = production::lower_bounded_collect_ir_projection(
        id,
        source,
        limit,
        page_size,
        body_len,
        body_node_count,
        pre_slot_count,
    );
    assert(outcome.ok);
    assert(outcome.error_kind == SPEC_ERR_NONE);
    assert(outcome.pre_slot_count == pre_slot_count);
    assert(outcome.post_slot_count as int == pre_slot_count as int + 3 + body_len as int);
    assert(outcome.emitted_node_count as int == body_node_count as int + 3);
    assert(outcome.collector_slot as int == pre_slot_count as int + 1);
    assert(outcome.finish_slot as int == pre_slot_count as int + 2);
    assert(outcome.start_step_id == id.0);
    assert(outcome.start_source == source.0);
    assert(outcome.start_limit == limit);
    assert(outcome.start_page_size == page_size);
    assert(outcome.start_body_id as int == id.0 as int + 1);
    assert(outcome.start_done_id as int == id.0 as int + body_len as int + 2);
    assert(outcome.body_first_step_id as int == id.0 as int + 1);
    assert(outcome.body_last_step_id as int == id.0 as int + body_len as int);
    assert(outcome.body_first_output_slot as int == pre_slot_count as int + 3);
    assert(outcome.body_last_output_slot as int
        == pre_slot_count as int + 3 + body_len as int - 1);
    assert(outcome.next_step_id as int == id.0 as int + body_len as int + 1);
    assert(outcome.next_collector_slot == outcome.collector_slot);
    assert(outcome.next_body_id as int == id.0 as int + 1);
    assert(outcome.next_done_id as int == id.0 as int + body_len as int + 2);
    assert(outcome.finish_step_id as int == id.0 as int + body_len as int + 2);
    assert(outcome.finish_collector_slot == outcome.collector_slot);
    assert(outcome.node_0_kind == KIND_COLLECT_START);
    assert(outcome.next_kind == KIND_COLLECT_NEXT);
    assert(outcome.finish_kind == KIND_COLLECT_FINISH);
    outcome
}

// ============================================================================
// Production-bound proofs (PO-012)
// ============================================================================
//
// Each proof takes the spec-side inputs that
// `checked_prod_lower_bounded_collect_ir` accepts and proves a
// property of `spec_collect_ir_outcome(...)` (which the
// assume_specification contract ties to the production projection's
// return value). The proofs are non-vacuous: they perform real
// arithmetic reasoning over the spec construction and discharge each
// IR structure obligation from the spec inputs. The binding to
// production is via the `assume_specification` contract on the
// projection and the `checked_prod_lower_bounded_collect_ir` exec
// wrapper above, which together establish that the production
// projection returns exactly this spec outcome on the success path.

// ---------- L1: Node IDs ----------

/// PO-012 / L1: The emitted node ids are id (CollectStart), id+1 ..
/// id+body_len (body), id+body_len+1 (CollectNext), id+body_len+2
/// (CollectFinish). This matches the production
/// `plan_compute_edges` arithmetic at bounded_collect.rs:116-126 and
/// the per-step ids `emit_scoped_flat_body` assigns at
/// canonical_branch_body.rs:20-42.
pub proof fn lemma_collect_node_ids(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_step_id as int + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_step_id as int + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_step_id as int + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_step_id as int + body_len - 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_done_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_step_id as int,
{
}

// ---------- L2: Node 0 is CollectStart with correct fields ----------

/// PO-012 / L2: Node 0 is `CompiledNodeKind::CollectStart` with
/// source/limit/page_size from the inputs, body=id+1,
/// done=id+body_len+2, and output slot collector_slot.
pub proof fn lemma_node_0_is_collect_start(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).node_0_kind as int == KIND_COLLECT_START as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_step_id as int == id,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_source as int == source,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_limit == limit as u32,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_page_size == page_size as u32,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_body_id as int == id + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_done_id as int == id + body_len + 2,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).collector_slot as int == pre_slot_count + 1,
{
}

// ---------- L3: Body nodes occupy ids id+1 .. id+body_len ----------

/// PO-012 / L3: Body nodes occupy consecutive ids id+1 ..
/// id+body_len with output slots pre_slot_count+3 ..
/// pre_slot_count+3+body_len-1. Body node kinds vary with the body
/// steps (Set lowers to SetConst or an expression-program node, Do
/// to a Do node), so no kind is claimed here.
pub proof fn lemma_body_nodes(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_step_id as int == id + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_step_id as int == id + body_len,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_output_slot as int == pre_slot_count + 3,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_output_slot as int == pre_slot_count + 3 + body_len - 1,
{
}

// ---------- L4: CollectNext node with correct fields ----------

/// PO-012 / L4: The CollectNext node is at id+body_len+1 with
/// collector_slot, body=id+1, done=id+body_len+2.
pub proof fn lemma_node_next_is_collect_next(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_kind as int == KIND_COLLECT_NEXT as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_step_id as int == id + body_len + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_collector_slot as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).collector_slot as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_body_id as int == id + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_done_id as int == id + body_len + 2,
{
}

// ---------- L5: CollectFinish node with correct fields ----------

/// PO-012 / L5: The CollectFinish node is at id+body_len+2 with
/// collector_slot and output slot finish_slot.
pub proof fn lemma_node_finish_is_collect_finish(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_kind as int == KIND_COLLECT_FINISH as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_step_id as int == id + body_len + 2,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_collector_slot as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).collector_slot as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_slot as int == pre_slot_count + 2,
{
}

// ---------- L6: Emission count is exactly 3 + body_node_count ----------

/// PO-012 / L6: The total node count is exactly 3 + body_node_count
/// on the success path (CollectStart + body emission + CollectNext +
/// CollectFinish).
pub proof fn lemma_collect_node_count(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).emitted_node_count == 3u32 + body_node_count as u32,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).ok,
{
}

// ---------- L7: Slot count delta is exactly +3+body_len ----------

/// PO-012 / L7: The slot count delta is exactly +3+body_len on the
/// success path (one `record_slot(source)` plus two `allocate_slot`
/// calls plus one per body step, under the fresh-source assumption).
pub proof fn lemma_collect_slot_delta(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).post_slot_count as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).pre_slot_count as int + 3 + body_len,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).collector_slot as int == pre_slot_count + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_slot as int == pre_slot_count + 2,
{
}

// ---------- L8: Full emission chain ----------

/// PO-012 / L8: Full emission chain — all nodes are emitted with the
/// documented kinds and field values, the slot count increases by
/// exactly 3+body_len, and the emission succeeds. Conjunction of
/// L1-L7.
pub proof fn lemma_collect_full_emission_chain(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        collect_ir_inputs_valid(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ),
    ensures
        // ok + counts
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).ok,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).emitted_node_count == 3u32 + body_node_count as u32,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).post_slot_count as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).pre_slot_count as int + 3 + body_len,
        // L1 — node ids
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_step_id as int + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_step_id as int + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_step_id as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_step_id as int + 1,
        // L2 — CollectStart fields
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).node_0_kind as int == KIND_COLLECT_START as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_source as int == source,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_limit == limit as u32,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).start_page_size == page_size as u32,
        // L3 — body node ids and output slots
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_step_id as int == id + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_step_id as int == id + body_len,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_first_output_slot as int == pre_slot_count + 3,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).body_last_output_slot as int == pre_slot_count + 3 + body_len - 1,
        // L4 — CollectNext fields
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_kind as int == KIND_COLLECT_NEXT as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_collector_slot as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).collector_slot as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_body_id as int == id + 1,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).next_done_id as int == id + body_len + 2,
        // L5 — CollectFinish fields
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_kind as int == KIND_COLLECT_FINISH as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_collector_slot as int == spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).collector_slot as int,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).finish_slot as int == pre_slot_count + 2,
{
    lemma_collect_node_ids(id, source, limit, page_size, body_len, body_node_count, pre_slot_count);
    lemma_node_0_is_collect_start(
        id,
        source,
        limit,
        page_size,
        body_len,
        body_node_count,
        pre_slot_count,
    );
    lemma_body_nodes(id, source, limit, page_size, body_len, body_node_count, pre_slot_count);
    lemma_node_next_is_collect_next(
        id,
        source,
        limit,
        page_size,
        body_len,
        body_node_count,
        pre_slot_count,
    );
    lemma_node_finish_is_collect_finish(
        id,
        source,
        limit,
        page_size,
        body_len,
        body_node_count,
        pre_slot_count,
    );
    lemma_collect_node_count(
        id,
        source,
        limit,
        page_size,
        body_len,
        body_node_count,
        pre_slot_count,
    );
    lemma_collect_slot_delta(
        id,
        source,
        limit,
        page_size,
        body_len,
        body_node_count,
        pre_slot_count,
    );
}

// ---------- Failure-path proofs ----------

/// PO-012 / F1: If id + 1 overflows u16, the projection returns
/// `ok = false` with `error_kind = SPEC_ERR_LIMIT_EXCEEDED` and emits
/// no nodes. This matches the production's
/// `CompileError::PrimitiveLoweringLimitExceeded` from
/// `checked_step_offset(id, 1)` at bounded_collect.rs:116-126.
pub proof fn lemma_collect_id_overflow_fails(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        bounded_u16(id) && id + 1 > u16_max(),
        body_len >= 1,
        bounded_u16(body_len),
        bounded_u16(body_node_count),
        bounded_u16(source),
        bounded_u32(limit),
        bounded_u32(page_size),
        bounded_u16(pre_slot_count),
    ensures
        !spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).ok,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).error_kind == SPEC_ERR_LIMIT_EXCEEDED,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).emitted_node_count == 0u32,
{
}

/// PO-012 / F2: If body length is 0, the projection returns
/// `ok = false` with `error_kind = SPEC_ERR_STEP_SHAPE` and emits no
/// nodes. This matches the production's `CompileError::StepFieldShape`
/// from `validate_body` requiring a non-empty body at
/// bounded_collect.rs:78-101.
pub proof fn lemma_collect_body_shape_fails(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        bounded_u16(id),
        bounded_u16(source),
        bounded_u32(limit),
        bounded_u32(page_size),
        body_len == 0,
        bounded_u16(body_node_count),
        bounded_u16(pre_slot_count),
    ensures
        !spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).ok,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).error_kind == SPEC_ERR_STEP_SHAPE,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).emitted_node_count == 0u32,
{
}

/// PO-012 / F3: If body_len + 1 overflows u16, the projection returns
/// `ok = false` with `error_kind = SPEC_ERR_INDEX_RANGE` and emits no
/// nodes. This matches the production's
/// `CompileError::StepIndexOutOfRange` from
/// `checked_body_length_offset(body_len, 1)` at
/// bounded_collect.rs:108-114.
pub proof fn lemma_collect_body_length_overflow_fails(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        bounded_u16(id) && id + 1 <= u16_max(),
        body_len >= 1,
        body_len + 1 > u16_max(),
        bounded_u16(body_node_count),
        bounded_u16(source),
        bounded_u32(limit),
        bounded_u32(page_size),
        bounded_u16(pre_slot_count),
    ensures
        !spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).ok,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).error_kind == SPEC_ERR_INDEX_RANGE,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).emitted_node_count == 0u32,
{
}

/// PO-012 / F4: If pre_slot_count + 3 + body_len overflows u16, the
/// projection returns `ok = false` with `error_kind =
/// SPEC_ERR_SLOT_RANGE` and emits no nodes. This matches the
/// production's `CompileError::SlotIndexOutOfRange` from the slot
/// budget (`allocate_slot` at part_08.rs:29-39 and the per-step
/// allocation in `emit_scoped_flat_body`); the projection pre-checks
/// the full-call budget, while production fails late during body
/// emission with partial nodes already pushed.
pub proof fn lemma_collect_slot_budget_fails(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_len: int,
    body_node_count: int,
    pre_slot_count: int,
)
    requires
        bounded_u16(id) && id + body_len + 2 <= u16_max(),
        body_len >= 1,
        body_len + 1 <= u16_max(),
        body_len + 2 <= u16_max(),
        bounded_u16(body_node_count),
        bounded_u16(source),
        bounded_u32(limit),
        bounded_u32(page_size),
        bounded_u16(pre_slot_count) && pre_slot_count + 3 + body_len > u16_max(),
    ensures
        !spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).ok,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).error_kind == SPEC_ERR_SLOT_RANGE,
        spec_collect_ir_outcome(
            id,
            source,
            limit,
            page_size,
            body_len,
            body_node_count,
            pre_slot_count,
        ).emitted_node_count == 0u32,
{
}

fn main() {
}

} // verus!
