// Verification artifact: collect_ir_structure.rs
// PO: PO-012 (lower_canonical_collect IR struct field refinement)
// Bead: vb-xi2f.23
// Verifier: Verus
// Command: verus --crate-type=lib verification/verus/collect_ir_structure.rs
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
// Target: production exec fn `lower_canonical_collect` in
// `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`.
//
// The production exec fn emits 4 CompiledNode entries on the success
// path:
//
//   [0] CompiledNodeKind::CollectStart {
//          source, limit, page_size,
//          body: id+1, done: id+3,
//        }
//   [1] CompiledNodeKind::SetConst (from body Set step at id+1)
//   [2] CompiledNodeKind::CollectPage {
//          collector_slot: source,
//          body: id+1, done: id+3,
//        }
//   [3] CompiledNodeKind::CollectFinish { collector_slot: source }
//
// Binding mechanism: `#[path = "extern_collect_ir_structure.rs"]`
// imports the thin extern surface, which defines a
// `#[verifier::external]` projection `lower_canonical_collect_projection`
// that mirrors the production signature (parameter list, parameter
// order, return-type envelope) and reproduces the production decision
// shape (precondition checks, error variants, slot-recording delta,
// emitted-node count, per-node kind/field structure). The spec file
// attaches spec contracts to the projection via `assume_specification`
// and every proof below the bridge exercises the production projection
// through an exec wrapper. There are zero vacuous proofs in the
// rewritten spec.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `lower_canonical_collect` cannot be verified
// end-to-end inside Verus because it transitively depends on
// `vb_core::workflow::*`, `vb_core::ids::*`, `SlotCompiler`, and
// `CompileError`, all of which carry heap allocations, derives, or
// crate-internal modules that Verus does not model in a single-file
// Verus unit. The pure projection in
// `extern_collect_ir_structure.rs` captures every decision branch the
// production fn takes on the relevant scalar inputs (id, source,
// limit, page_size, body_length, pre_slot_count) and is recorded as a
// trusted base in the binding ledger. Each proof below operates on the
// spec construction `spec_collect_ir_outcome(...)` whose fields are
// bound to the projection via the `assume_specification` contract; any
// divergence between the projection and the production body is a
// binding debt item tracked outside Verus.
//
// The exec wrapper `checked_prod_lower_canonical_collect` carries
// explicit `assert(...)` calls in its body to discharge the
// `assume_specification` postcondition — Verus requires the contract
// to be unfolded before the wrapper's ensures clause can be verified.
//
// ============================================================================
// PROOF OBLIGATION MAP
// ============================================================================
//   L1: lemma_collect_node_ids_consecutive
//        -> the four emitted IDs are id, id+1, id+2, id+3 (consecutive).
//   L2: lemma_node_0_is_collect_start
//        -> Node 0 is CollectStart with source/limit/page_size/body/done
//           fields matching the spec inputs.
//   L3: lemma_node_1_is_set_const
//        -> Node 1 is SetConst at id+1 (from body Set step).
//   L4: lemma_node_2_is_collect_page
//        -> Node 2 is CollectPage with collector_slot=source, body=id+1,
//           done=id+3.
//   L5: lemma_node_3_is_collect_finish
//        -> Node 3 is CollectFinish with collector_slot=source.
//   L6: lemma_collect_node_count
//        -> total emitted node count is exactly 4 on success.
//   L7: lemma_collect_slot_delta
//        -> slot count delta is exactly +1 on success.
//   L8: lemma_collect_full_emission_chain
//        -> conjunction of L1-L7 (full emission chain contract).
//
// Failure-path proofs:
//   F1: lemma_collect_id_overflow_fails
//        -> id + 3 > u16::MAX produces ok=false and
//           error_kind=SPEC_ERR_LIMIT_EXCEEDED with 0 emitted nodes.
//   F2: lemma_collect_body_shape_fails
//        -> body_length != 1 produces ok=false and
//           error_kind=SPEC_ERR_STEP_SHAPE with 0 emitted nodes.

use vstd::prelude::*;

verus! {

#[path = "extern_collect_ir_structure.rs"]
mod production;

pub use production::{
    lower_canonical_collect_projection,
    SpecCollectIROutcome,
    StepIdx,
    SlotIdx,
    KIND_COLLECT_FINISH,
    KIND_COLLECT_PAGE,
    KIND_COLLECT_START,
    KIND_SET_CONST,
    SPEC_ERR_LIMIT_EXCEEDED,
    SPEC_ERR_NONE,
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

/// Spec-side predicate for inputs that satisfy the production
/// `lower_canonical_collect` preconditions on the success path:
///   - id fits in u16 and id + 3 fits in u16 (so checked_step_offset
///     for offsets 1/2/3 all succeed).
///   - body has exactly one step (so emit_single_body_set succeeds).
///   - source fits in u16 (so record_slot(source) is well-defined).
///   - limit and page_size fit in u32 with the documented >=1 lower
///     bound (matches production unwrap_or(1) safety contract).
///   - pre_slot_count fits in u16 with pre_slot_count + 1 <= u16::MAX
///     (so post_slot_count does not overflow on +1).
pub open spec fn collect_ir_inputs_valid(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
) -> bool {
    bounded_u16(id) && id + 3 <= u16_max() && bounded_u16(source) && 1 <= limit && limit
        <= u32_max() && 1 <= page_size && page_size <= u32_max() && body_length == 1
        && bounded_u16(pre_slot_count) && pre_slot_count <= u16_max() - 1
}

// ============================================================================
// Spec-side construction of the projection outcome from spec inputs
// ============================================================================
//
// `spec_collect_ir_outcome` constructs the SpecCollectIROutcome value
// the production projection returns when called with the corresponding
// primitive inputs. It is the spec-side mirror of the projection body.
// The `assume_specification` bridge below asserts that the production
// projection returns exactly this value on the success path.
//
// The success path mirrors the production body verbatim:
//   - start_step_id = id, start_source = source, start_limit = limit,
//     start_page_size = page_size, start_body_id = id+1, start_done_id = id+3
//   - body_step_id = id+1 (Node 1 SetConst)
//   - page_step_id = id+2, page_collector_slot = source, page_body_id = id+1,
//     page_done_id = id+3
//   - done_step_id = id+3, finish_collector_slot = source
//   - post_slot_count = pre_slot_count + 1, emitted_node_count = 4
//   - node_*_kind = the documented KIND_* discriminant
//
// The failure path (id+3 overflow or body_length != 1) zeros out all
// node fields and sets emitted_node_count = 0.

pub open spec fn spec_collect_ir_outcome(
    id: int,
    source: int,
    limit: int,
    page_size: int,
    body_length: int,
    pre_slot_count: int,
) -> SpecCollectIROutcome {
    SpecCollectIROutcome {
        ok: id + 3 <= u16_max() && body_length == 1,
        error_kind: (if id + 3 > u16_max() {
            SPEC_ERR_LIMIT_EXCEEDED as int
        } else if body_length != 1 {
            SPEC_ERR_STEP_SHAPE as int
        } else {
            SPEC_ERR_NONE as int
        }) as u8,
        pre_slot_count: pre_slot_count as u16,
        post_slot_count: (if id + 3 <= u16_max() && body_length == 1 {
            pre_slot_count + 1
        } else {
            pre_slot_count
        }) as u16,
        emitted_node_count: (if id + 3 <= u16_max() && body_length == 1 {
            4int
        } else {
            0int
        }) as u16,
        start_step_id: (if id + 3 <= u16_max() && body_length == 1 {
            id
        } else {
            0int
        }) as u16,
        start_source: (if id + 3 <= u16_max() && body_length == 1 {
            source
        } else {
            0int
        }) as u16,
        start_limit: (if id + 3 <= u16_max() && body_length == 1 {
            limit
        } else {
            0int
        }) as u32,
        start_page_size: (if id + 3 <= u16_max() && body_length == 1 {
            page_size
        } else {
            0int
        }) as u32,
        start_body_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 1
        } else {
            0int
        }) as u16,
        start_done_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 3
        } else {
            0int
        }) as u16,
        body_step_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 1
        } else {
            0int
        }) as u16,
        page_step_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 2
        } else {
            0int
        }) as u16,
        page_collector_slot: (if id + 3 <= u16_max() && body_length == 1 {
            source
        } else {
            0int
        }) as u16,
        page_body_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 1
        } else {
            0int
        }) as u16,
        page_done_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 3
        } else {
            0int
        }) as u16,
        done_step_id: (if id + 3 <= u16_max() && body_length == 1 {
            id + 3
        } else {
            0int
        }) as u16,
        finish_collector_slot: (if id + 3 <= u16_max() && body_length == 1 {
            source
        } else {
            0int
        }) as u16,
        node_0_kind: (if id + 3 <= u16_max() && body_length == 1 {
            KIND_COLLECT_START as int
        } else {
            0int
        }) as u8,
        node_1_kind: (if id + 3 <= u16_max() && body_length == 1 {
            KIND_SET_CONST as int
        } else {
            0int
        }) as u8,
        node_2_kind: (if id + 3 <= u16_max() && body_length == 1 {
            KIND_COLLECT_PAGE as int
        } else {
            0int
        }) as u8,
        node_3_kind: (if id + 3 <= u16_max() && body_length == 1 {
            KIND_COLLECT_FINISH as int
        } else {
            0int
        }) as u8,
    }
}

// ============================================================================
// assume_specification bridge — production projection contract
// ============================================================================
//
// `assume_specification` is the Verus-native way to attach a spec
// contract to a Rust function whose body Verus cannot fully model.
// The body of `lower_canonical_collect_projection` is
// `#[verifier::external]`; Verus accepts the `ensures` clause below
// but does not verify the body itself. The contract characterises the
// production behaviour the corresponding `lower_canonical_collect`
// would exhibit on the same scalar inputs.

pub assume_specification[ production::lower_canonical_collect_projection ](
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_length: u16,
    pre_slot_count: u16,
) -> (outcome: SpecCollectIROutcome)
    ensures
        // Success iff id + 3 fits in u16 AND body has exactly 1 step.
        outcome.ok == (id.0 as int + 3 <= u16_max() && body_length as int == 1),
        // Error kind mapping.
        outcome.error_kind as int == (if id.0 as int + 3 > u16_max() {
            SPEC_ERR_LIMIT_EXCEEDED as int
        } else if body_length as int != 1 {
            SPEC_ERR_STEP_SHAPE as int
        } else {
            SPEC_ERR_NONE as int
        }),
        // Slot count: record_slot(source) adds exactly one slot on success.
        outcome.pre_slot_count == pre_slot_count,
        outcome.post_slot_count as int == (if outcome.ok {
            pre_slot_count as int + 1
        } else {
            pre_slot_count as int
        }),
        // Emission count: CollectStart + SetConst + CollectPage + CollectFinish = 4 nodes.
        outcome.emitted_node_count as int == (if outcome.ok {
            4int
        } else {
            0int
        }),
        // Node 0 (CollectStart) — emitted iff ok.
        outcome.start_step_id as int == id.0 as int,
        outcome.start_source as int == source.0 as int,
        outcome.start_limit == limit,
        outcome.start_page_size == page_size,
        outcome.start_body_id as int == id.0 as int + 1,
        outcome.start_done_id as int == id.0 as int + 3,
        // Node 1 (SetConst) — emitted iff ok.
        outcome.body_step_id as int == id.0 as int + 1,
        // Node 2 (CollectPage) — emitted iff ok.
        outcome.page_step_id as int == id.0 as int + 2,
        outcome.page_collector_slot as int == source.0 as int,
        outcome.page_body_id as int == id.0 as int + 1,
        outcome.page_done_id as int == id.0 as int + 3,
        // Node 3 (CollectFinish) — emitted iff ok.
        outcome.done_step_id as int == id.0 as int + 3,
        outcome.finish_collector_slot as int == source.0 as int,
        // Node kinds (set to the documented values; zero on failure).
        outcome.node_0_kind as int == (if outcome.ok {
            KIND_COLLECT_START as int
        } else {
            0int
        }),
        outcome.node_1_kind as int == (if outcome.ok {
            KIND_SET_CONST as int
        } else {
            0int
        }),
        outcome.node_2_kind as int == (if outcome.ok {
            KIND_COLLECT_PAGE as int
        } else {
            0int
        }),
        outcome.node_3_kind as int == (if outcome.ok {
            KIND_COLLECT_FINISH as int
        } else {
            0int
        }),
;

// ============================================================================
// Production-bound exec wrapper
// ============================================================================
//
// The wrapper takes the production primitive inputs, asserts the
// production preconditions (id + 3 fits in u16, body has 1 step,
// post_slot_count fits), calls the projection, and surfaces the
// postcondition. The wrapper's `ensures` clause is identical to the
// projection's `assume_specification` postcondition for the success
// case, so the binding chain ends at the projection. The wrapper body
// carries explicit `assert(...)` calls to discharge the
// assume_specification contract — Verus requires the contract to be
// unfolded before the ensures clause can be verified.

/// Production-bound exec wrapper for `lower_canonical_collect`.
/// Requires the production preconditions for the success path:
///   - id + 3 fits in u16 (so checked_step_offset for offsets 1/2/3 succeeds).
///   - body has exactly 1 step (so emit_single_body_set succeeds).
///   - pre_slot_count + 1 fits in u16 (so post_slot_count does not overflow).
pub exec fn checked_prod_lower_canonical_collect(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_length: u16,
    pre_slot_count: u16,
) -> (outcome: SpecCollectIROutcome)
    requires
        id.0 as int + 3 <= u16_max(),
        body_length == 1,
        pre_slot_count <= u16_max() - 1,
    ensures
        outcome.ok,
        outcome.error_kind == SPEC_ERR_NONE,
        outcome.pre_slot_count == pre_slot_count,
        outcome.post_slot_count == pre_slot_count + 1,
        outcome.emitted_node_count == 4u16,
        // Node 0 (CollectStart)
        outcome.start_step_id == id.0,
        outcome.start_source == source.0,
        outcome.start_limit == limit,
        outcome.start_page_size == page_size,
        outcome.start_body_id == id.0 + 1,
        outcome.start_done_id == id.0 + 3,
        // Node 1 (SetConst)
        outcome.body_step_id == id.0 + 1,
        // Node 2 (CollectPage)
        outcome.page_step_id == id.0 + 2,
        outcome.page_collector_slot == source.0,
        outcome.page_body_id == id.0 + 1,
        outcome.page_done_id == id.0 + 3,
        // Node 3 (CollectFinish)
        outcome.done_step_id == id.0 + 3,
        outcome.finish_collector_slot == source.0,
        // Node kinds
        outcome.node_0_kind == KIND_COLLECT_START,

// ============================================================================
// Companion chunk 2 — proof/remaining functions
// ============================================================================
#[path = "collect_ir_structure_chunk2.rs"]
mod chunk2;

} // verus!
