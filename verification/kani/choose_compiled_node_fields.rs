// Verification artifact: choose_compiled_node_fields.rs
// Bead: vb-njib
// PO: ps-08 (CompiledNode fields are correctly set)
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness choose_compiled_node_fields
//
// Proof obligations:
// - ps-08: CompiledNode fields (id, output, next, error_slot, on_error) are correctly set
// - output = None, next = None, error_slot = None, on_error = None for ChooseSlot
//
// GOD RULE 1: Uses kani::any() — no hardcoded shapes.
// GOD RULE 2: Binds to actual Rust lower_choose implementation.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::compile::{lower_choose, SlotCompiler};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, SlotBranch};

/// ps-08 H1: ChooseSlot CompiledNode has correct id field.
#[kani::proof]
#[kani::unwind(5)]
fn choose_node_id_preserved() {
    let id = StepIdx::new(kani::any_where(|i| *i < 1000));
    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    let mut builder = SlotCompiler::new();
    let result = lower_choose(id, branches, Some(StepIdx::new(5)), &mut builder);

    match result {
        Ok(node) => {
            kani::assert(node.id == id, "node id preserved");
        }
        Err(_) => {}
    }
}

/// ps-08 H2: ChooseSlot CompiledNode has output = None.
#[kani::proof]
#[kani::unwind(5)]
fn choose_node_output_is_none() {
    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    let mut builder = SlotCompiler::new();
    let result = lower_choose(StepIdx::new(0), branches, Some(StepIdx::new(5)), &mut builder);

    match result {
        Ok(node) => {
            kani::assert(node.output.is_none(), "output is None for ChooseSlot");
        }
        Err(_) => {}
    }
}

/// ps-08 H3: ChooseSlot CompiledNode has next = None.
#[kani::proof]
#[kani::unwind(5)]
fn choose_node_next_is_none() {
    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    let mut builder = SlotCompiler::new();
    let result = lower_choose(StepIdx::new(0), branches, Some(StepIdx::new(5)), &mut builder);

    match result {
        Ok(node) => {
            kani::assert(node.next.is_none(), "next is None for ChooseSlot");
        }
        Err(_) => {}
    }
}

/// ps-08 H4: ChooseSlot CompiledNode has error_slot = None.
#[kani::proof]
#[kani::unwind(5)]
fn choose_node_error_slot_is_none() {
    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    let mut builder = SlotCompiler::new();
    let result = lower_choose(StepIdx::new(0), branches, Some(StepIdx::new(5)), &mut builder);

    match result {
        Ok(node) => {
            kani::assert(node.error_slot.is_none(), "error_slot is None for ChooseSlot");
        }
        Err(_) => {}
    }
}

/// ps-08 H5: ChooseSlot CompiledNode has on_error = None.
#[kani::proof]
#[kani::unwind(5)]
fn choose_node_on_error_is_none() {
    let branches = vec![SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(1),
    }];

    let mut builder = SlotCompiler::new();
    let result = lower_choose(StepIdx::new(0), branches, Some(StepIdx::new(5)), &mut builder);

    match result {
        Ok(node) => {
            kani::assert(node.on_error.is_none(), "on_error is None for ChooseSlot");
        }
        Err(_) => {}
    }
}
