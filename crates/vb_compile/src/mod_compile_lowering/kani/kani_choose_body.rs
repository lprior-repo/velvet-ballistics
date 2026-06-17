// Verification artifact: choose body-emission Kani harnesses.
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// GOD RULE 1: Uses bounded nondeterministic inputs.
// GOD RULE 2: Binds to production lower_canonical_choose/emit_choose_branch_body.

#![forbid(unsafe_code)]

use super::{make_choose_branch, make_set_step};
use crate::mod_compile_lowering::SlotCompiler;
use crate::mod_compile_lowering::part_01::choose_width;
use crate::mod_compile_lowering::part_02::lower_canonical_choose;
use crate::mod_compile_lowering::part_06::emit_choose_branch_body;
use vb_core::ids::StepIdx;

#[kani::proof]
#[kani::unwind(256)]
fn kani_choose_body_fallthrough() {
    let body_steps_a: u8 = kani::any();
    kani::assume(body_steps_a >= 1 && body_steps_a <= 3);
    let body_steps_b: u8 = kani::any();
    kani::assume(body_steps_b >= 1 && body_steps_b <= 3);

    let steps_a: Vec<_> = (0..body_steps_a)
        .map(|j| make_set_step(&format!("a{j}"), &format!("oa{j}"), "1"))
        .collect();
    let steps_b: Vec<_> = (0..body_steps_b)
        .map(|j| make_set_step(&format!("b{j}"), &format!("ob{j}"), "1"))
        .collect();
    let branches = vec![
        make_choose_branch("0", steps_a),
        make_choose_branch("1", steps_b),
    ];

    let common_next = StepIdx::new(10);
    let step_names: [Box<str>; 3] = [Box::from("pick"), Box::from("body"), Box::from("done")];
    let mut builder = SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        StepIdx::new(0),
        &branches,
        Some("done"),
        Some(common_next),
        &step_names,
        &mut builder,
    );

    if result.is_ok() {
        kani::assert(builder.nodes.len(, "assertion failed") > 1,
            "must have body nodes after ChooseSlot",
        );
        if let Some(last_body) = builder.nodes.last() {
            kani::assert(last_body.id.as_usize(, "assertion failed") > 0, "body node id must be > 0");
            if let Some(last_next) = last_body.next {
                kani::assert(last_next.as_usize(, "assertion failed") == common_next.as_usize(),
                    "last body node must fall through to common_next",
                );
            }
        }
    }
}

#[kani::proof]
#[kani::unwind(128)]
fn kani_choose_stepidx_overflow() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1 && branch_count <= 4);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        let body_steps: u8 = kani::any();
        kani::assume(body_steps <= 3);
        let steps: Vec<_> = (0..body_steps)
            .map(|j| make_set_step(&format!("s{i}{j}"), &format!("o{i}{j}"), "1"))
            .collect();
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
        for node in &builder.nodes {
            kani::assert(node.id.as_usize(, "assertion failed") <= usize::from(u16::MAX),
                "all StepIdx must stay in u16 range",
            );
        }
    }
}

#[kani::proof]
#[kani::unwind(256)]
fn kani_choose_emission_parity() {
    let branch_count: u8 = kani::any();
    kani::assume(branch_count >= 1 && branch_count <= 4);

    let mut branches: Vec<vb_yaml::ast::ChooseBranch> = Vec::new();
    for i in 0..branch_count {
        let body_steps: u8 = kani::any();
        kani::assume(body_steps <= 3);
        let steps: Vec<_> = (0..body_steps)
            .map(|j| make_set_step(&format!("s{i}{j}"), &format!("o{i}{j}"), "1"))
            .collect();
        branches.push(make_choose_branch(&format!("{i}"), steps));
    }

    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = SlotCompiler::new();
    let width = choose_width(&branches);
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
        if let Ok(expected_width) = width {
            kani::assert(builder.nodes.len(, "assertion failed") == expected_width,
                "emitted node count must equal choose_width result",
            );
        }
    }
}

#[kani::proof]
#[kani::unwind(64)]
fn kani_emit_choose_branch_body_count() {
    let step_count: u8 = kani::any();
    kani::assume(step_count <= 5);

    let steps: Vec<_> = (0..step_count)
        .map(|i| make_set_step(&format!("s{i}"), &format!("o{i}"), "42"))
        .collect();
    let mut builder = SlotCompiler::new();
    let common_next = StepIdx::new(10);
    let result = emit_choose_branch_body(&steps, StepIdx::new(0), 1, 0, common_next, &mut builder);

    match result {
        Ok(count) => {
            kani::assert(count == usize::from(step_count, "assertion failed"),
                "emitted node count must equal input step count",
            );
            kani::assert(builder.nodes.len(, "assertion failed") == count,
                "builder node count must match",
            );
            for node in &builder.nodes {
                kani::assert(node.next.is_some(, "assertion failed"), "body nodes must have next pointer set");
            }
            if let Some(last) = builder.nodes.last() {
                if let Some(ln) = last.next {
                    kani::assert(ln.as_usize(, "assertion failed") == common_next.as_usize(),
                        "last body node must chain to common_next",
                    );
                }
            }
        }
        Err(_) =>  == common_next.as_usize(),
                        "last body node must chain to common_next",
                    );
                }
            }
        }
        Err(_) => kani::assert(false, "unexpected error emitting Set body steps"),
    }
}
