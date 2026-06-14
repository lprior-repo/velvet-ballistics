// Verification artifact: choose width/fanout Kani harnesses.
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// GOD RULE 1: Uses kani::any() with bounded assumptions.
// GOD RULE 2: Binds to production choose_width/lower_canonical_choose.

#![forbid(unsafe_code)]

use super::{make_choose_branch, make_set_step};
use crate::mod_compile_lowering::SlotCompiler;
use crate::mod_compile_lowering::part_01::choose_width;
use crate::mod_compile_lowering::part_02::lower_canonical_choose;
use vb_core::ids::StepIdx;

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_width_parity() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count <= 16);

    let mut expected_width = 1usize;
    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        let body_steps: u8 = kani::any();
        kani::assume(body_steps <= 5);
        let mut steps = Vec::new();
        for j in 0..body_steps {
            steps.push(make_set_step(
                &format!("s{i}{j}"),
                &format!("o{i}{j}"),
                "42",
            ));
        }
        expected_width = expected_width.wrapping_add(usize::from(body_steps));
        branches.push(make_choose_branch(&format!("{i}"), steps));
    }

    match choose_width(&branches) {
        Ok(width) => {
            kani::assert(width >= 1, "choose_width must be >= 1");
            kani::assert(
                width == expected_width,
                "choose_width must equal 1 + sum of body step counts",
            );
        }
        Err(_) => kani::assert(false, "unexpected error in choose_width"),
    }
}

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_width_overflow() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count <= 64);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        let body_steps: u8 = kani::any();
        kani::assume(body_steps <= 10);
        let mut steps = Vec::new();
        for _ in 0..body_steps {
            steps.push(make_set_step("s", "o", "1"));
        }
        branches.push(make_choose_branch(&format!("{i}"), steps));
    }

    match choose_width(&branches) {
        Ok(width) => kani::assert(width >= 1, "valid width must be >= 1"),
        Err(_) => {},
    }
}

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_fanout() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count <= 128);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        branches.push(make_choose_branch(&format!("{i}"), vec![]));
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

    if branch_count > 64 {
        kani::assert(result.is_err(), ">64 branches must be rejected");
    } else {
        kani::assert(result.is_ok(), "0..64 branches with otherwise must succeed");
    }
}

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_otherwise_span() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1 && branch_count <= 4);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    let mut total_body_steps = 0usize;
    for i in 0..branch_count {
        let body_steps: u8 = kani::any();
        kani::assume(body_steps <= 3);
        total_body_steps = total_body_steps.wrapping_add(usize::from(body_steps));
        let steps: Vec<_> = (0..body_steps)
            .map(|j| make_set_step(&format!("s{i}{j}"), &format!("o{i}{j}"), "1"))
            .collect();
        branches.push(make_choose_branch(&format!("{i}"), steps));
    }

    if let Ok(width) = choose_width(&branches) {
        kani::assert(
            width == 1usize.wrapping_add(total_body_steps),
            "choose_width must match 1 + sum of body step counts",
        );
    }
}
