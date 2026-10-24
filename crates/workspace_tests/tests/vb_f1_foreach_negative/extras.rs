use super::{SOURCE, compile_workflow};

#[test]
fn unrelated_schema_is_not_a_foreach_restriction() -> Result<(), String> {
    use vb_core::CompiledNodeKind;

    let schemas = [
        "{is: list, of: number}",
        "{is: list, of: any}",
        "{is: list, of: any, max: 4294967296}",
    ];

    for schema in schemas {
        let source = SOURCE.replace("inputs:\n", &format!("inputs:\n  unrelated: {schema}\n"));
        let workflow = compile_workflow(source.as_bytes()).map_err(|e| format!("{e:?}"))?;

        let slots = workflow.input_slots();
        if slots.len() != 2 {
            return Err(format!("expected 2 input slots, got {}", slots.len()));
        }

        let items_slot = slots.get(1).ok_or("missing items slot")?.slot;

        let found = workflow
            .to_parts()
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                CompiledNodeKind::ForEachStart { input, limit, .. } => Some((*input, *limit)),
                _ => None,
            });

        match found {
            Some((input, limit)) => {
                if input != items_slot {
                    return Err(format!("input mismatch: {:?} vs {:?}", input, items_slot));
                }
                if limit != 3 {
                    return Err(format!("limit mismatch: {}", limit));
                }
            }
            None => return Err("ForEachStart not found".to_string()),
        }
    }

    Ok(())
}

#[test]
fn schema_bound_drives_ir_digest_and_budget() -> Result<(), String> {
    use vb_core::CompiledNodeKind;
    use vb_core::budget::WholeWorkflowBudget;

    let source = r#"version: velvet-ballistics/v1
name: foreach_negative
when: {manual: {}}
inputs:
  items: {is: list, of: any, max: 3}
steps:
  - id: seed
    set: {output: answer, value: "42"}
  - id: bounded_loop
    for_each:
      variable: entry
      input: "$input.items"
      at_once: 1
      steps:
        - id: copy
          set: {output: copied, value: "$entry"}
  - id: done
    finish: {result: answer}
"#;

    let max1_source = source.replace("max: 3", "max: 1");
    let serial_source = source.replace("      at_once: 1\n", "");

    let max3 = compile_workflow(source.as_bytes()).map_err(|e| format!("{e:?}"))?;
    let max1 = compile_workflow(max1_source.as_bytes()).map_err(|e| format!("{e:?}"))?;
    let serial = compile_workflow(serial_source.as_bytes()).map_err(|e| format!("{e:?}"))?;

    let max3_parts = max3.to_parts();
    let max1_parts = max1.to_parts();
    let serial_parts = serial.to_parts();

    let max3_budget = WholeWorkflowBudget::compute(
        &max3_parts.nodes,
        max3_parts.entry,
        &max3_parts.resource_contract,
    )
    .map_err(|e| format!("{e:?}"))?;
    let max1_budget = WholeWorkflowBudget::compute(
        &max1_parts.nodes,
        max1_parts.entry,
        &max1_parts.resource_contract,
    )
    .map_err(|e| format!("{e:?}"))?;
    let serial_budget = WholeWorkflowBudget::compute(
        &serial_parts.nodes,
        serial_parts.entry,
        &serial_parts.resource_contract,
    )
    .map_err(|e| format!("{e:?}"))?;

    let max3_limit = max3_parts.nodes.iter().find_map(|node| match &node.kind {
        CompiledNodeKind::ForEachStart { limit, .. } => Some(*limit),
        _ => None,
    });
    let max1_limit = max1_parts.nodes.iter().find_map(|node| match &node.kind {
        CompiledNodeKind::ForEachStart { limit, .. } => Some(*limit),
        _ => None,
    });

    if max3_limit != Some(3) {
        return Err(format!("expected max3 limit 3, got {:?}", max3_limit));
    }
    if max1_limit != Some(1) {
        return Err(format!("expected max1 limit 1, got {:?}", max1_limit));
    }

    if max1_parts.digest == max3_parts.digest {
        return Err("expected different digests for max1 vs max3".to_string());
    }

    if max1_budget.max_for_each_iterations != 1 {
        return Err(format!(
            "expected max1 budget max_for_each_iterations 1, got {}",
            max1_budget.max_for_each_iterations
        ));
    }
    if max3_budget.max_for_each_iterations != 3 {
        return Err(format!(
            "expected max3 budget max_for_each_iterations 3, got {}",
            max3_budget.max_for_each_iterations
        ));
    }

    if max1_budget.max_total_steps != 5 {
        return Err(format!(
            "expected max1 budget max_total_steps 5, got {}",
            max1_budget.max_total_steps
        ));
    }
    if max3_budget.max_total_steps != 9 {
        return Err(format!(
            "expected max3 budget max_total_steps 9, got {}",
            max3_budget.max_total_steps
        ));
    }

    if serial_parts.nodes != max3_parts.nodes {
        return Err("expected serial nodes == max3 nodes".to_string());
    }

    if serial_budget != max3_budget {
        return Err("expected serial budget == max3 budget".to_string());
    }

    Ok(())
}
