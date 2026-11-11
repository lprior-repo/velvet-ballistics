//! Nested body composition: loops and branches inside loop bodies.
//!
//! Compile-only layout proofs; execution is qualified by runtime drive tests
//! and the workspace `nested_composition` end-to-end suite.

use crate::{CompileError, compile_workflow};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, FanoutLimit, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: nested_bodies\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  outer_items: {is: list, of: any, max: 4}\n  inner_items: {is: list, of: any, max: 4}\n";

fn source(body: &str) -> String {
    format!("{HEADER}{INPUTS}steps:\n{body}")
}

fn compile(text: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn node(workflow: &CompiledWorkflow, id: u16) -> Result<&CompiledNode, String> {
    workflow
        .node(StepIdx::new(id))
        .ok_or_else(|| format!("missing node {id}"))
}

fn output(workflow: &CompiledWorkflow, id: u16) -> Result<SlotIdx, String> {
    node(workflow, id)?
        .output
        .slot()
        .ok_or_else(|| format!("missing output {id}"))
}

fn rejects(text: &str, predicate: impl Fn(&CompileError) -> bool) -> Result<(), String> {
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

fn finish_tail() -> &'static str {
    "  - id: done\n    finish: {result: '$seed'}\n"
}

mod layout;
mod foreach;
