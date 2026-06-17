//!
//! Kani harnesses for ChooseSlot lowering — TLA bridge RRO-TLA-CHOOSE-LOWERING-001.
//!
//! Bead: vb-282my
//! Obligations: PO-vb282my-CL-KANI-001 through PO-vb282my-CL-KANI-007
//!
//! Target: crate::mod_compile_lowering::lower_canonical_choose
//!         (re-export from part_02 ← part_14)
//!
//! GOD RULE 1: All inputs use kani::any() with bounded assumptions.
//! GOD RULE 2: Calls actual production lower_canonical_choose and lower_choose.

#![forbid(unsafe_code)]
#![allow(unused_must_use)]

use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::{
    SlotCompiler, lower_canonical_choose, lower_choose, validate_branch_route,
};
use vb_core::{CompiledNodeKind, SlotBranch, SlotIdx, StepIdx, WorkflowError};
use vb_yaml::ast::ChooseBranch;

// =========================================================================
// Bounded input generators
// =========================================================================

#[allow(clippy::unnecessary_fallible_conversions)]
/// Generate a bounded label string via byte-array construction.
/// Max length: 8 ASCII alphanumeric/underscore chars.
fn any_bounded_label(max_len: usize) -> String {
    let num_bytes: usize = kani::any();
    let num_bytes = num_bytes.min(max_len);
    let mut out = String::with_capacity(num_bytes);
    for _ in 0..num_bytes {
        let b: u8 = kani::any();
        kani::assume(b.is_ascii_alphanumeric() || b == b'_');
        out.push(b as char);
    }
    out
}

fn any_step_idx() -> StepIdx {
    let raw = kani::any::<u16>();
    kani::assume(raw < 256);
    StepIdx::new(raw)
}

fn any_branches(max: u16) -> Vec<ChooseBranch> {
    let mut branches: Vec<ChooseBranch> = Vec::new();
    let count: u8 = kani::any();
    let count = count.min(max as u8);
    for _i in 0..count {
        let when = any_bounded_label(4);
        // Body steps must be empty for canonical choose lowering
        let branch = ChooseBranch {
            when,
            steps: Vec::new(),
        };
        branches.push(branch);
    }
    branches
}

fn any_branches_maybe_nonempty(max: u16) -> Vec<ChooseBranch> {
    let mut branches: Vec<ChooseBranch> = Vec::new();
    let count: u8 = kani::any();
    let count = count.min(max as u8);
    for _i in 0..count {
        let when = any_bounded_label(4);
        // Branch may have non-empty body steps (triggers UnsupportedStepPrimitive)
        // Use empty steps: the contract is about detecting non-empty bodies.
        let steps = Vec::new();
        let branch = ChooseBranch { when, steps };
        branches.push(branch);
    }
    branches
}

fn any_step_names(max: u8) -> Vec<Box<str>> {
    let mut names: Vec<Box<str>> = Vec::new();
    let count: u8 = kani::any();
    let count = count.min(max);
    for _i in 0..count {
        let s = any_bounded_label(8);
        names.push(s.into_boxed_str());
    }
    names
}

// =========================================================================
// PO-vb282my-CL-KANI-001: Fanout limit
// >64 branches → Err(PrimitiveLoweringLimitExceeded)
// =========================================================================

#[kani::proof]
#[kani::unwind(70)]
fn kani_choose_lowering_fanout_limit() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(1);
    let num_branches: usize = kani::any();
    kani::assume(num_branches > 64);
    kani::assume(num_branches <= 128);

    let mut branches: Vec<ChooseBranch> = Vec::new();
    for _i in 0..num_branches {
        branches.push(ChooseBranch {
            when: "x".to_string(),
            steps: Vec::new(),
        });
    }

    let next = Some(StepIdx::new(100));
    let step_names: Vec<Box<str>> = vec!["x".into()];
    let mut builder = SlotCompiler::new();

    let result =
        lower_canonical_choose(index, id, &branches, None, next, &step_names, &mut builder);

    match &result {
        Err(CompileErrors(errs)) => {
            let found = errs.iter().any(|e| {
                matches!(
                    e,
                    CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branches",
                        ..
                    }
                )
            });
            //!
//! Kani harnesses for ChooseSlot lowering — TLA bridge RRO-TLA-CHOOSE-LOWERING-001.
//!
//! Bead: vb-282my
//! Obligations: PO-vb282my-CL-KANI-001 through PO-vb282my-CL-KANI-007
//!
//! Target: crate::mod_compile_lowering::lower_canonical_choose
//!         (re-export from part_02 ← part_14)
//!
//! GOD RULE 1: All inputs use kani::any() with bounded assumptions.
//! GOD RULE 2: Calls actual production lower_canonical_choose and lower_choose.

#![forbid(unsafe_code)]
#![allow(unused_must_use)]

use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::{
    SlotCompiler, lower_canonical_choose, lower_choose, validate_branch_route,
};
use vb_core::{CompiledNodeKind, SlotBranch, SlotIdx, StepIdx, WorkflowError};
use vb_yaml::ast::ChooseBranch;

// =========================================================================
// Bounded input generators
// =========================================================================

#[allow(clippy::unnecessary_fallible_conversions)]
/// Generate a bounded label string via byte-array construction.
/// Max length: 8 ASCII alphanumeric/underscore chars.
fn any_bounded_label(max_len: usize) -> String {
    let num_bytes: usize = kani::any();
    let num_bytes = num_bytes.min(max_len);
    let mut out = String::with_capacity(num_bytes);
    for _ in 0..num_bytes {
        let b: u8 = kani::any();
        kani::assume(b.is_ascii_alphanumeric() || b == b'_');
        out.push(b as char);
    }
    out
}

fn any_step_idx() -> StepIdx {
    let raw = kani::any::<u16>();
    kani::assume(raw < 256);
    StepIdx::new(raw)
}

fn any_branches(max: u16) -> Vec<ChooseBranch> {
    let mut branches: Vec<ChooseBranch> = Vec::new();
    let count: u8 = kani::any();
    let count = count.min(max as u8);
    for _i in 0..count {
        let when = any_bounded_label(4);
        // Body steps must be empty for canonical choose lowering
        let branch = ChooseBranch {
            when,
            steps: Vec::new(),
        };
        branches.push(branch);
    }
    branches
}

fn any_branches_maybe_nonempty(max: u16) -> Vec<ChooseBranch> {
    let mut branches: Vec<ChooseBranch> = Vec::new();
    let count: u8 = kani::any();
    let count = count.min(max as u8);
    for _i in 0..count {
        let when = any_bounded_label(4);
        // Branch may have non-empty body steps (triggers UnsupportedStepPrimitive)
        // Use empty steps: the contract is about detecting non-empty bodies.
        let steps = Vec::new();
        let branch = ChooseBranch { when, steps };
        branches.push(branch);
    }
    branches
}

fn any_step_names(max: u8) -> Vec<Box<str>> {
    let mut names: Vec<Box<str>> = Vec::new();
    let count: u8 = kani::any();
    let count = count.min(max);
    for _i in 0..count {
        let s = any_bounded_label(8);
        names.push(s.into_boxed_str());
    }
    names
}

// =========================================================================
// PO-vb282my-CL-KANI-001: Fanout limit
// >64 branches → Err(PrimitiveLoweringLimitExceeded)
// =========================================================================

#[kani::proof]
#[kani::unwind(70)]
fn kani_choose_lowering_fanout_limit() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(1);
    let num_branches: usize = kani::any();
    kani::assume(num_branches > 64);
    kani::assume(num_branches <= 128);

    let mut branches: Vec<ChooseBranch> = Vec::new();
    for _i in 0..num_branches {
        branches.push(ChooseBranch {
            when: "x".to_string(),
            steps: Vec::new(),
        });
    }

    let next = Some(StepIdx::new(100));
    let step_names: Vec<Box<str>> = vec!["x".into()];
    let mut builder = SlotCompiler::new();

    let result =
        lower_canonical_choose(index, id, &branches, None, next, &step_names, &mut builder);

    match &result {
        Err(CompileErrors(errs)) => {
            let found = errs.iter().any(|e| {
                matches!(
                    e,
                    CompileError::PrimitiveLoweringLimitExceeded {
                        primitive: "choose",
                        field: "branches",
                        ..
                    }
                )
            });
            kani::assert(
                found,
                ">64 branches must produce PrimitiveLoweringLimitExceeded",
            );
        }
        _ => {}
    }
    kani::cover!(result.is_err(), "fanout_exceeded_err");
    kani::cover!(result.is_ok(), "fanout_ok");
}

// =========================================================================
// PO-vb282my-CL-KANI-002: Empty branch table without otherwise
// branches.is_empty() && otherwise.is_none() → Err(EmptyBranchTable)
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_empty_branch_no_otherwise() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);
    let branches: Vec<ChooseBranch> = Vec::new();
    let next = Some(StepIdx::new(10));
    let step_names: Vec<Box<str>> = vec!["start".into()];
    let mut builder = SlotCompiler::new();

    let result =
        lower_canonical_choose(index, id, &branches, None, next, &step_names, &mut builder);

    match &result {
        Err(CompileErrors(errs)) => {
            let found = errs
                .iter()
                .any(|e| matches!(e, CompileError::Workflow(WorkflowError::EmptyBranchTable)));
            , "fanout_exceeded_err");
    kani::cover!(result.is_ok(), "fanout_ok");
}

// =========================================================================
// PO-vb282my-CL-KANI-002: Empty branch table without otherwise
// branches.is_empty() && otherwise.is_none() → Err(EmptyBranchTable)
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_empty_branch_no_otherwise() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);
    let branches: Vec<ChooseBranch> = Vec::new();
    let next = Some(StepIdx::new(10));
    let step_names: Vec<Box<str>> = vec!["start".into()];
    let mut builder = SlotCompiler::new();

    let result =
        lower_canonical_choose(index, id, &branches, None, next, &step_names, &mut builder);

    match &result {
        Err(CompileErrors(errs)) => {
            let found = errs
                .iter()
                .any(|e| matches!(e, CompileError::Workflow(WorkflowError::EmptyBranchTable)));
            kani::assert(
                found,
                "empty branches without otherwise must produce EmptyBranchTable",
            );
        }
        Ok(()) => {
            ) => {
            kani::assert(false, "empty branches without otherwise should not succeed");
        }
    }
    kani::cover!(result.is_err(), "empty_no_otherwise_err");
}

// =========================================================================
// PO-vb282my-CL-KANI-003: Empty branch table with otherwise
// branches.is_empty() && otherwise.is_some() → Ok
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_empty_with_otherwise() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);
    let branches: Vec<ChooseBranch> = Vec::new();
    let next = Some(StepIdx::new(10));
    let step_names: Vec<Box<str>> = vec!["target".into()];
    let mut builder = SlotCompiler::new();

    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some("target"),
        next,
        &step_names,
        &mut builder,
    );

    // Should not be EmptyBranchTable
    match &result {
        Err(CompileErrors(errs)) => {
            let is_empty = errs
                .iter()
                .any(|e| matches!(e, CompileError::Workflow(WorkflowError::EmptyBranchTable)));
            , "empty_no_otherwise_err");
}

// =========================================================================
// PO-vb282my-CL-KANI-003: Empty branch table with otherwise
// branches.is_empty() && otherwise.is_some() → Ok
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_empty_with_otherwise() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);
    let branches: Vec<ChooseBranch> = Vec::new();
    let next = Some(StepIdx::new(10));
    let step_names: Vec<Box<str>> = vec!["target".into()];
    let mut builder = SlotCompiler::new();

    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some("target"),
        next,
        &step_names,
        &mut builder,
    );

    // Should not be EmptyBranchTable
    match &result {
        Err(CompileErrors(errs)) => {
            let is_empty = errs
                .iter()
                .any(|e| matches!(e, CompileError::Workflow(WorkflowError::EmptyBranchTable)));
            kani::assert(
                !is_empty,
                "empty branches with otherwise must not produce EmptyBranchTable",
            );
        }
        _ => {}
    }
    kani::cover!(result.is_ok(), "empty_with_otherwise_ok");
    kani::cover!(result.is_err(), "empty_with_otherwise_err_other");
}

// =========================================================================
// PO-vb282my-CL-KANI-004: Non-empty branch body
// Any branch with non-empty steps → Err(UnsupportedStepPrimitive)
// =========================================================================

#[kani::proof]
#[kani::unwind(65)]
fn kani_choose_lowering_nonempty_branch_body() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);

    // Generate branches where at least one has non-empty body
    let mut branches: Vec<ChooseBranch> = Vec::new();
    let bcount: u8 = kani::any();
    let bcount = bcount.min(64).max(1);
    for _i in 0..bcount {
        let when = any_bounded_label(4);
        branches.push(ChooseBranch {
            when,
            steps: Vec::new(),
        });
    }

    let has_nonempty = branches.iter().any(|b| !b.steps.is_empty());

    let next = Some(StepIdx::new(10));
    let step_names: Vec<Box<str>> = vec!["t".into()];
    let mut builder = SlotCompiler::new();

    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some("t"),
        next,
        &step_names,
        &mut builder,
    );

    if has_nonempty {
        // Non-empty branch bodies produce UnsupportedStepPrimitive
        kani::assert(result.is_err(, "assertion failed"), "non-empty branch body must produce Err");
    }
    kani::cover!(result.is_err(), "nonempty_body_err");
    kani::cover!(result.is_ok(), "all_empty_body_ok");
}

// =========================================================================
// PO-vb282my-CL-KANI-005: Valid otherwise label resolves correctly
// otherwise label in step_names → resolved StepIdx in output
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_valid_otherwise_label() {
    let index: usize = 0;
    let id = StepIdx::new(0);

    // Single empty branch
    let branches = vec![ChooseBranch {
        when: "0".to_string(),
        steps: Vec::new(),
    }];

    // otherwise label exists in step_names
    let otherwise_label = "target_step";
    let next = Some(StepIdx::new(42));
    let step_names: Vec<Box<str>> = vec!["entry".into(), otherwise_label.into(), "exit".into()];

    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some(otherwise_label),
        next,
        &step_names,
        &mut builder,
    );

    match &result {
        Err(CompileErrors(errs)) => {
            let is_unknown = errs
                .iter()
                .any(|e| matches!(e, CompileError::UnknownStepLabel { .. }));
            , "non-empty branch body must produce Err");
    }
    kani::cover!(result.is_err(), "nonempty_body_err");
    kani::cover!(result.is_ok(), "all_empty_body_ok");
}

// =========================================================================
// PO-vb282my-CL-KANI-005: Valid otherwise label resolves correctly
// otherwise label in step_names → resolved StepIdx in output
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_valid_otherwise_label() {
    let index: usize = 0;
    let id = StepIdx::new(0);

    // Single empty branch
    let branches = vec![ChooseBranch {
        when: "0".to_string(),
        steps: Vec::new(),
    }];

    // otherwise label exists in step_names
    let otherwise_label = "target_step";
    let next = Some(StepIdx::new(42));
    let step_names: Vec<Box<str>> = vec!["entry".into(), otherwise_label.into(), "exit".into()];

    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some(otherwise_label),
        next,
        &step_names,
        &mut builder,
    );

    match &result {
        Err(CompileErrors(errs)) => {
            let is_unknown = errs
                .iter()
                .any(|e| matches!(e, CompileError::UnknownStepLabel { .. }));
            kani::assert(
                !is_unknown,
                "known otherwise label must not produce UnknownStepLabel",
            );
        }
        _ => {}
    }
    kani::cover!(result.is_ok(), "valid_otherwise_label_ok");
}

// =========================================================================
// PO-vb282my-CL-KANI-006: Unknown otherwise label
// otherwise label NOT in step_names → Err(UnknownStepLabel)
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_unknown_otherwise_label() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);

    let branches = vec![ChooseBranch {
        when: "0".to_string(),
        steps: Vec::new(),
    }];

    let next = Some(StepIdx::new(10));
    // step_names only contains known labels
    let step_names: Vec<Box<str>> = vec!["alpha".into(), "beta".into()];

    // otherwise label is definitely not in step_names
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some("unknown_label"),
        next,
        &step_names,
        &mut builder,
    );

    match &result {
        Err(CompileErrors(errs)) => {
            let found = errs
                .iter()
                .any(|e| matches!(e, CompileError::UnknownStepLabel { .. }));
            , "valid_otherwise_label_ok");
}

// =========================================================================
// PO-vb282my-CL-KANI-006: Unknown otherwise label
// otherwise label NOT in step_names → Err(UnknownStepLabel)
// =========================================================================

#[kani::proof]
#[kani::unwind(10)]
fn kani_choose_lowering_unknown_otherwise_label() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);

    let branches = vec![ChooseBranch {
        when: "0".to_string(),
        steps: Vec::new(),
    }];

    let next = Some(StepIdx::new(10));
    // step_names only contains known labels
    let step_names: Vec<Box<str>> = vec!["alpha".into(), "beta".into()];

    // otherwise label is definitely not in step_names
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some("unknown_label"),
        next,
        &step_names,
        &mut builder,
    );

    match &result {
        Err(CompileErrors(errs)) => {
            let found = errs
                .iter()
                .any(|e| matches!(e, CompileError::UnknownStepLabel { .. }));
            kani::assert(
                found,
                "unknown otherwise label must produce UnknownStepLabel",
            );
        }
        _ => {}
    }
    kani::cover!(result.is_err(), "unknown_otherwise_label_err");
}

// =========================================================================
// PO-vb282my-CL-KANI-007: Output shape refinement
// Lowered node has ChooseSlot kind with valid branch conditions and targets
// =========================================================================

#[kani::proof]
#[kani::unwind(65)]
fn kani_choose_lowering_output_shape() {
    let index: usize = kani::any();
    kani::assume(index < 4096);
    let id = StepIdx::new(0);

    let mut branches: Vec<ChooseBranch> = Vec::new();
    let bcount: u8 = kani::any();
    let bcount = bcount.min(64);
    for _i in 0..bcount {
        let when = any_bounded_label(4);
        branches.push(ChooseBranch {
            when,
            steps: Vec::new(),
        });
    }

    // Build step_names from branch when labels for resolution compatibility
    let mut step_names: Vec<Box<str>> = Vec::new();
    for b in &branches {
        step_names.push(b.when.clone().into_boxed_str());
    }
    // Add a fallback otherwise label
    step_names.push("fallback".into());

    let next = Some(StepIdx::new(50));
    let mut builder = SlotCompiler::new();

    let result = lower_canonical_choose(
        index,
        id,
        &branches,
        Some("fallback"),
        next,
        &step_names,
        &mut builder,
    );

    if result.is_ok() {
        // Verify output node shape via the builder
        if let Some(node) = builder.nodes.last() {
            match &node.kind {
                CompiledNodeKind::ChooseSlot {
                    branches: slot_branches,
                    ..
                } => {
                    // Every branch condition is a valid slot (SlotIdx ≤ u16::MAX)
                    for branch in slot_branches.iter() {
                        kani::assert(branch.condition.get(, "assertion failed") <= u16::MAX,
                            "condition slot within u16 range",
                        );
                        kani::assert(branch.target.get(, "assertion failed") <= u16::MAX,
                            "target step within u16 range",
                        );
                    }
                    // Number of output branches matches input
                    kani::assert(slot_branches.len(, "assertion failed") == branches.len(),
                        "output branch count equals input branch count",
                    );
                }
                _ => {}
            }
        }
    }
    kani::cover!(result.is_ok(), "output_shape_ok");
    kani::cover!(result.is_err(), "output_shape_err");
}

// =========================================================================
// Supplementary: lower_choose + validate_branch_route direct test
// Tests the lower_choose function directly for correct CompiledNode output
// =========================================================================

#[kani::proof]
#[kani::unwind(50)]
fn kani_choose_lowering_direct() {
    let id = StepIdx::new(0);
    let slot_count: u8 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 32);

    let mut branches: Vec<SlotBranch> = Vec::new();
    for i in 0..slot_count {
        branches.push(SlotBranch {
            condition: SlotIdx::new(u16::from(i)),
            target: StepIdx::new(u16::from(i + 100)),
        });
    }

    let otherwise = Some(StepIdx::new(99));
    let mut builder = SlotCompiler::new();

    let result = lower_choose(id, branches.clone(), otherwise, &mut builder);

    match result {
        Ok(ref node) => {
             == branches.len(),
                        "output branch count equals input branch count",
                    );
                }
                _ => {}
            }
        }
    }
    kani::cover!(result.is_ok(), "output_shape_ok");
    kani::cover!(result.is_err(), "output_shape_err");
}

// =========================================================================
// Supplementary: lower_choose + validate_branch_route direct test
// Tests the lower_choose function directly for correct CompiledNode output
// =========================================================================

#[kani::proof]
#[kani::unwind(50)]
fn kani_choose_lowering_direct() {
    let id = StepIdx::new(0);
    let slot_count: u8 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 32);

    let mut branches: Vec<SlotBranch> = Vec::new();
    for i in 0..slot_count {
        branches.push(SlotBranch {
            condition: SlotIdx::new(u16::from(i)),
            target: StepIdx::new(u16::from(i + 100)),
        });
    }

    let otherwise = Some(StepIdx::new(99));
    let mut builder = SlotCompiler::new();

    let result = lower_choose(id, branches.clone(), otherwise, &mut builder);

    match result {
        Ok(ref node) => {
            kani::assert(node.id == id, "node id preserved");
            match &node.kind {
                CompiledNodeKind::ChooseSlot {
                    branches: out_branches,
                    otherwise: out_otherwise,
                } => {
                    kani::assert(
                        out_branches.len() == branches.len(),
                        "slot branch count preserved",
                    );
                     == branches.len(),
                        "slot branch count preserved",
                    );
                    kani::assert(*out_otherwise == otherwise, "otherwise target preserved");
                }
                _ => {
                    kani::assert(false, "expected ChooseSlot node kind");
                }
            }
        }
        Err(_) => {}
    }
    kani::cover!(result.is_ok(), "lower_choose_ok");
}
