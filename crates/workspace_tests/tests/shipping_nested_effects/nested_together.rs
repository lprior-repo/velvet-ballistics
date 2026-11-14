//! Cell E: the retained NestedTogether prohibition at any depth.
//!
//! Bead: vb-eirmk.4.7.3. Together inside Together stays a typed compile
//! refusal rather than a partially lowered source family.

use vb_compile::CompileError;
use vb_core::{StepIdx, WorkflowError};

use super::source::{compound, indent};
use super::support::TestResult;

fn nested_together_source(outer: &str, inner: &str) -> TestResult<String> {
    let leaf = "- id: leaf_leaf\n  set: {output: leaf_value, value: '7'}\n";
    let middle = compound("together", inner, leaf)?;
    let top = compound("together", outer, &middle)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_effects_nested_together\nwhen: {{manual: {{}}}}\nsteps:\n{}  - id: done\n    finish: {{result: '7'}}\n",
        indent(&top, 2)
    ))
}

fn rejection(source: &str) -> TestResult<(StepIdx, StepIdx)> {
    let errors = match vb_compile::compile_workflow(source.as_bytes()) {
        Ok(_) => return Err("nested Together source unexpectedly compiled".into()),
        Err(errors) => errors,
    };
    let rejected = errors.0.iter().find_map(|error| match error {
        CompileError::Workflow(WorkflowError::NestedTogether { outer, inner }) => {
            Some((*outer, *inner))
        }
        _ => None,
    });
    rejected.ok_or_else(|| format!("missing exact NestedTogether refusal: {errors:?}").into())
}

#[test]
fn nested_together_refusal_is_exact_and_deterministic_at_two_depths() -> TestResult {
    for (outer, inner) in [
        ("outer_group", "inner_group"),
        ("second_outer", "second_inner"),
    ] {
        let source = nested_together_source(outer, inner)?;
        let first = rejection(&source)?;
        let second = rejection(&source)?;
        assert_eq!(first, second, "refusal pair must be deterministic");
        assert_ne!(
            first.0, first.1,
            "refusal must identify two distinct scopes"
        );
    }
    Ok(())
}

/// Together nested under Repeat nested in Together is still refused.
#[test]
fn together_inside_together_nested_in_repeat_is_refused() -> TestResult {
    let leaf = "- id: deep_leaf\n  set: {output: deep, value: '3'}\n";
    let inner = compound("together", "deep_group", leaf)?;
    let repeat = compound("repeat", "wrapper", &inner)?;
    let outer = compound("together", "top_group", &repeat)?;
    let source = format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_effects_nested_together_repeat\nwhen: {{manual: {{}}}}\nsteps:\n{}  - id: done\n    finish: {{result: '3'}}\n",
        indent(&outer, 2)
    );
    let _ = rejection(&source)?;
    Ok(())
}
