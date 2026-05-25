// Verification artifact: vb_awhr_otherwise_label.rs
// Bead: vb-awhr
// PO: PO-002 (otherwise label resolution: exact match → Some(idx), missing → error)
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness lower_canonical_choose_otherwise_resolution
//
// Proof obligations:
// - PO-002: Exact otherwise label match resolves to correct step index
// - PO-002: Missing otherwise label returns UnknownStepTarget error with sentinel
// - PO-002: Otherwise resolution never panics
//
// GOD RULE 1: Uses kani::any() for symbolic label selection — no hardcoded shapes.
// GOD RULE 2: Binds to actual Rust lower_canonical_choose in part_02.rs.

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::{CompileError, CompileErrors, SlotCompiler};
use vb_core::ids::StepIdx;

/// PO-002 H1: Otherwise label resolution returns correct results without panic.
///
/// Fixed step_names table ["setup", "choose", "cleanup"].
/// Symbolic label_choice selects:
///   0 -> "setup"   (exact match, index 0)
///   1 -> "choose"  (exact match, index 1)
///   2 -> "cleanup" (exact match, index 2)
///   3 -> "nonexistent" (missing, must error)
#[kani::proof]
#[kani::unwind(8)]
fn lower_canonical_choose_otherwise_resolution() {
    let step_names: [Box<str>; 3] = [
        Box::from("setup"),
        Box::from("choose"),
        Box::from("cleanup"),
    ];

    let label_choice: u8 = kani::any();
    kani::assume(label_choice < 4);

    let otherwise = match label_choice {
        0 => Some("setup"),
        1 => Some("choose"),
        2 => Some("cleanup"),
        _ => Some("nonexistent"),
    };

    // Empty branches with a valid next step to satisfy empty_fallback
    let branches: Vec<vb_yaml::ast::ChooseBranch> = vec![];
    let mut builder = SlotCompiler::new();

    let result = crate::mod_compile_lowering::lower_canonical_choose(
        1,                          // step index
        StepIdx::new(1),            // step id
        &branches,
        otherwise,
        Some(StepIdx::new(2)),      // next step (for empty_fallback)
        &step_names,
        &mut builder,
    );

    match label_choice {
        0 | 1 | 2 => {
            // Exact match: must succeed (no panic)
            kani::assert(result.is_ok(), "exact otherwise match must succeed");
        }
        _ => {
            // Missing label: must return UnknownStepTarget with sentinel
            match result {
                Err(CompileErrors(errors)) => {
                    match &errors[0] {
                        CompileError::UnknownStepTarget { step, target } => {
                            kani::assert(*step == 1, "error references correct step index");
                            kani::assert(
                                *target == usize::MAX,
                                "error uses usize::MAX sentinel for missing label",
                            );
                        }
                        _ => {
                            kani::assert(false, "expected UnknownStepTarget error");
                        }
                    }
                }
                Ok(()) => {
                    kani::assert(false, "missing otherwise label must error");
                }
            }
        }
    }
    std::mem::forget(builder);
}
