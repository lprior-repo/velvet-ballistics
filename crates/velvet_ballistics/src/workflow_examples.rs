//! Bounded execution of source examples during verification.

use vb_compile::{AuthorValue, WorkflowSource, author_value_to_owned};
use vb_core::{
    CompiledWorkflow, InputDefaultValue, RunId, SlotValue, StepBudget, Taint, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

#[cfg(test)]
mod tests;

pub(crate) fn verify(source: &WorkflowSource, workflow: &CompiledWorkflow) -> Result<(), String> {
    for (index, example) in source.examples().iter().enumerate() {
        run(source, workflow, example).map_err(|error| format!("example {index}: {error}"))?;
    }
    Ok(())
}

fn run(
    source: &WorkflowSource,
    workflow: &CompiledWorkflow,
    example: &vb_compile::ExampleAst,
) -> Result<(), String> {
    let mut checkpoint =
        ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1();
    let offset = workflow
        .input_slots()
        .len()
        .checked_sub(source.inputs().len())
        .ok_or("input declaration mismatch")?;
    let declarations = workflow
        .input_slots()
        .get(offset..)
        .ok_or("input declaration mismatch")?;
    let mut roots = Vec::new();
    if let Some(AuthorValue::Mapping(fields)) = &example.input {
        for field in fields {
            let declaration = source
                .inputs()
                .iter()
                .zip(declarations)
                .find_map(|(name, slot)| (name.key == field.key).then_some(slot))
                .ok_or_else(|| format!("unknown input {}", field.key))?;
            let value = author_value_to_owned(&field.value).map_err(|error| error.to_string())?;
            let taint = if declaration.constraint.secret {
                Taint::Secret
            } else {
                Taint::Clean
            };
            let value =
                vb_core::input_schema::materialize_input_default(&value, &mut checkpoint, taint)
                    .map_err(|error| error.to_string())?;
            roots.push((declaration.slot, value, taint));
        }
    }
    let bundle =
        vb_core::root_bundle::OwnedRootBundle::new(roots.into_boxed_slice(), checkpoint, workflow)
            .map_err(|error| error.to_string())?;
    let mut store = ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())
        .map_err(|error| error.to_string())?;
    let mut frame = new_run_frame(RunId::new(1), workflow).map_err(|error| error.to_string())?;
    for (slot, value, taint) in bundle.roots() {
        frame
            .write_slot_with_taint(*slot, *value, *taint)
            .map_err(|error| error.to_string())?;
    }
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let parts = workflow.to_parts();
    let bound = vb_core::budget::WholeWorkflowBudget::compute(
        &parts.nodes,
        parts.entry,
        &parts.resource_contract,
    )
    .map_err(|error| error.to_string())?
    .max_total_steps;
    let mut finished = None;
    for _ in 0..=bound {
        let mut budget = StepBudget::new(workflow.resource_contract().max_step_budget_per_tick);
        let before = budget.remaining();
        let signal = drive_deterministic_full(
            workflow,
            &mut frame,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &vb_core::CapabilitySet::empty(),
            1,
        )
        .map_err(|error| error.to_string())?;
        match signal {
            RuntimeSignal::Finished(value) => {
                finished = Some(value);
                break;
            }
            RuntimeSignal::StepBudgetExhausted if budget.remaining() < before => {}
            other => {
                return Err(format!(
                    "example requires a deterministic finish; received {other:?}"
                ));
            }
        }
    }
    let value = finished.ok_or("example exceeded the compiled execution bound")?;
    let expected = example.expected.as_ref().ok_or("missing expected value")?;
    let expected = author_value_to_owned(expected).map_err(|error| error.to_string())?;
    if equal(&expected, value, &store)? {
        Ok(())
    } else {
        Err("result differs from expected".into())
    }
}

fn equal(
    expected: &InputDefaultValue,
    actual: SlotValue,
    store: &ValueStore,
) -> Result<bool, String> {
    Ok(match (expected, actual) {
        (InputDefaultValue::Null, SlotValue::Null) => true,
        (InputDefaultValue::Bool(left), SlotValue::Bool(right)) => *left == right,
        (InputDefaultValue::I64(left), SlotValue::I64(right)) => *left == right,
        (InputDefaultValue::Text(left), SlotValue::Symbol(right)) => {
            left.as_ref() == store.symbol(right).map_err(|error| error.to_string())?
        }
        (InputDefaultValue::List(left), SlotValue::List(right)) => {
            let right = store.list(right).map_err(|error| error.to_string())?;
            if left.len() != right.len() {
                false
            } else {
                let mut matched = true;
                for (left, right) in left.iter().zip(right) {
                    matched &= equal(left, *right, store)?;
                }
                matched
            }
        }
        (InputDefaultValue::Object(left), SlotValue::Object(right)) => {
            let right = store.object(right).map_err(|error| error.to_string())?;
            if left.len() != right.len() {
                false
            } else {
                let mut matched = true;
                for (name, left) in left {
                    let mut found = None;
                    for field in right {
                        if store.symbol(field.key).map_err(|error| error.to_string())?
                            == name.as_ref()
                        {
                            found = Some(field.value);
                            break;
                        }
                    }
                    matched &= match found {
                        Some(right) => equal(left, right, store)?,
                        None => false,
                    };
                }
                matched
            }
        }
        _ => false,
    })
}
