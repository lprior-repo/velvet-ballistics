// SPDX-License-Identifier: MIT
//
// Extern surface for collect_ir_structure Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the
// `collect_ir_structure.rs` Verus spec. It contains:
//
//   1. A `#[path]` inclusion of the verbatim production mirror at
//      `verification/verus/production_inner/lower_canonical_collect_production.rs`,
//      which is itself a verbatim copy of
//      `crates/vb_compile/src/mod_compile_lowering/part_03.rs:195-256`
//      with minimal substitutions (u16 newtypes for StepIdx/SlotIdx,
//      local CompileError mirror, local SlotCompiler mirror). Any
//      rename, discriminant drift, or signature change in the
//      production source breaks this mirror at compile time.
//
//   2. A spec-side `lower_canonical_collect_projection` exec fn that
//      reduces the production `lower_canonical_collect` signature
//      (which takes `CollectLowering<'_>` and `&mut SlotCompiler`) to
//      the scalar envelope the spec reasons about. The exec fn is
//      marked `#[verifier::external]` so Verus skips body
//      verification; the companion spec file attaches the production
//      contract via `assume_specification`.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `lower_canonical_collect` is NOT verified by
// Verus. The `lower_canonical_collect_projection` body is
// `#[verifier::external]`, the contract is attached via
// `assume_specification` in the companion spec file
// `collect_ir_structure.rs`, and exec wrappers in the spec file are
// the non-vacuum witnesses that the bridge contracts hold.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
#[path = "production_inner/lower_canonical_collect_production.rs"]
pub mod prod_src;

// Re-export the production newtypes (StepIdx, SlotIdx, etc.) so the
// spec file can reference them.
pub use prod_src::{SlotIdx, StepIdx};

// ============================================================================
// SpecCollectIROutcome — projection return shape
// ============================================================================
pub struct SpecCollectIROutcome {
    pub ok: bool,
    pub error_kind: u8,
    pub pre_slot_count: u16,
    pub post_slot_count: u16,
    pub emitted_node_count: u16,
    pub start_step_id: u16,
    pub start_source: u16,
    pub start_limit: u32,
    pub start_page_size: u32,
    pub start_body_id: u16,
    pub start_done_id: u16,
    pub body_step_id: u16,
    pub page_step_id: u16,
    pub page_collector_slot: u16,
    pub page_body_id: u16,
    pub page_done_id: u16,
    pub done_step_id: u16,
    pub finish_collector_slot: u16,
    pub node_0_kind: u8,
    pub node_1_kind: u8,
    pub node_2_kind: u8,
    pub node_3_kind: u8,
}

pub const SPEC_ERR_NONE: u8 = 0;
pub const SPEC_ERR_LIMIT_EXCEEDED: u8 = 1;
pub const SPEC_ERR_STEP_SHAPE: u8 = 2;
pub const KIND_COLLECT_START: u8 = 0;
pub const KIND_SET_CONST: u8 = 1;
pub const KIND_COLLECT_PAGE: u8 = 2;
pub const KIND_COLLECT_FINISH: u8 = 3;

// ============================================================================
// Projection exec fn (lower_canonical_collect)
// ============================================================================
#[verifier::external]
pub fn lower_canonical_collect_projection(
    id: StepIdx,
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_length: u16,
    pre_slot_count: u16,
) -> SpecCollectIROutcome {
    if id.0.checked_add(3).is_none() {
        return SpecCollectIROutcome {
            ok: false,
            error_kind: SPEC_ERR_LIMIT_EXCEEDED,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
            start_step_id: 0,
            start_source: 0,
            start_limit: 0,
            start_page_size: 0,
            start_body_id: 0,
            start_done_id: 0,
            body_step_id: 0,
            page_step_id: 0,
            page_collector_slot: 0,
            page_body_id: 0,
            page_done_id: 0,
            done_step_id: 0,
            finish_collector_slot: 0,
            node_0_kind: 0,
            node_1_kind: 0,
            node_2_kind: 0,
            node_3_kind: 0,
        };
    }
    if body_length != 1 {
        return SpecCollectIROutcome {
            ok: false,
            error_kind: SPEC_ERR_STEP_SHAPE,
            pre_slot_count,
            post_slot_count: pre_slot_count,
            emitted_node_count: 0,
            start_step_id: 0,
            start_source: 0,
            start_limit: 0,
            start_page_size: 0,
            start_body_id: 0,
            start_done_id: 0,
            body_step_id: 0,
            page_step_id: 0,
            page_collector_slot: 0,
            page_body_id: 0,
            page_done_id: 0,
            done_step_id: 0,
            finish_collector_slot: 0,
            node_0_kind: 0,
            node_1_kind: 0,
            node_2_kind: 0,
            node_3_kind: 0,
        };
    }
    let id_val = id.0;
    SpecCollectIROutcome {
        ok: true,
        error_kind: SPEC_ERR_NONE,
        pre_slot_count,
        post_slot_count: pre_slot_count.saturating_add(1),
        emitted_node_count: 4,
        start_step_id: id_val,
        start_source: source.0,
        start_limit: limit,
        start_page_size: page_size,
        start_body_id: id_val + 1,
        start_done_id: id_val + 3,
        body_step_id: id_val + 1,
        page_step_id: id_val + 2,
        page_collector_slot: source.0,
        page_body_id: id_val + 1,
        page_done_id: id_val + 3,
        done_step_id: id_val + 3,
        finish_collector_slot: source.0,
        node_0_kind: KIND_COLLECT_START,
        node_1_kind: KIND_SET_CONST,
        node_2_kind: KIND_COLLECT_PAGE,
        node_3_kind: KIND_COLLECT_FINISH,
    }
}

} // verus!