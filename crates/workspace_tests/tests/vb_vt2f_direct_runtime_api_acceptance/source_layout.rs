use super::source_wait::SOURCE;
use super::*;
use vb_compile::compile_workflow;

fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

#[test]
fn numeric_vs_named_refs() -> Result<(), String> {
    let numeric_ask = SOURCE.replace("$alias", "0").replace("$steps.clock", "1");
    let compiled_numeric = compile(&numeric_ask)?;
    assert_eq!(compiled_numeric.node_count(), 5);

    let wait_numeric = numeric_ask
        .replace("ask:", "wait:")
        .replace("prompt:", "event:");
    let compiled_wait = compile(&wait_numeric)?;
    assert_eq!(compiled_wait.node_count(), 4);

    let named_variants = vec![
        SOURCE.to_owned(),
        SOURCE.replace("$alias", "$seed"),
        SOURCE.replace("$alias", "$step.seed"),
        SOURCE.replace("$alias", "$steps.seed"),
        SOURCE.replace("$steps.clock", "$duration"),
        SOURCE.replace("$steps.clock", "$clock"),
        SOURCE.replace("$steps.clock", "$step.clock"),
    ];

    for named_source in &named_variants {
        let compiled = compile(named_source)?;
        assert_eq!(compiled.node_count(), compiled_numeric.node_count());
        assert_eq!(compiled.to_parts().nodes, compiled_numeric.to_parts().nodes);
        assert_eq!(compiled.slot_count(), compiled_numeric.slot_count());

        let wait_named = named_source
            .replace("ask:", "wait:")
            .replace("prompt:", "event:");
        let compiled_wait_named = compile(&wait_named)?;
        assert_eq!(compiled_wait_named.node_count(), compiled_wait.node_count());
        assert_eq!(
            compiled_wait_named.to_parts().nodes,
            compiled_wait.to_parts().nodes
        );
        assert_eq!(compiled_wait_named.slot_count(), compiled_wait.slot_count());
    }

    let baseline = compiled_numeric
        .node(StepIdx::new(2))
        .ok_or("missing baseline node")?;
    match &baseline.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            assert_eq!(*prompt, SlotIdx::new(0));
            assert_eq!(*timeout_slot, Some(SlotIdx::new(1)));
        }
        _ => return Err("baseline should be Ask node".into()),
    }

    let wait_node = compiled_wait
        .node(StepIdx::new(2))
        .ok_or("missing wait node")?;
    match &wait_node.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert_eq!(*event, SlotIdx::new(0));
            assert_eq!(*timeout_slot, Some(SlotIdx::new(1)));
        }
        _ => return Err("wait should be WaitEvent node".into()),
    }

    Ok(())
}

#[test]
fn absent_timeout_and_same_id_alias() -> Result<(), String> {
    let no_timeout = SOURCE.replace("      timeout: \"$steps.clock\"\n", "");
    let compiled = compile(&no_timeout)?;
    assert_eq!(compiled.node_count(), 5);
    let node = compiled.node(StepIdx::new(2)).ok_or("missing node")?;
    match &node.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            assert_eq!(*prompt, SlotIdx::new(0));
            assert_eq!(*timeout_slot, None);
        }
        _ => return Err("should be Ask node".into()),
    }

    let wait_no_timeout = no_timeout
        .replace("ask:", "wait:")
        .replace("prompt:", "event:");
    let compiled_wait = compile(&wait_no_timeout)?;
    assert_eq!(compiled_wait.node_count(), 4);
    let node_wait = compiled_wait
        .node(StepIdx::new(2))
        .ok_or("missing wait node")?;
    match &node_wait.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert_eq!(*event, SlotIdx::new(0));
            assert_eq!(*timeout_slot, None);
        }
        _ => return Err("should be WaitEvent node".into()),
    }

    let same_id_alias = SOURCE
        .replace("output: alias", "output: seed")
        .replace("$alias", "$seed");
    let compiled_same = compile(&same_id_alias)?;
    let compiled_orig = compile(SOURCE)?;
    assert_eq!(compiled_same.node_count(), compiled_orig.node_count());
    assert_eq!(
        compiled_same.to_parts().nodes,
        compiled_orig.to_parts().nodes
    );

    Ok(())
}

#[test]
fn input_and_vars_slot_allocation() -> Result<(), String> {
    let input_source = SOURCE
        .replace("steps:", "inputs:\n  num: number\nsteps:")
        .replace("$alias", "$input.num")
        .replace("$steps.clock", "$input.num");
    let compiled_input = compile(&input_source)?;
    assert_eq!(compiled_input.node_count(), 5);
    let ask_node = compiled_input
        .node(StepIdx::new(2))
        .ok_or("missing ask node")?;
    match &ask_node.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            assert_eq!(*prompt, SlotIdx::new(2));
            assert_eq!(*timeout_slot, Some(SlotIdx::new(2)));
        }
        _ => return Err("should be Ask node".into()),
    }

    let vars_source = SOURCE
        .replace("steps:", "vars:\n  num: 42\nsteps:")
        .replace("$alias", "$vars.num")
        .replace("$steps.clock", "$vars.num");
    let compiled_vars = compile(&vars_source)?;
    assert_eq!(compiled_vars.node_count(), 6);
    let ask_node_vars = compiled_vars
        .node(StepIdx::new(3))
        .ok_or("missing ask node")?;
    match &ask_node_vars.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            assert_eq!(*prompt, SlotIdx::new(2));
            assert_eq!(*timeout_slot, Some(SlotIdx::new(2)));
        }
        _ => return Err("should be Ask node".into()),
    }

    let wait_input = input_source
        .replace("ask:", "wait:")
        .replace("prompt:", "event:");
    let compiled_wait_input = compile(&wait_input)?;
    assert_eq!(compiled_wait_input.node_count(), 4);
    let wait_node = compiled_wait_input
        .node(StepIdx::new(2))
        .ok_or("missing wait node")?;
    match &wait_node.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert_eq!(*event, SlotIdx::new(2));
            assert_eq!(*timeout_slot, Some(SlotIdx::new(2)));
        }
        _ => return Err("should be WaitEvent node".into()),
    }

    let wait_vars = vars_source
        .replace("ask:", "wait:")
        .replace("prompt:", "event:");
    let compiled_wait_vars = compile(&wait_vars)?;
    assert_eq!(compiled_wait_vars.node_count(), 5);
    let wait_node_vars = compiled_wait_vars
        .node(StepIdx::new(3))
        .ok_or("missing wait node")?;
    match &wait_node_vars.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert_eq!(*event, SlotIdx::new(2));
            assert_eq!(*timeout_slot, Some(SlotIdx::new(2)));
        }
        _ => return Err("should be WaitEvent node".into()),
    }

    Ok(())
}
