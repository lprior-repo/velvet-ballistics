use vb_core::{
    CapabilitySet, CompiledWorkflow, RunId, SlotValue, StepBudget, Taint, ValueStore,
    new_run_frame, root_bundle::OwnedRootBundle,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn workflow(prefix: bool) -> TestResult<CompiledWorkflow> {
    let declarations = if prefix {
        "vars: {unused: marker}\n"
    } else {
        ""
    };
    let source = source(
        declarations,
        "{left: text, right: text}",
        "$input.left == $input.right",
    );
    Ok(vb_compile::compile_workflow(source.as_bytes())?)
}

fn source(declarations: &str, inputs: &str, expression: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: symbol_equality\nwhen: {{manual: {{}}}}\ninputs: {inputs}\n{declarations}steps:\n  - id: compare\n    set: {{output: same, value: '{expression}'}}\n  - id: done\n    finish: {{result: '$same'}}\n"
    )
}

fn bundle(workflow: &CompiledWorkflow, left: &str, right: &str) -> TestResult<OwnedRootBundle> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let a = store.insert_symbol(left)?;
    let b = store.insert_symbol(right)?;
    assert_ne!(a, b, "fixture requires distinct source handles");
    let roots = workflow
        .input_slots()
        .iter()
        .zip([a, b])
        .map(|(declaration, symbol)| (declaration.slot, SlotValue::Symbol(symbol), Taint::Clean))
        .collect::<Vec<_>>();
    Ok(OwnedRootBundle::new(
        roots.into_boxed_slice(),
        store.checkpoint_v1(),
        workflow,
    )?)
}

fn execute(workflow: &CompiledWorkflow, bundle: &OwnedRootBundle) -> TestResult<SlotValue> {
    let mut frame = new_run_frame(RunId::new(117), workflow)?;
    let mut store = ValueStore::from_checkpoint_v1(bundle.checkpoint().clone())?;
    for (slot, value, taint) in bundle.roots() {
        frame.write_slot_with_taint(*slot, *value, *taint)?;
    }
    let mut replay = frame.clone();
    let mut replay_store = store.clone();
    let result = drive_deterministic_full(
        workflow,
        &mut frame,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    let RuntimeSignal::Finished(value) = result else {
        return Err(format!("expected deterministic finish, received {result:?}").into());
    };
    for _ in 0..=workflow.node_count() {
        let node = workflow.node(replay.pc()).ok_or("replay node")?;
        let action = vb_core::replay::replay_step(node, &mut replay, &mut replay_store, workflow)
            .map_err(|error| format!("replay: {error:?}"))?;
        if matches!(action, vb_core::replay::ReplayAction::Finished) {
            let vb_core::CompiledNodeKind::Finish { result } = node.kind else {
                return Err("replay finished outside Finish".into());
            };
            assert_eq!(*replay.read_slot(result)?, value);
            return Ok(value);
        }
    }
    Err("replay did not finish".into())
}

#[path = "vb_f3_owned_symbol_equality/routes.rs"]
mod routes;

#[test]
fn equal_owned_text_is_independent_of_unused_source_string() -> TestResult {
    let mut observed = Vec::new();
    for prefix in [false, true] {
        let workflow = workflow(prefix)?;
        assert_eq!(
            workflow
                .source_metadata()
                .map(|metadata| metadata.symbols().to_vec()),
            if prefix {
                Some(vec![Box::<str>::from("marker")])
            } else {
                None
            }
        );
        observed.push(execute(&workflow, &bundle(&workflow, "same", "same")?)?);
    }
    assert_eq!(observed, vec![SlotValue::Bool(true), SlotValue::Bool(true)]);
    Ok(())
}

#[test]
fn owned_symbol_interning_is_independent_of_source_prefix() -> TestResult {
    for prefix in ["", "vars: {unused: marker}\n", "vars: {unused: same}\n"] {
        for (left, right) in [("same", "same"), ("same", "other"), ("é", "é")] {
            for shared in [false, true] {
                for operator in ["==", "!="] {
                    let source = source(
                        prefix,
                        "{left: text, right: text}",
                        &format!("$input.left {operator} $input.right"),
                    );
                    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
                    let mut store =
                        ValueStore::with_max_slots(workflow.resource_contract().max_slots);
                    let a = store.insert_symbol(left)?;
                    let b = if shared {
                        a
                    } else {
                        store.insert_symbol(right)?
                    };
                    let roots = workflow
                        .input_slots()
                        .iter()
                        .zip([a, b])
                        .map(|(input, id)| (input.slot, SlotValue::Symbol(id), Taint::Clean))
                        .collect::<Vec<_>>();
                    let bundle = OwnedRootBundle::new(
                        roots.into_boxed_slice(),
                        store.checkpoint_v1(),
                        &workflow,
                    )?;
                    let mut actual = bundle.roots().iter().map(|(_, value, _)| *value);
                    let first = actual.next().ok_or("first root")?;
                    let second = actual.next().ok_or("second root")?;
                    let equal = shared || left == right;
                    if equal {
                        assert_eq!(first, second);
                    } else {
                        assert_ne!(first, second);
                    }
                    let restored = OwnedRootBundle::new(
                        bundle.roots().to_vec().into_boxed_slice(),
                        bundle.checkpoint().clone(),
                        &workflow,
                    )?;
                    assert_eq!(restored.roots(), bundle.roots());
                    assert_eq!(restored.checkpoint(), bundle.checkpoint());
                    assert_eq!(
                        execute(&workflow, &restored)?,
                        SlotValue::Bool(equal == (operator == "=="))
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn text_defaults_and_source_literal_aliases_keep_their_existing_identity() -> TestResult {
    for prefix in ["", "vars: {unused: marker}\n", "vars: {unused: same}\n"] {
        let source = source(
            prefix,
            "{left: {is: text, default: same}, right: {is: text, default: same}}",
            "$input.left == $input.right",
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let bundle = OwnedRootBundle::new(Box::from([]), store.checkpoint_v1(), &workflow)?;
        assert_eq!(execute(&workflow, &bundle)?, SlotValue::Bool(true));
    }
    let source = source(
        "vars: {literal: same}\n",
        "{left: text, right: {is: text, default: same}}",
        "$input.left == $vars.literal",
    );
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let symbol = store.insert_symbol("same")?;
    let input = workflow.input_slots().first().ok_or("input")?.slot;
    let bundle = OwnedRootBundle::new(
        Box::from([(input, SlotValue::Symbol(symbol), Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    )?;
    assert_eq!(execute(&workflow, &bundle)?, SlotValue::Bool(true));
    Ok(())
}
