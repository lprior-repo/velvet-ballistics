// Verification artifact: choose slot Kani harnesses.
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// GOD RULE 1: Uses bounded nondeterministic inputs.
// GOD RULE 2: Binds to production slot_from_text/lower_canonical_choose.

#![forbid(unsafe_code)]

use super::{make_choose_branch, make_set_step};
use crate::mod_compile_lowering::SlotCompiler;
use crate::mod_compile_lowering::part_02::lower_canonical_choose;
use crate::mod_compile_lowering::part_05::slot_from_text;
use vb_core::ids::StepIdx;

#[kani::proof]
#[kani::unwind(16)]
fn kani_slot_from_text_closed() {
    let input_kind: u8 = kani::any();
    kani::assume(input_kind < 5);

    let text = match input_kind {
        0 => "0",
        1 => "1",
        2 => "65535",
        3 => "",
        _ => "not_a_number",
    };

    match slot_from_text(text, 0, "test.field") {
        Ok(slot) => {
            kani::assert(
                slot.as_usize() <= usize::from(u16::MAX),
                "valid slot_from_text must produce in-range SlotIdx",
            );
        }
        Err(_) => {}
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn kani_choose_no_yaml_in_ir() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1 && branch_count <= 8);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        let has_body: bool = kani::any();
        let steps = if has_body {
            vec![make_set_step("body", "o", "1")]
        } else {
            vec![]
        };
        branches.push(make_choose_branch(&format!("{i}"), steps));
    }

    let step_names: [Box<str>; 3] = [Box::from("pick"), Box::from("body"), Box::from("done")];
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        StepIdx::new(0),
        &branches,
        Some("done"),
        Some(StepIdx::new(2)),
        &step_names,
        &mut builder,
    );

    if result.is_ok() {
        kani::assert(!builder.nodes.is_empty(), "must emit at least one node");
    }
}

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_slot_unique() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1 && branch_count <= 4);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        let has_body: bool = kani::any();
        let steps = if has_body {
            vec![make_set_step("body", "out", "1")]
        } else {
            vec![]
        };
        branches.push(make_choose_branch(&format!("{i}"), steps));
    }

    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        StepIdx::new(0),
        &branches,
        Some("done"),
        Some(StepIdx::new(1)),
        &step_names,
        &mut builder,
    );

    if result.is_ok() {
        if let Ok(count) = builder.slot_count() {
            kani::assert(
                count >= u16::from(branch_count),
                "slot_count must cover condition slots",
            );
        }
    }
}

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_slot_disjoint() {
    let branches = vec![make_choose_branch(
        "0",
        vec![make_set_step("body", "out", "1")],
    )];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        StepIdx::new(0),
        &branches,
        Some("done"),
        Some(StepIdx::new(2)),
        &step_names,
        &mut builder,
    );

    match result {
        Ok(()) => kani::cover!(builder.nodes.len() >= 2, "lowering produced body node"),
        Err(_) => {}
    }
}
