#![forbid(unsafe_code)]

use vb_compile::{CompileError, compile_workflow};
use vb_core::{
    CapabilitySet, RunId, SlotIdx, SlotValue, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), String>;

const FINISH: &str = "  - id: done\n    finish:\n      result: \"$steps.folded\"\n";

fn source(prefix: &str, expression: &str, suffix: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: reduce_lexical_binding\nwhen:\n  manual: {{}}\nsteps:\n{prefix}  - id: folded\n    reduce:\n      variable: total\n      input: \"8\"\n      initial: \"10\"\n      steps:\n        - id: update\n          set:\n            output: local_value\n            value: \"{expression}\"\n{suffix}"
    )
}

fn assert_run(source: &str, input: &[i64], expected: i64) -> TestResult {
    assert_run_value(source, input, SlotValue::I64(expected))
}

fn assert_run_value(source: &str, input: &[i64], expected: SlotValue) -> TestResult {
    let workflow = compile_workflow(source.as_bytes())
        .map_err(|error| format!("compilation failed: {error:?}"))?;
    let original_ir = postcard::to_allocvec(&workflow.to_parts())
        .map_err(|error| format!("IR encoding failed: {error:?}"))?;
    let mut frame = new_run_frame(RunId::new(1), &workflow)
        .map_err(|error| format!("frame creation failed: {error:?}"))?;
    let values: Vec<SlotValue> = input.iter().copied().map(SlotValue::I64).collect();
    let mut store = ValueStore::new();
    let list = store
        .insert_list(values.clone())
        .map_err(|error| format!("input list allocation failed: {error:?}"))?;
    frame
        .write_slot_with_taint(
            SlotIdx::new(8),
            SlotValue::List(list),
            Taint::DerivedFromSecret,
        )
        .map_err(|error| format!("input binding failed: {error:?}"))?;
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|error| format!("execution failed: {error:?}"))?;
    assert_eq!(signal, RuntimeSignal::Finished(expected));
    assert_eq!(
        *frame
            .read_slot(SlotIdx::new(8))
            .map_err(|error| format!("input slot: {error:?}"))?,
        SlotValue::List(list)
    );
    assert_eq!(
        frame
            .read_taint(SlotIdx::new(8))
            .map_err(|error| format!("input taint: {error:?}"))?,
        Taint::DerivedFromSecret
    );
    assert_eq!(
        store
            .list(list)
            .map_err(|error| format!("input list: {error:?}"))?,
        values.as_slice()
    );
    assert_eq!(
        postcard::to_allocvec(&workflow.to_parts())
            .map_err(|error| format!("IR encoding failed: {error:?}"))?,
        original_ir
    );
    Ok(())
}

#[test]
fn declared_accumulator_executes_for_empty_single_and_multiple_items() -> TestResult {
    let source = source("", "$total + 1", FINISH);
    for (input, expected) in [(&[][..], 10), (&[4][..], 11), (&[4, 7, -3][..], 13)] {
        assert_run(&source, input, expected)?;
    }
    Ok(())
}

#[test]
fn reduce_result_and_outer_binding_are_available_to_later_expressions() -> TestResult {
    let prefix = "  - id: seed\n    set:\n      output: increment\n      value: \"5\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$steps.folded + 7\"\n  - id: done\n    finish:\n      result: \"$step.after\"\n";
    assert_run(&source(prefix, "$total + $increment", suffix), &[2, 9], 27)
}

#[test]
fn accumulator_shadowing_stays_inside_the_reduce_body() -> TestResult {
    let prefix = "  - id: seed\n    set:\n      output: total\n      value: \"99\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$steps.folded + $total\"\n  - id: done\n    finish:\n      result: \"$steps.after\"\n";
    assert_run(&source(prefix, "$total + 1", suffix), &[1, 2, 3], 112)
}

#[test]
fn accumulator_binding_does_not_escape_the_reduce_body() -> TestResult {
    let suffix = "  - id: done\n    finish:\n      result: \"$total\"\n";
    let errors = match compile_workflow(source("", "$total + 1", suffix).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("body-local accumulator escaped".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::UnknownReferenceName { kind: "output", name, .. } if name.as_ref() == "total"
    )));
    Ok(())
}

#[test]
fn accumulator_expression_preserves_the_initial_integer_type() -> TestResult {
    let errors = match compile_workflow(source("", "$total == 10", FINISH).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("reduce accepted a changing accumulator type".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::StepFieldShape {
            field: "reduce.steps",
            ..
        }
    )));
    Ok(())
}

#[test]
fn local_accumulator_precedes_outer_step_id_without_hiding_explicit_steps() -> TestResult {
    let prefix = "  - id: total\n    set:\n      output: increment\n      value: \"7\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$steps.folded + $total + $steps.total + $increment\"\n  - id: done\n    finish:\n      result: \"$steps.after\"\n";
    assert_run(
        &source(prefix, "$total + $steps.total", suffix),
        &[2, 9],
        45,
    )
}

#[test]
fn global_output_and_step_name_ambiguity_remains_an_error() -> TestResult {
    let prefix = "  - id: total\n    set:\n      output: increment\n      value: \"7\"\n  - id: seed\n    set:\n      output: total\n      value: \"99\"\n";
    let suffix = "  - id: done\n    finish:\n      result: \"$total\"\n";
    let expression = "$total + 1";
    assert_run(&source(prefix, expression, FINISH), &[2, 9], 12)?;
    let errors = match compile_workflow(source(prefix, expression, suffix).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("global ambiguous reference was accepted".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::ExpressionLoweringUnsupported { feature }
            if feature.contains("ambiguous reference")
    )));
    Ok(())
}

fn with_typed_list_input(source: String) -> String {
    source.replacen(
        "steps:\n",
        "inputs:\n  items: {is: list, of: any, max: 3}\nsteps:\n",
        1,
    )
}

fn assert_sum_control_compiles(source: &str) -> TestResult {
    let workflow = compile_workflow(source.as_bytes())
        .map_err(|error| format!("sum list control failed compilation: {error:?}"))?;
    assert!(workflow.to_parts().expressions.iter().any(|program| {
        program
            .ops
            .iter()
            .any(|op| matches!(op, vb_core::ExprOp::Sum))
    }));
    Ok(())
}

fn assert_exact_list_i64_mismatch(source: &str) -> TestResult {
    let errors = match compile_workflow(source.as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("sum accepted a statically known i64 argument".into()),
    };
    assert!(matches!(
        errors.0.as_slice(),
        [CompileError::TypeMismatch {
            field: "set.value",
            expected: "list",
            found: "i64",
        }]
    ));
    Ok(())
}

#[test]
fn accumulator_sum_domain_rejection_has_successful_list_control() -> TestResult {
    let control = with_typed_list_input(source("", "$total + sum($input.items)", FINISH));
    assert_sum_control_compiles(&control)?;
    let rejected = with_typed_list_input(source("", "sum($total)", FINISH));
    assert_exact_list_i64_mismatch(&rejected)
}

#[test]
fn reduce_result_sum_domain_rejection_has_successful_list_control() -> TestResult {
    let suffix = |expression: &str| {
        format!(
            "  - id: after\n    set:\n      output: answer\n      value: \"{expression}\"\n  - id: done\n    finish:\n      result: \"$steps.after\"\n"
        )
    };
    let control = with_typed_list_input(source("", "$total + 1", &suffix("sum($input.items)")));
    assert_sum_control_compiles(&control)?;
    let rejected = with_typed_list_input(source("", "$total + 1", &suffix("sum($steps.folded)")));
    assert_exact_list_i64_mismatch(&rejected)
}

const NUMERIC_DO_REDUCE: &str = r#"version: velvet-ballistics/v1
name: reduce_numeric_do_body
when:
  manual: {}
steps:
  - id: folded
    reduce:
      variable: total
      input: "8"
      initial: "10"
      steps:
        - id: update
          do:
            action: "0"
            input: "1"
  - id: done
    finish:
      result: "$steps.folded"
"#;

fn numeric_do_contract() -> Result<vb_core::ActionContract, String> {
    Ok(vb_core::ActionContract {
        id: vb_core::ActionId::new(0),
        name: vb_core::action::ActionName::new("reduce-update")
            .map_err(|error| error.to_string())?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 64,
        max_output_bytes: 64,
        timeout_ms: 1000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::from([]),
    })
}

fn drive_numeric_reduce(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    store: &mut ValueStore,
    contracts: &[vb_core::ActionContract],
) -> Result<RuntimeSignal, String> {
    drive_deterministic_full(
        workflow,
        frame,
        &mut StepBudget::new(128),
        store,
        contracts,
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|error| format!("numeric Do Reduce drive failed: {error:?}"))
}

fn assert_numeric_do_reduce(input: &[i64], completions: &[i64], expected: i64) -> TestResult {
    let workflow = compile_workflow(NUMERIC_DO_REDUCE.as_bytes())
        .map_err(|error| format!("numeric Do Reduce compile failed: {error:?}"))?;
    let parts = workflow.to_parts();
    let original_ir = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    let bodies = parts
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, vb_core::CompiledNodeKind::Do { .. }))
        .collect::<Vec<_>>();
    assert_eq!(bodies.len(), 1);
    let body = bodies.first().ok_or("numeric Do body missing")?;
    assert!(
        matches!(body.kind, vb_core::CompiledNodeKind::Do { action, input }
        if action == vb_core::ActionId::new(0) && input == SlotIdx::new(1))
    );
    let accumulator = body.output.slot().ok_or("numeric Do output missing")?;
    let next = body
        .routing
        .next
        .step()
        .ok_or("numeric Do continuation missing")?;
    let run = RunId::new(93);
    let mut frame = new_run_frame(run, &workflow).map_err(|error| error.to_string())?;
    let mut store = ValueStore::new();
    let values = input
        .iter()
        .copied()
        .map(SlotValue::I64)
        .collect::<Vec<_>>();
    let list = store
        .insert_list(values.clone())
        .map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(
            SlotIdx::new(8),
            SlotValue::List(list),
            Taint::DerivedFromSecret,
        )
        .map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(7), Taint::Clean)
        .map_err(|error| error.to_string())?;
    assert_eq!(input.len(), completions.len());
    let contracts = [numeric_do_contract()?];
    let mut signal = drive_numeric_reduce(&workflow, &mut frame, &mut store, &contracts)?;
    let mut previous = 10;
    for value in completions {
        let ticket = match signal {
            RuntimeSignal::AwaitingAction(ticket) => ticket,
            other => return Err(format!("expected numeric Do suspension, got {other:?}")),
        };
        assert_eq!(ticket.run, run);
        assert_eq!(ticket.step, body.id);
        assert_eq!(ticket.action, vb_core::ActionId::new(0));
        assert_eq!(
            *frame
                .read_slot(accumulator)
                .map_err(|error| error.to_string())?,
            SlotValue::I64(previous)
        );
        assert_eq!(
            frame
                .step_state(body.id)
                .map_err(|error| error.to_string())?,
            vb_core::StepState::Running
        );
        let payload =
            postcard::to_allocvec(&SlotValue::I64(*value)).map_err(|error| error.to_string())?;
        let output = vb_core::action::ActionOutputReady {
            output_slot: accumulator,
            value: SlotValue::I64(*value),
            taint: Taint::Clean,
            encoded_len: u32::try_from(payload.len()).map_err(|error| error.to_string())?,
        };
        let (continued, event) = vb_core::resume_action_completion(
            &workflow,
            &mut frame,
            ticket,
            output,
            &payload,
            contracts.first().ok_or("action contract missing")?,
        )
        .map_err(|error| format!("numeric Do completion failed: {error:?}"))?;
        assert_eq!(continued, vb_core::EngineSignal::Continue);
        assert!(
            matches!(event, vb_core::action::ActionJournalEvent::Completed { ticket: actual, output, .. }
            if actual == ticket && output.value == SlotValue::I64(*value) && output.output_slot == accumulator)
        );
        assert_eq!(frame.pc(), next);
        assert_eq!(
            frame
                .step_state(body.id)
                .map_err(|error| error.to_string())?,
            vb_core::StepState::Succeeded
        );
        previous = *value;
        signal = drive_numeric_reduce(&workflow, &mut frame, &mut store, &contracts)?;
    }
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(expected)));
    assert_eq!(
        *frame
            .read_slot(SlotIdx::new(8))
            .map_err(|error| error.to_string())?,
        SlotValue::List(list)
    );
    assert_eq!(
        frame
            .read_taint(SlotIdx::new(8))
            .map_err(|error| error.to_string())?,
        Taint::DerivedFromSecret
    );
    assert_eq!(
        store.list(list).map_err(|error| error.to_string())?,
        values.as_slice()
    );
    assert_eq!(
        postcard::to_allocvec(&workflow.to_parts()).map_err(|error| error.to_string())?,
        original_ir
    );
    Ok(())
}

#[test]
fn numeric_do_reduce_suspends_and_completes_each_item_without_rewriting_ir() -> TestResult {
    assert_numeric_do_reduce(&[4, 7], &[20, 30], 30)
}

#[test]
fn numeric_do_reduce_empty_input_finishes_with_initial_without_suspending() -> TestResult {
    assert_numeric_do_reduce(&[], &[], 10)
}

// ---------------------------------------------------------------------------
// Multi-step bodies, nested containers, and typed accumulators.
// ---------------------------------------------------------------------------

fn multi_source(prefix: &str, initial: &str, steps: &str, suffix: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: reduce_multi_step\nwhen:\n  manual: {{}}\nsteps:\n{prefix}  - id: folded\n    reduce:\n      variable: total\n      input: \"8\"\n      initial: \"{initial}\"\n      steps:\n{steps}{suffix}"
    )
}

fn accumulator_slot(workflow: &vb_core::CompiledWorkflow) -> Result<SlotIdx, String> {
    workflow
        .to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            vb_core::CompiledNodeKind::ReduceFinish { accumulator } => Some(accumulator),
            _ => None,
        })
        .ok_or_else(|| "reduce accumulator slot missing".to_owned())
}

fn assert_typed_taint(
    workflow: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    accumulator: SlotIdx,
) -> TestResult {
    assert_eq!(
        frame
            .read_taint(accumulator)
            .map_err(|error| format!("accumulator taint: {error:?}"))?,
        Taint::Clean
    );
    let _ = workflow;
    Ok(())
}

#[test]
fn multi_step_reduce_accumulates_outer_derived_delta_per_item() -> TestResult {
    let prefix = "  - id: seed\n    set:\n      output: increment\n      value: \"5\"\n";
    let steps = "        - id: tmp_step\n          set:\n            output: tmp_value\n            value: \"$increment * 2\"\n        - id: update\n          set:\n            output: local_value\n            value: \"$total + $tmp_value\"\n";
    let source = multi_source(prefix, "10", steps, FINISH);
    for (input, expected) in [(&[][..], 10), (&[4][..], 20), (&[4, 7][..], 30)] {
        assert_run(&source, input, expected)?;
    }
    Ok(())
}

#[test]
fn nested_repeat_in_multi_step_reduce_feeds_the_accumulator() -> TestResult {
    let steps = "        - id: loop\n          repeat:\n            max_attempts: 1\n            steps:\n              - id: inner\n                set:\n                  output: tmp_value\n                  value: \"6\"\n        - id: update\n          set:\n            output: local_value\n            value: \"$total + $steps.loop\"\n";
    let source = multi_source("", "10", steps, FINISH);
    for (input, expected) in [(&[][..], 10), (&[1][..], 16), (&[1, 2][..], 22)] {
        assert_run(&source, input, expected)?;
    }
    Ok(())
}

fn f64_source() -> String {
    let steps = "        - id: tmp_step\n          set:\n            output: delta\n            value: \"0.75\"\n        - id: update\n          set:\n            output: local_value\n            value: \"$delta\"\n";
    multi_source("", "0.5", steps, FINISH)
}

#[test]
fn f64_accumulator_tracks_the_typed_initial_and_preserves_input_taint() -> TestResult {
    let source = f64_source();
    let expected =
        SlotValue::F64(vb_core::FiniteF64::new(0.75).map_err(|error| error.to_string())?);
    let initial_expected =
        SlotValue::F64(vb_core::FiniteF64::new(0.5).map_err(|error| error.to_string())?);
    assert_run_value(&source, &[], initial_expected)?;
    assert_run_value(&source, &[4], expected)?;
    let workflow = compile_workflow(source.as_bytes())
        .map_err(|error| format!("compilation failed: {error:?}"))?;
    let accumulator = accumulator_slot(&workflow)?;
    // Drive once more against the same compiled IR to inspect live taint.
    let mut frame = new_run_frame(RunId::new(2), &workflow)
        .map_err(|error| format!("frame creation failed: {error:?}"))?;
    let values = vec![SlotValue::I64(4)];
    let mut store = ValueStore::new();
    let list = store
        .insert_list(values)
        .map_err(|error| format!("input list allocation failed: {error:?}"))?;
    frame
        .write_slot_with_taint(
            SlotIdx::new(8),
            SlotValue::List(list),
            Taint::DerivedFromSecret,
        )
        .map_err(|error| format!("input binding failed: {error:?}"))?;
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(128),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        2,
    )
    .map_err(|error| format!("execution failed: {error:?}"))?;
    assert_eq!(signal, RuntimeSignal::Finished(expected));
    assert_typed_taint(&workflow, &mut frame, accumulator)?;
    Ok(())
}

#[test]
fn bool_accumulator_folds_with_constants() -> TestResult {
    let steps = "        - id: tmp_step\n          set:\n            output: flag\n            value: \"true\"\n        - id: update\n          set:\n            output: local_value\n            value: \"$total or $flag\"\n";
    let source = multi_source("", "false", steps, FINISH);
    assert_run_value(&source, &[], SlotValue::Bool(false))?;
    assert_run_value(&source, &[4], SlotValue::Bool(true))?;
    assert_run_value(&source, &[4, 7], SlotValue::Bool(true))?;
    Ok(())
}

#[test]
fn empty_body_is_rejected() -> TestResult {
    let text = "version: velvet-ballistics/v1\nname: reduce_empty\nwhen:\n  manual: {}\nsteps:\n  - id: folded\n    reduce:\n      variable: total\n      input: \"8\"\n      initial: \"10\"\n      steps: []\n  - id: done\n    finish:\n      result: \"$steps.folded\"\n";
    let errors = match compile_workflow(text.as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("empty body was accepted".into()),
    };
    assert!(
        errors
            .0
            .iter()
            .any(|error| matches!(error, CompileError::StepFieldShape { field: "steps", .. }))
    );
    Ok(())
}

#[test]
fn body_step_outputs_do_not_escape_the_reduce_body() -> TestResult {
    let steps = "        - id: update\n          set:\n            output: tmp_value\n            value: \"2\"\n        - id: final\n          set:\n            output: local_value\n            value: \"$total + $tmp_value\"\n";
    let suffix = "  - id: after\n    set:\n      output: answer\n      value: \"$tmp_value\"\n  - id: done\n    finish:\n      result: \"$steps.after\"\n";
    let errors = match compile_workflow(multi_source("", "10", steps, suffix).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("body-local output escaped the reduce body".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::UnknownReferenceName { kind: "output", name, .. }
            if name.as_ref() == "tmp_value"
    )));
    Ok(())
}

#[test]
fn body_step_output_cannot_alias_the_accumulator_variable() -> TestResult {
    let steps = "        - id: tmp_step\n          set:\n            output: tmp_value\n            value: \"2\"\n        - id: update\n          set:\n            output: total\n            value: \"$tmp_value\"\n";
    let errors = match compile_workflow(multi_source("", "10", steps, FINISH).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("variable alias was accepted".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"
    )));
    Ok(())
}

#[test]
fn body_step_id_cannot_shadow_the_accumulator_variable() -> TestResult {
    let steps = "        - id: total\n          set:\n            output: tmp_value\n            value: \"2\"\n        - id: update\n          set:\n            output: local_value\n            value: \"$total\"\n";
    let errors = match compile_workflow(multi_source("", "10", steps, FINISH).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("variable step id alias was accepted".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::DuplicateOutputName { name, .. } if name.as_ref() == "total"
    )));
    Ok(())
}

#[test]
fn multi_step_type_mismatch_is_rejected() -> TestResult {
    let steps = "        - id: tmp_step\n          set:\n            output: tmp_value\n            value: \"2\"\n        - id: update\n          set:\n            output: local_value\n            value: \"$total == 10\"\n";
    let errors = match compile_workflow(multi_source("", "10", steps, FINISH).as_bytes()) {
        Err(errors) => errors,
        Ok(_) => return Err("type-mismatched body was accepted".into()),
    };
    assert!(errors.0.iter().any(|error| matches!(
        error,
        CompileError::StepFieldShape {
            field: "reduce.steps",
            ..
        }
    )));
    Ok(())
}

const MULTI_DO_REDUCE: &str = r#"version: velvet-ballistics/v1
name: reduce_multi_do_body
when:
  manual: {}
steps:
  - id: folded
    reduce:
      variable: total
      input: "8"
      initial: "10"
      steps:
        - id: tmp_step
          set:
            output: tmp_value
            value: "2"
        - id: update
          do:
            action: "0"
            input: "1"
  - id: done
    finish:
      result: "$steps.folded"
"#;

#[test]
fn multi_step_do_suspension_updates_the_accumulator_slot() -> TestResult {
    let workflow = compile_workflow(MULTI_DO_REDUCE.as_bytes())
        .map_err(|error| format!("multi Do Reduce compile failed: {error:?}"))?;
    let accumulator = accumulator_slot(&workflow)?;
    let run = RunId::new(94);
    let mut frame = new_run_frame(run, &workflow).map_err(|error| error.to_string())?;
    let mut store = ValueStore::new();
    let list = store
        .insert_list(vec![SlotValue::I64(4), SlotValue::I64(7)])
        .map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(SlotIdx::new(8), SlotValue::List(list), Taint::Clean)
        .map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(7), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let contracts = [numeric_do_contract()?];
    let mut signal = drive_numeric_reduce(&workflow, &mut frame, &mut store, &contracts)?;
    let mut previous = 10;
    for completion in [20, 30] {
        let ticket = match signal {
            RuntimeSignal::AwaitingAction(ticket) => ticket,
            other => return Err(format!("expected suspension, got {other:?}")),
        };
        assert_eq!(
            *frame
                .read_slot(accumulator)
                .map_err(|error| error.to_string())?,
            SlotValue::I64(previous)
        );
        let payload = postcard::to_allocvec(&SlotValue::I64(completion))
            .map_err(|error| error.to_string())?;
        let output = vb_core::action::ActionOutputReady {
            output_slot: accumulator,
            value: SlotValue::I64(completion),
            taint: Taint::Clean,
            encoded_len: u32::try_from(payload.len()).map_err(|error| error.to_string())?,
        };
        let (continued, _) = vb_core::resume_action_completion(
            &workflow,
            &mut frame,
            ticket,
            output,
            &payload,
            contracts.first().ok_or("action contract missing")?,
        )
        .map_err(|error| format!("completion failed: {error:?}"))?;
        assert_eq!(continued, vb_core::EngineSignal::Continue);
        previous = completion;
        signal = drive_numeric_reduce(&workflow, &mut frame, &mut store, &contracts)?;
    }
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(30)));
    assert_eq!(
        *frame
            .read_slot(accumulator)
            .map_err(|error| error.to_string())?,
        SlotValue::I64(30)
    );
    Ok(())
}
