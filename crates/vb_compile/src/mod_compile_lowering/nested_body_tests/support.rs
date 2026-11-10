use super::*;

const HEADER: &str = "version: velvet-ballistics/v1\nname: nested_bodies\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  outer_items: {is: list, of: any, max: 4}\n  inner_items: {is: list, of: any, max: 4}\n";

pub(super) fn source(body: &str) -> String {
    format!("{HEADER}{INPUTS}steps:\n{body}")
}

pub(super) fn compile(text: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

pub(super) fn node(workflow: &CompiledWorkflow, id: u16) -> Result<&CompiledNode, String> {
    workflow
        .node(StepIdx::new(id))
        .ok_or_else(|| format!("missing node {id}"))
}

pub(super) fn output(workflow: &CompiledWorkflow, id: u16) -> Result<SlotIdx, String> {
    node(workflow, id)?
        .output
        .slot()
        .ok_or_else(|| format!("missing output {id}"))
}

pub(super) fn rejects(text: &str, predicate: impl Fn(&CompileError) -> bool) -> Result<(), String> {
    let errors = match compile_workflow(text.as_bytes()) {
        Ok(_) => return Err("unexpected source acceptance".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(predicate) {
        Ok(())
    } else {
        Err(format!("wrong diagnostic: {errors:?}"))
    }
}

pub(super) fn finish_tail() -> &'static str {
    "  - id: done\n    finish: {result: '$seed'}\n"
}

pub(super) fn in_for_each(body: &str) -> String {
    source(&format!(
        "  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: outer\n    for_each:\n      input: '$input.outer_items'\n      variable: entry\n      steps:\n{body}"
    )) + finish_tail()
}
