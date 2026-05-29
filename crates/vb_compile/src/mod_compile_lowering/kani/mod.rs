// Verification artifacts: Kani harnesses for choose lowering fix.
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO-KANI-001 through PO-KANI-013
//
// GOD RULE 1: Uses kani::any() with bounded assumptions for exhaustive
//   input generation. No hardcoded structural inputs.
// GOD RULE 2: Binds to actual production choose_width, lower_canonical_choose,
//   slot_from_text, and emit_choose_branch_body in the module.
// GOD RULE 4: Unwinding bounds are documented per harness.

#![cfg(kani)]

mod kani_choose_body;
mod kani_choose_slots;
mod kani_choose_width;

fn make_set_step(id: &str, output: &str, value: &str) -> vb_yaml::ast::StepAst {
    vb_yaml::ast::StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: vb_yaml::ast::StepPrimitive::Set {
            output: output.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn make_choose_branch(when: &str, steps: Vec<vb_yaml::ast::StepAst>) -> vb_yaml::ast::ChooseBranch {
    vb_yaml::ast::ChooseBranch {
        when: when.to_string(),
        steps,
    }
}
