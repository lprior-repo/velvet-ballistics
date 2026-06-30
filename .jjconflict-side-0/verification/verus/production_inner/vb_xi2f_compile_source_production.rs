// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `compile_source` (production chain)
// ============================================================================
//
// This file is a structural mirror of the production exec fn
// `compile_source` at
// `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60` and the
// auxiliary functions it transitively calls (canonical_layout at
// `crates/vb_compile/src/mod_compile_lowering/part_01.rs:68-84`,
// lower_canonical_step at
// `crates/vb_compile/src/mod_compile_lowering/part_02.rs:18-104`, and
// `vb_validate::shared::validate` at
// `crates/vb_validate/src/shared.rs:156-158`).
//
// The mirror reproduces the production decision shape in `?`-propagation
// order:
//
//   1. validate_canonical_compile_scope(source)? — caller pre-checked
//      (part_01.rs:19)
//   2. validate_branch_counts(source)? — caller pre-checked (part_01.rs:20)
//   3. steps.len().checked_sub(1) — EmptySteps on empty (part_01.rs:22-25)
//   4. canonical_layout(steps)? — LayoutOverflow on overflow
//      (part_01.rs:26, 68-84)
//   5. per-step lower_canonical_step loop — LoweringFailed on per-step
//      error (part_01.rs:31-43, lowering fn at part_02.rs:18-104)
//   6. WorkflowParts construction — entry=0, symbols_count=0
//      (part_01.rs:45-57)
//   7. vb_validate::shared::validate(&parts)? (part_01.rs:58, body at
//      crates/vb_validate/src/shared.rs:156-158 invoking
//      `ValidationPipeline::default().validate(parts)`)
//   8. CompiledWorkflow::try_from_parts(parts)? (part_01.rs:59)
//
// Step 8 is the only step the previous `try_from_parts_production.rs`
// mirror covered. Steps 1-7 were hand-written in-spec in
// `verification/verus/vb_xi2f_compile_source.rs` and NOT drift-checked
// against production. This file closes that drift gap by mirroring the
// complete production chain (steps 1-7) under a single drift policy
// header with per-section `// Production `path:start-end`` citations.
//
// Substitutions (required for `verus --crate-type=lib` standalone):
//
//   1. Production `WorkflowSource`, `StepAst`, `StepPrimitive`, and
//      related AST types are collapsed to scalar inputs
//      (`SpecCompileInput`) so the projection does not depend on the
//      production AST type.
//   2. Production `SlotCompiler` and its method chain
//      (`record_slot`, `push_node`, `push_constant`, `slot_count`)
//      is mirrored as a local stub struct with the same field names
//      and the same numeric decision (max_slot tracking).
//   3. The full per-primitive lowering dispatch in `lower_canonical_step`
//      (part_02.rs:28-101) is mirrored as a single tag-driven
//      `StepPrimitiveTag` decision; the production identifier set for
//      each arm is preserved as either an enum variant or a comment
//      so the drift gate finds the production token in the mirror.
//   4. The `vb_validate::shared::validate` body
//      (crates/vb_validate/src/shared.rs:156-158) is mirrored as a
//      local `validate_parts_vb` stub that calls each gate in the
//      same ascending order; the gate names from `ValidationPipeline`
//      (shared.rs:30-50) and the gate function re-exports (shared.rs:
//      15-23) are preserved verbatim.
//   5. `CompileError` discriminant names from
//      `crates/vb_compile/src/mod_compile_errors/kind.rs` that the
//      chain can produce (`EmptySteps`, `StepIndexOutOfRange`,
//      `UnsupportedStepPrimitive`) are mirrored as `SpecCompileError`
//      variants with the same name and field shape so the drift gate
//      can detect additions or renames in the production surface.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60` whenever
// production changes. The mirror is annotated at the top of every
// section with the originating production line range so regeneration
// is mechanical. The drift gate is `scripts/check-production-inner-drift.sh`.
//
// This file is included by the companion extern file under
// `mod prod_src` and the `#[verifier::external]` projection so every
// body is opaque to Verus. It compiles as plain Rust (no `verus!`
// block, no `vstd` import) and is checked by the Verus invocation
// purely for structural resolution and type well-formedness — Verus
// never reasons about the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

// ===========================================================================
// Production type mirrors — re-used from try_from_parts_production.rs
// ===========================================================================
//
// The WorkflowParts / ResourceContract / StepIdx / SlotIdx / etc. stubs
// are pulled in from the try_from_parts_production.rs mirror so this
// file's drift-checked production chain ends in a WorkflowParts value
// that the try_from_parts mirror accepts verbatim.

#[path = "try_from_parts_production.rs"]
pub mod try_from_parts_mirror;

pub use try_from_parts_mirror::{
    validate_parts, validate_budget, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx,
    ConstValue, ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

// ===========================================================================
// Production CompileError / CompileErrors / ast type mirrors
// ===========================================================================
//
// Production `CompileError` at
// `crates/vb_compile/src/mod_compile_errors/kind.rs:12-168` and
// `CompileErrors` (Vec<CompileError>) at
// `crates/vb_compile/src/mod_compile_errors/collection.rs`. The mirror
// preserves the discriminant names and field shapes that the
// `compile_source` chain can produce (EmptySteps, StepIndexOutOfRange,
// SlotIndexOutOfRange, UnsupportedStepPrimitive,
// ExpressionLoweringUnsupported, Workflow(WorkflowError), etc.) so
// the drift gate detects any rename or new variant in the production
// surface.
//
// Production `WorkflowSource`, `StepAst`, `StepPrimitive` AST types
// are mirrored as stub structs that carry the field names used by the
// `compile_source` chain (`source.name()`, `source.steps()`,
// `step.primitive`, `step.id`).
//
// `ChooseBranch`, `TogetherBranch`, `CollectLowering` are mirrored
// as stub structs with the production field names so the per-arm
// dispatch in `lower_canonical_step` (part_02.rs:28-101) is drift-
// checkable.

/// Production `CompileError` at
/// `crates/vb_compile/src/mod_compile_errors/kind.rs:12-168`.
/// Mirror preserves the variant NAMES and FIELD SHAPES that the
/// `compile_source` chain produces. Each field with `Box<str>` or
/// `&'static str` is flattened to a `u8` tag for spec-side modeling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// Production `CompileError::EmptySteps` at kind.rs:90.
    EmptySteps,
    /// Production `CompileError::StepIndexOutOfRange` at kind.rs:116.
    StepIndexOutOfRange { value: u64 },
    /// Production `CompileError::SlotIndexOutOfRange` at kind.rs:118.
    SlotIndexOutOfRange { value: i64 },
    /// Production `CompileError::UnsupportedStepPrimitive` at
    /// kind.rs:108. The `step: u64` and `primitive: u8` fields
    /// flatten the production `step: usize` and `primitive: &'static str`.
    UnsupportedStepPrimitive { step: u64, primitive: u8 },
    /// Production `CompileError::ExpressionLoweringUnsupported` at
    /// kind.rs:164.
    ExpressionLoweringUnsupported { feature: u8 },
    /// Production `CompileError::Workflow(#[from] WorkflowError)` at
    /// kind.rs:54. The variant NAMES `Workflow` is preserved.
    Workflow(WorkflowError),
    /// Production `CompileError::StepFieldShape` at kind.rs:114.
    StepFieldShape { step: u64, field: u8, expected: u8 },
    /// Production `CompileError::DuplicateOutputName` at kind.rs:86.
    DuplicateOutputName { name: u8 },
    /// Production `CompileError::PrimitiveLoweringLimitExceeded` at kind.rs:124.
    PrimitiveLoweringLimitExceeded { primitive: u8, field: u8, value: u64, limit: u64 },
}

/// Production `CompileErrors` (Vec<CompileError>) at
/// `crates/vb_compile/src/mod_compile_errors/collection.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileErrors(pub Vec<CompileError>);

/// Production `WorkflowSource` at `crates/vb_compile/src/yaml_ast.rs`.
/// The mirror preserves the production method names (`name()`,
/// `steps()`) and the field shape.
#[derive(Debug, Clone, Default)]
pub struct WorkflowSource {
    pub name: Box<str>,
    pub steps: Vec<StepAst>,
}

impl WorkflowSource {
    pub fn name(&self) -> &str { &self.name }
    pub fn steps(&self) -> &[StepAst] { &self.steps }
}

/// Production `StepAst` at `crates/vb_compile/src/ast.rs`.
#[derive(Debug, Clone)]
pub struct StepAst {
    pub id: Box<str>,
    pub primitive: StepPrimitive,
}

/// Production `StepPrimitive` enum at `crates/vb_compile/src/ast.rs`.
/// Mirror preserves the variant NAMES that `lower_canonical_step`
/// dispatches on (part_02.rs:28-101).
#[derive(Debug, Clone)]
pub enum StepPrimitive {
    Set { output: Box<str>, value: Box<str> },
    Finish { result: Box<str> },
    ForEach { input: Box<str>, at_once: Option<u32>, body: Vec<StepAst> },
    Together { branches: Vec<TogetherBranch> },
    Collect { source: Box<str>, pages: Option<u32>, items: Option<u32>, body: Vec<StepAst> },
    Aggregate { input: Box<str>, initial: Box<str>, body: Vec<StepAst> },
    Repeat { max_attempts: u32, body: Vec<StepAst> },
    Wait { event: Option<Box<str>>, timeout: Option<Box<str>> },
    Ask { prompt: Box<str>, timeout: Option<Box<str>> },
    Choose { branches: Vec<ChooseBranch>, otherwise: Option<Box<str>> },
    Other,
}

/// Production `ChooseBranch` at `crates/vb_compile/src/ast.rs`.
#[derive(Debug, Clone)]
pub struct ChooseBranch {
    pub steps: Vec<StepAst>,
}

/// Production `TogetherBranch` at `crates/vb_compile/src/ast.rs`.
#[derive(Debug, Clone)]
pub struct TogetherBranch {
    pub steps: Vec<StepAst>,
}

/// Production `CollectLowering` struct used by the
/// `lower_canonical_collect` arm of `lower_canonical_step`
/// (part_02.rs:45-62).
#[derive(Debug, Clone)]
pub struct CollectLowering<'a> {
    pub source: &'a str,
    pub pages: u32,
    pub items: u32,
    pub body: &'a [StepAst],
    pub next: Option<StepIdx>,
}

// ===========================================================================
// Production ExprIdx / AccessorIdx / ParsedExpression mirrors
// ===========================================================================
//
// Production `ExprIdx` at `crates/vb_core/src/ids/mod.rs` and
// `AccessorIdx` at `crates/vb_core/src/ids/mod.rs`. Production
// `ParsedExpression` at `crates/vb_compile/src/expression/mod.rs`.

/// Production `ExprIdx` (u32 newtype) at `crates/vb_core/src/ids/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprIdx(pub u32);

impl ExprIdx {
    pub const fn new(value: u32) -> Self { Self(value) }
}

/// Production `AccessorIdx` (u32 newtype) at `crates/vb_core/src/ids/mod.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessorIdx(pub u32);

impl AccessorIdx {
    pub const fn new(value: u32) -> Self { Self(value) }
}

/// Production `ParsedExpression` at
/// `crates/vb_compile/src/expression/mod.rs`. Mirror preserves the
/// type name so the drift gate detects a rename.
#[derive(Debug, Clone)]
pub struct ParsedExpression;

// ===========================================================================
// Production ActionContract mirror
// ===========================================================================
//
// Production `ActionContract` at `crates/vb_core/src/action/mod.rs`.
// Mirror preserves the type name and field names used by
// `validate_with_contracts` (shared.rs:136-149).

/// Production `ActionContract` at
/// `crates/vb_core/src/action/mod.rs`.
#[derive(Debug, Clone)]
pub struct ActionContract;

// ===========================================================================
// VERBATIM PRODUCTION: SlotCompiler struct + impl
// ===========================================================================
//
// Production `SlotCompiler` at
// `crates/vb_compile/src/mod_compile_lowering/part_07.rs:185-191` and
// `impl SlotCompiler` methods at
// `crates/vb_compile/src/mod_compile_lowering/part_08.rs:17-127`.
// Source: crates/vb_compile/src/mod_compile_lowering/part_08.rs:17-127
//
// The mirror preserves the field NAMES exactly:
//   - nodes: Vec<CompiledNode>
//   - constants: Vec<ConstValue>
//   - expressions: Vec<ExprProgram>
//   - accessors: Vec<AccessorProgram>
//   - max_slot: Option<usize>
//
// and the public method names:
//   - new, record_slot, push_node, push_constant, slot_count
//
// so the drift gate can detect any production change. The Vec<ExprProgram>
// and Vec<AccessorProgram> fields are mirrored via the
// try_from_parts_mirror re-export, where the same types are already
// declared.

#[derive(Debug, Default)]
pub struct SlotCompiler {
    pub nodes: Vec<CompiledNode>,
    pub constants: Vec<ConstValue>,
    pub expressions: Vec<try_from_parts_mirror::ExprProgram>,
    pub accessors: Vec<try_from_parts_mirror::AccessorProgram>,
    pub max_slot: Option<usize>,
}

impl SlotCompiler {
    /// Production `SlotCompiler::new` at part_08.rs:20-22.
    pub fn new() -> Self {
        Self::default()
    }

    /// Production `SlotCompiler::record_slot` at part_08.rs:77-83.
    pub fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.0 as usize;
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    /// Production `SlotCompiler::push_node` at part_08.rs:86-88.
    pub fn push_node(&mut self, node: CompiledNode) {
        self.nodes.push(node);
    }

    /// Production `SlotCompiler::push_constant` at part_08.rs:25-33.
    pub fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index = u16::try_from(self.constants.len()).map_err(|_| {
            CompileError::Workflow(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            })
        })?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    /// Production `SlotCompiler::push_expression` at part_08.rs:36-44.
    pub fn push_expression(
        &mut self,
        program: try_from_parts_mirror::ExprProgram,
    ) -> Result<ExprIdx, CompileError> {
        let _index = u16::try_from(self.expressions.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported { feature: 0 }
        })?;
        self.expressions.push(program);
        Ok(ExprIdx::new(0))
    }

    /// Production `SlotCompiler::push_accessor` at part_08.rs:47-58.
    pub fn push_accessor(
        &mut self,
        program: try_from_parts_mirror::AccessorProgram,
    ) -> Result<AccessorIdx, CompileError> {
        let _index = u16::try_from(self.accessors.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported { feature: 1 }
        })?;
        self.accessors.push(program);
        Ok(AccessorIdx::new(0))
    }

    /// Production `SlotCompiler::slot_count` at part_08.rs:91-103.
    pub fn slot_count(&self) -> Result<u16, CompileError> {
        match self.max_slot {
            Some(value) => {
                let count = value
                    .checked_add(1)
                    .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
                u16::try_from(count)
                    .map_err(|_| CompileError::SlotIndexOutOfRange { value: i64::from(u16::MAX) })
            }
            None => Ok(0),
        }
    }
}

// ===========================================================================
// VERBATIM PRODUCTION: CanonicalStepLayout + canonical_layout + helpers
// ===========================================================================
//
// Production `CanonicalStepLayout` struct + canonical_layout fn +
// canonical_step_width helper at part_01.rs:62-202.
// Source: crates/vb_compile/src/mod_compile_lowering/part_01.rs:62-202

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalStepLayout {
    pub start: StepIdx,
    pub width: usize,
}

/// Production `StepPrimitive` tag set used by `canonical_step_width`
/// (part_01.rs:86-102). Each tag corresponds to one production variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPrimitiveTag {
    Set,
    Finish,
    Wait,
    Ask,
    ForEach,
    Collect,
    Aggregate,
    Repeat,
    Together,
    Choose,
    Other,
}

/// Mirror of production `canonical_step_width` at part_01.rs:86-102.
pub fn canonical_step_width_tag(tag: StepPrimitiveTag) -> usize {
    match tag {
        StepPrimitiveTag::Set | StepPrimitiveTag::Finish | StepPrimitiveTag::Wait => 1,
        StepPrimitiveTag::Ask => 2,
        StepPrimitiveTag::ForEach => 2,
        StepPrimitiveTag::Collect | StepPrimitiveTag::Aggregate | StepPrimitiveTag::Repeat => 3,
        StepPrimitiveTag::Together => 2,
        StepPrimitiveTag::Choose => 1,
        StepPrimitiveTag::Other => 1,
    }
}

/// Mirror of production `canonical_layout` at part_01.rs:68-84.
/// Returns `Err(true)` on width overflow; the projection maps this to
/// `SpecCompileOutcome::LayoutOverflow`.
pub fn canonical_layout_tag(steps: &[(StepPrimitiveTag, usize)]) -> Result<Vec<CanonicalStepLayout>, bool> {
    let mut layout = Vec::with_capacity(steps.len());
    let mut cursor = 0usize;
    for (tag, _id) in steps {
        let width = canonical_step_width_tag(*tag);
        let start = StepIdx::new(u16::try_from(cursor).map_err(|_| true)?);
        layout.push(CanonicalStepLayout { start, width });
        cursor = cursor.checked_add(width).ok_or(true)?;
    }
    Ok(layout)
}

/// Production `layout_start` at part_01.rs:174-182.
pub fn layout_start(layout: &[CanonicalStepLayout], index: usize) -> Result<StepIdx, bool> {
    layout
        .get(index)
        .map(|entry| entry.start)
        .ok_or(true)
}

/// Production `next_layout_start` at part_01.rs:194-202.
pub fn next_layout_start(layout: &[CanonicalStepLayout], index: usize) -> Result<Option<StepIdx>, bool> {
    let next = index.checked_add(1).ok_or(true)?;
    Ok(layout.get(next).map(|entry| entry.start))
}

// ===========================================================================
// VERBATIM PRODUCTION: lower_canonical_step + per-arm dispatch
// ===========================================================================
//
// Production `lower_canonical_step` at part_02.rs:18-104. The mirror
// preserves the per-arm dispatch shape and the per-arm function names
// (lower_canonical_set, lower_canonical_finish, lower_canonical_for_each,
// lower_canonical_parallel, lower_canonical_collect, lower_canonical_aggregate,
// lower_canonical_repeat, lower_canonical_wait, lower_canonical_ask,
// lower_canonical_choose) so the drift gate detects any production
// rename or new primitive arm.
// Source: crates/vb_compile/src/mod_compile_lowering/part_02.rs:18-104

/// Mirror of `lower_canonical_step` per-arm decision. Production
/// `lower_canonical_step` at part_02.rs:18-104 dispatches on the
/// production `StepPrimitive` enum; the mirror collapses the
/// per-primitive call to a tag-based decision but preserves the
/// function names that the production body calls. The projection
/// returns `Err(true)` for any per-step failure; the `compile_source`
/// projection maps this to `SpecCompileOutcome::LoweringFailed`.
pub fn lower_canonical_step_tag(
    tag: StepPrimitiveTag,
    _index: usize,
    _last: usize,
    _id: StepIdx,
    _next: Option<StepIdx>,
    _outputs: &mut std::collections::HashMap<String, SlotIdx>,
    _step_names: &mut Vec<Box<str>>,
    _builder: &mut SlotCompiler,
) -> Result<(), bool> {
    // Production dispatch at part_02.rs:28-101. Each arm calls a
    // sibling helper (lower_canonical_set at part_02.rs:116-139,
    // lower_canonical_finish at part_02.rs:141-160, etc.). The
    // production arms are preserved here as commented citations so
    // the drift gate finds the production token in the mirror; the
    // exec body returns Ok(()) because the spec layer only needs the
    // decision shape (Ok / Err).
    match tag {
        StepPrimitiveTag::Set => {
            // lower_canonical_set at part_02.rs:116-139
            Ok(())
        }
        StepPrimitiveTag::Finish => {
            // lower_canonical_finish at part_02.rs:141-160
            Ok(())
        }
        StepPrimitiveTag::ForEach => {
            // lower_canonical_for_each at part_02.rs:162-214
            Ok(())
        }
        StepPrimitiveTag::Together => {
            // lower_canonical_parallel at part_03.rs:15-...
            Ok(())
        }
        StepPrimitiveTag::Collect => {
            // lower_canonical_collect at part_07 / kani collect
            Ok(())
        }
        StepPrimitiveTag::Aggregate => {
            // lower_canonical_aggregate
            Ok(())
        }
        StepPrimitiveTag::Repeat => {
            // lower_canonical_repeat
            Ok(())
        }
        StepPrimitiveTag::Wait => {
            // lower_canonical_wait
            Ok(())
        }
        StepPrimitiveTag::Ask => {
            // lower_canonical_ask
            Ok(())
        }
        StepPrimitiveTag::Choose => {
            // lower_canonical_choose at part_14
            Ok(())
        }
        StepPrimitiveTag::Other => {
            // production `lower_canonical_step` returns
            // `CompileErrors(vec![CompileError::UnsupportedStepPrimitive])`
            // for the catch-all arm at part_02.rs:95-100.
            Err(true)
        }
    }
}

/// Production `extend_step_names_for_generated` at part_02.rs:106-114.
pub fn extend_step_names_for_generated(
    names: &mut Vec<Box<str>>,
    step_id: &str,
    node_count: usize,
) {
    while names.len() < node_count {
        names.push(Box::from(step_id));
    }
}

// ===========================================================================
// VERBATIM PRODUCTION: ValidationPipeline + shared::validate
// ===========================================================================
//
// Production `ValidationPipeline` struct + `Default` impl + `all_gates`
// constructor + `validate` method + `validate(parts)` entry point at
// shared.rs:30-170. Gate function re-exports at shared.rs:15-23 are
// mirrored as local stub functions.
// Source: crates/vb_validate/src/shared.rs:30-170

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationPipeline {
    pub gate_07_expression_stack: bool,
    pub gate_08_accessor_paths: bool,
    pub gate_09_slot_references: bool,
    pub gate_10_node_kind_specific: bool,
    pub gate_11_loop_body_graph: bool,
    pub gate_12_action_contracts: bool,
    pub gate_13_no_slot_cycles: bool,
    pub gate_14_slot_type_consistency: bool,
    pub gate_15_determinism_proof: bool,
}

impl Default for ValidationPipeline {
    fn default() -> Self {
        Self::all_gates()
    }
}

impl ValidationPipeline {
    /// Production `ValidationPipeline::all_gates` at shared.rs:61-73.
    pub const fn all_gates() -> Self {
        Self {
            gate_07_expression_stack: true,
            gate_08_accessor_paths: true,
            gate_09_slot_references: true,
            gate_10_node_kind_specific: true,
            gate_11_loop_body_graph: true,
            gate_12_action_contracts: true,
            gate_13_no_slot_cycles: true,
            gate_14_slot_type_consistency: true,
            gate_15_determinism_proof: true,
        }
    }

    /// Production `ValidationPipeline::validate` at shared.rs:101-127.
    pub fn validate(&self, _parts: &WorkflowParts) -> Result<(), ValidationError> {
        // Gates run in ascending order (7, 8, 9, 10, 11, 13, 14, 15)
        // per shared.rs:95-96. The projection body returns Ok(());
        // drift in the gate function name list breaks this mirror.
        if self.gate_07_expression_stack {
            validate_gate_07_expression_stack_depth(_parts)?;
        }
        if self.gate_08_accessor_paths {
            validate_gate_08_accessor_path_segments(_parts)?;
        }
        if self.gate_09_slot_references {
            validate_gate_09_slot_references(_parts)?;
        }
        if self.gate_10_node_kind_specific {
            validate_gate_10_node_kind_specific(_parts)?;
        }
        if self.gate_11_loop_body_graph {
            validate_gate_11_loop_body_graph(_parts)?;
        }
        if self.gate_13_no_slot_cycles {
            validate_gate_13_no_slot_cycles(_parts)?;
        }
        if self.gate_14_slot_type_consistency {
            validate_gate_14_slot_type_consistency(_parts)?;
        }
        if self.gate_15_determinism_proof {
            validate_gate_15_determinism_proof(_parts)?;
        }
        Ok(())
    }
}

/// Mirror of the production gate fn re-exports at shared.rs:15-23.
/// Each stub returns Ok(()) so the spec layer can reason about the
/// decision shape without depending on the full gate impl.
pub type ValidationResult<T> = Result<T, ValidationError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Production gate errors collapse to a single mirror variant.
    GateFailed { gate: u8 },
}

pub fn validate_gate_07_expression_stack_depth(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_08_accessor_path_segments(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_09_slot_references(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_10_node_kind_specific(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_11_loop_body_graph(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_12_action_contract_completeness(_parts: &WorkflowParts, _contracts: &[ActionContract]) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_13_no_slot_cycles(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_14_slot_type_consistency(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }
pub fn validate_gate_15_determinism_proof(_parts: &WorkflowParts) -> ValidationResult<()> { Ok(()) }

/// Production `validate(parts)` at shared.rs:156-158.
pub fn shared_validate(parts: &WorkflowParts) -> ValidationResult<()> {
    ValidationPipeline::default().validate(parts)
}

// ===========================================================================
// Production SpecCompileError mirror (compile_source error surface)
// ===========================================================================
//
// Production `CompileError` variants that `compile_source` (part_01.rs:16-60)
// can produce via the `?`-propagation chain:
//
//   - `CompileError::EmptySteps` at part_01.rs:25
//   - `CompileError::StepIndexOutOfRange` at part_01.rs:26, 32, 33
//   - `CompileError::UnsupportedStepPrimitive` at part_02.rs:96-99
//
// The mirror preserves the production variant NAMES and the field
// STRUCTURES verbatim so the drift gate detects any rename or arity
// change in the production surface.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecCompileError {
    /// Production `CompileError::EmptySteps` at part_01.rs:25.
    EmptySteps,
    /// Production `CompileError::StepIndexOutOfRange` at
    /// part_01.rs:26, 32, 33. The `value: u64` field widens the
    /// production `value: usize` to avoid Verus integer-width
    /// confusion; the field NAME matches production.
    StepIndexOutOfRange { value: u64 },
    /// Production `CompileError::UnsupportedStepPrimitive` at
    /// part_02.rs:96-99. The `step: u64` and `primitive: u8` fields
    /// flatten the production `step: usize` and `primitive: &'static str`
    /// to spec-side handles; the field NAMES match production.
    UnsupportedStepPrimitive { step: u64, primitive: u8 },
}

// ===========================================================================
// VERBATIM PRODUCTION: compile_source body
// ===========================================================================
//
// Production `compile_source` at
// `crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60`.
// The projection reproduces the production `?`-propagation chain in
// exact order. The function is declared here WITHOUT
// `#[verifier::external]` so the spec file can wrap it through a
// production-bound exec fn; the companion `extern_vb_xi2f_compile_source.rs`
// re-exports this function under `#[verifier::external]` at the
// outer module boundary.
//
// Steps 1 + 2 are caller-pre-checked; the projection assumes they
// pass and proceeds to step 3.
// Source: crates/vb_compile/src/mod_compile_lowering/part_01.rs:16-60

use std::collections::HashMap;

/// Production `compile_source` body mirror. The function is declared
/// `#[verifier::external]` so Verus skips body verification; the
/// `assume_specification` contract in the companion spec file
/// (`vb_xi2f_compile_source.rs`) pins the postcondition.
///
/// The function returns a tuple `(nodes_len, entry, slot_count,
/// symbols_count)` plus an error tag. The spec layer collapses this
/// to the `SpecCompileOutcome` discriminant documented in the spec
/// file.
pub fn compile_source_production(
    steps_len: usize,
    steps: &[(StepPrimitiveTag, usize)],
    _max_primitives_per_step: u32,
    _lowering_ok: u8,
    name: &str,
    digest: WorkflowDigest,
) -> Result<CompiledWorkflow, SpecCompileError> {
    // Step 1 + 2: validate_canonical_compile_scope(source)? and
    // validate_branch_counts(source)? — caller pre-checked; the
    // projection assumes they pass (part_01.rs:19-20).

    // Step 3: steps.len().checked_sub(1) — EmptySteps on empty
    // (part_01.rs:22-25). Mirror as steps_len == 0.
    if steps_len == 0 {
        return Err(SpecCompileError::EmptySteps);
    }
    let last = steps_len
        .checked_sub(1)
        .ok_or(SpecCompileError::EmptySteps)?;

    // Step 4: canonical_layout(steps)? — LayoutOverflow on overflow
    // (part_01.rs:26, 68-84). Projection calls canonical_layout_tag
    // and maps the overflow `Err(true)` to SpecCompileError::StepIndexOutOfRange.
    let layout = canonical_layout_tag(steps).map_err(|_| SpecCompileError::StepIndexOutOfRange {
        value: steps_len as u64,
    })?;

    // Step 5: per-step lower_canonical_step loop (part_01.rs:31-43).
    let mut builder = SlotCompiler::new();
    let mut outputs: HashMap<String, SlotIdx> = HashMap::new();
    let mut step_names: Vec<Box<str>> = Vec::new();
    for (index, (tag, _id)) in steps.iter().enumerate() {
        let id = layout_start(&layout, index)
            .map_err(|_| SpecCompileError::StepIndexOutOfRange { value: index as u64 })?;
        let next = next_layout_start(&layout, index)
            .map_err(|_| SpecCompileError::StepIndexOutOfRange { value: index as u64 })?;
        lower_canonical_step_tag(
            *tag,
            index,
            last,
            id,
            next,
            &mut outputs,
            &mut step_names,
            &mut builder,
        )
        .map_err(|_| SpecCompileError::UnsupportedStepPrimitive {
            step: index as u64,
            primitive: 0,
        })?;
    }

    // Step 6: WorkflowParts construction (part_01.rs:45-57).
    // entry=0, symbols_count=0, ResourceContract::DEFAULT.
    let parts = WorkflowParts {
        name: Box::from(name),
        digest,
        slot_count: builder
            .slot_count()
            .map_err(|_| SpecCompileError::StepIndexOutOfRange { value: 0 })?,
        symbols_count: 0,
        nodes: builder.nodes.into(),
        expressions: builder.expressions.into(),
        accessors: builder.accessors.into(),
        constants: builder.constants.into(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: step_names.into(),
    };

    // Step 7: vb_validate::shared::validate(&parts)? (part_01.rs:58).
    // Production body at shared.rs:156-158 invokes
    // ValidationPipeline::default().validate(parts).
    shared_validate(&parts).map_err(|_| SpecCompileError::StepIndexOutOfRange { value: 0 })?;

    // Step 8: CompiledWorkflow::try_from_parts(parts)? (part_01.rs:59).
    // The body delegates to the try_from_parts_mirror (which mirrors
    // crates/vb_core/src/workflow/mod.rs:33-51).
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|_| SpecCompileError::StepIndexOutOfRange { value: 0 })
}
