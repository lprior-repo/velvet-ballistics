// SPDX-License-Identifier: MIT
//
// Extern surface for vb_xi2f_compile_source Verus spec.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-bound re-export surface for the
// `compile_source` production chain. The body of the production
// `compile_source` (and its transitive surfaces — `canonical_layout`,
// `lower_canonical_step`, the `SlotCompiler`, the `WorkflowParts`
// constructor, and `vb_validate::shared::validate`) is mirrored
// verbatim in
// `verification/verus/production_inner/vb_xi2f_compile_source_production.rs`,
// which carries the drift policy header and per-section `// Production
// `path:start-end`` claims that the `scripts/check-production-inner-drift.sh`
// gate verifies.
//
// Steps 1-7 of the production chain (canonical_layout,
// lower_canonical_step loop, WorkflowParts construction,
// vb_validate::shared::validate) are now drift-checked against
// production at:
//   - crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60
//   - crates/vb_compile/src/mod_compile_lowering/part_01.rs:68-84
//   - crates/vb_compile/src/mod_compile_lowering/part_02.rs:18-104
//   - crates/vb_validate/src/shared.rs:156-158
//
// Step 8 (`CompiledWorkflow::try_from_parts`) is bound through the
// sibling `try_from_parts_production.rs` mirror.
//
// The companion spec file (`vb_xi2f_compile_source.rs`) attaches spec
// contracts to the projection via `assume_specification`, and every
// proof below the bridge exercises the production projection through
// an exec wrapper. There are zero vacuous proofs in the rewritten
// spec.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The body of the production chain (`compile_source` + transitive
// `canonical_layout`, `lower_canonical_step`, `SlotCompiler`,
// `WorkflowParts`, `shared_validate`) is NOT verified by Verus. The
// mirror is marked `#[verifier::external]` at the projection level so
// Verus skips body verification; the inclusion still validates Rust
// resolution (field names, discriminant sets, fn signatures) at
// compile time. Any drift in the production impl surface breaks this
// Verus build.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK BINDING)
// ============================================================================
//
// Direct `#[path]` inclusion of the in-tree production mirror at
// `production_inner/vb_xi2f_compile_source_production.rs`. The mirror
// declares the complete production chain (steps 1-7) plus the
// `#[verifier::external]` projection. The mirror in turn
// `#[path]`-includes `try_from_parts_production.rs` for the
// `WorkflowParts` / `CompiledWorkflow` / `ResourceContract` types and
// the `CompiledWorkflow::try_from_parts` body (step 8). The combined
// surface establishes a real end-to-end binding: any drift in the
// production field names, discriminant sets, or fn signatures breaks
// the `extern_vb_xi2f_compile_source` mirror and the spec proofs
// that depend on it.

#[path = "production_inner/vb_xi2f_compile_source_production.rs"]
pub mod prod_src;

pub use prod_src::{
    canonical_layout_tag, canonical_step_width_tag, compile_source_production,
    extend_step_names_for_generated, layout_start, lower_canonical_step_tag, next_layout_start,
    shared_validate, validate_gate_07_expression_stack_depth,
    validate_gate_08_accessor_path_segments, validate_gate_09_slot_references,
    validate_gate_10_node_kind_specific, validate_gate_11_loop_body_graph,
    validate_gate_13_no_slot_cycles, validate_gate_14_slot_type_consistency,
    validate_gate_15_determinism_proof, CanonicalStepLayout, SlotCompiler, SpecCompileError,
    StepPrimitiveTag, ValidationError, ValidationPipeline, ValidationResult,
};

// ============================================================================
// Production type re-exports for the spec file
// ============================================================================
//
// The spec file's `compile_source_pure` projection only needs the
// types it directly references. Re-export them here so the spec
// file can use the `production::` namespace without re-importing
// the try_from_parts_production.rs mirror.
pub use prod_src::try_from_parts_mirror::{
    validate_parts as try_from_parts_validate_parts, validate_budget as try_from_parts_validate_budget,
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};
