#![forbid(unsafe_code)]

#[path = "vb_f2_nested_matrix/source.rs"]
mod source;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HEAD: &str = "version: velvet-ballistics/v1\nname: nested_gather_budget\nwhen: {manual: {}}\nvars: {sentinel: 1}\ninputs: {items: {is: list, of: number, max: 2}}\nsteps:\n";
const BODY: &str = "- id: output_{name}\n  set: {output: value_{name}, value: '$page_{name}'}\n";

fn collect(name: &str) -> TestResult<String> {
    let body = BODY.replace("{name}", name);
    Ok(source::compound("collect", name, &body)?)
}

fn budget(outer: &str) -> TestResult<vb_core::budget::AggregateResourceBudget> {
    let _families = source::FAMILIES;
    let yaml = format!(
        "{HEAD}{}  - id: done\n    finish: {{result: '$vars.sentinel'}}\n",
        source::indent(outer, 2)
    );
    let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
    Ok(vb_core::budget::AggregateResourceBudget::from_workflow(
        &workflow,
    )?)
}

fn assert_gather_budget(budget: vb_core::budget::AggregateResourceBudget, pages: u32, items: u32) {
    assert_eq!(
        (budget.max_gather_pages, budget.max_gather_items),
        (pages, items)
    );
}

#[test]
fn repeat_scales_nested_collects_and_sums_sequential_siblings() -> TestResult {
    let body = format!("{}{}", collect("first")?, collect("second")?);
    let outer = source::compound("repeat", "outer", &body)?;
    assert_gather_budget(budget(&outer)?, 8, 8);
    Ok(())
}

#[test]
fn for_each_scales_each_nested_collection_once_per_iteration() -> TestResult {
    let inner = collect("nested")?;
    let outer = source::compound("for_each", "outer", &inner)?;
    assert_gather_budget(budget(&outer)?, 4, 4);
    Ok(())
}

#[test]
fn standalone_collection_and_empty_gather_path_keep_exact_caps() -> TestResult {
    assert_gather_budget(budget(&collect("single")?)?, 2, 2);
    assert_gather_budget(budget("")?, 0, 0);
    Ok(())
}

#[test]
fn nested_gather_overflow_fails_with_typed_budget_error() -> TestResult {
    let max_items = u32::MAX;
    let yaml = format!(
        "version: velvet-ballistics/v1\nname: nested_gather_overflow\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: {max_items}}}}}\nsteps:\n  - id: outer_loop\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: gather\n          collect:\n            variable: page_item\n            source: '$input.items'\n            pages: 1\n            items: {max_items}\n            steps:\n              - id: leaf\n                set: {{output: leaf, value: '$page_item'}}\n  - id: done\n    finish: {{result: '$step.outer_loop'}}\n"
    );
    let errors = match vb_compile::compile_workflow(yaml.as_bytes()) {
        Ok(_) => return Err("overflowing nested gather compiled".into()),
        Err(errors) => errors,
    };
    if errors.0.len() != 1
        || !matches!(
            errors.0.first(),
            Some(vb_compile::CompileError::Workflow(
                vb_core::WorkflowError::StepCountOverflow { actual }
            )) if *actual > u64::from(u32::MAX)
        )
    {
        return Err(format!("unexpected budget refusal: {errors:?}").into());
    }
    Ok(())
}
