#![forbid(unsafe_code)]
//! F1 declaration and lexical-branch acceptance through canonical source IR.

use vb_compile::{CompileError, compile_workflow};
use vb_core::{
    CapabilitySet, CompiledNodeKind, CompiledWorkflow, RunId, SlotValue, StepBudget, Taint,
    ValueStore, new_run_frame,
};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

const HEADER: &str = "version: velvet-ballistics/v1\nname: lexical\nwhen: {manual: {}}\n";

fn source(declarations: &str, steps: &str) -> String {
    format!("{HEADER}{declarations}steps:\n{steps}")
}

fn scalar_source(declarations: &str, expression: &str) -> String {
    source(
        declarations,
        &format!(
            "  - id: calculate\n    set: {{output: answer, value: \"{expression}\"}}\n  - id: done\n    finish: {{result: answer}}\n"
        ),
    )
}

fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes()).map_err(|e| format!("canonical compile failed: {e:?}"))
}

fn execute(
    source: &str,
    input: Option<SlotValue>,
    expected: SlotValue,
) -> Result<(CompiledWorkflow, vb_core::RunFrame), String> {
    let workflow = compile(source)?;
    let original = workflow.to_parts();
    let mut frame = new_run_frame(RunId::new(1), &workflow).map_err(|e| format!("frame: {e:?}"))?;
    let input_slot = workflow.input_slots().first().map(|port| port.slot);
    if let Some(value) = input {
        let slot = input_slot.ok_or("source did not declare an input")?;
        frame
            .write_slot(slot, value)
            .map_err(|e| format!("input write: {e:?}"))?;
    }
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(256),
        &mut ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("unchanged source execution: {e:?}"))?;
    assert_eq!(signal, RuntimeSignal::Finished(expected));
    if let (Some(slot), Some(value)) = (input_slot, input) {
        assert_eq!(
            *frame
                .read_slot(slot)
                .map_err(|e| format!("input read: {e:?}"))?,
            value
        );
        assert_eq!(
            frame
                .read_taint(slot)
                .map_err(|e| format!("input taint: {e:?}"))?,
            Taint::Clean
        );
    }
    assert_eq!(workflow.to_parts(), original, "runtime changed compiler IR");
    Ok((workflow, frame))
}

fn rejects(source: &str, predicate: impl Fn(&CompileError) -> bool) -> Result<(), String> {
    let errors = match compile_workflow(source.as_bytes()) {
        Ok(_) => return Err("unexpected source acceptance".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(predicate) {
        Ok(())
    } else {
        Err(format!("wrong diagnostic: {errors:?}"))
    }
}

#[test]
fn declared_input_is_available_before_finish() -> Result<(), String> {
    execute(
        &scalar_source("inputs: {'n': number}\n", "$input.n + 2"),
        Some(SlotValue::I64(40)),
        SlotValue::I64(42),
    )?;
    Ok(())
}

#[test]
fn input_boolean_arithmetic_is_rejected_statically() -> Result<(), String> {
    rejects(
        &scalar_source("inputs: {'n': boolean}\n", "$input.n + 2"),
        |e| {
            matches!(
                e,
                CompileError::TypeMismatch {
                    expected: "number",
                    found: "boolean",
                    ..
                }
            )
        },
    )
}

#[test]
fn declared_input_finish_uses_the_same_binding() -> Result<(), String> {
    execute(
        &source(
            "inputs: {'n': number}\n",
            "  - id: done\n    finish: {result: '$input.n'}\n",
        ),
        Some(SlotValue::I64(40)),
        SlotValue::I64(40),
    )?;
    Ok(())
}

#[test]
fn scalar_vars_execute_before_source_steps() -> Result<(), String> {
    execute(
        &scalar_source(
            "vars: {'n': 40, enabled: true, absent: null}\n",
            "$vars.n + 2",
        ),
        None,
        SlotValue::I64(42),
    )?;
    Ok(())
}

#[test]
fn boolean_var_preserves_its_value() -> Result<(), String> {
    execute(
        &scalar_source("vars: {enabled: true}\n", "$vars.enabled"),
        None,
        SlotValue::Bool(true),
    )?;
    Ok(())
}

#[test]
fn null_var_preserves_its_value() -> Result<(), String> {
    execute(
        &scalar_source("vars: {absent: null}\n", "$vars.absent"),
        None,
        SlotValue::Null,
    )?;
    Ok(())
}

#[test]
fn vars_remain_namespaced_when_a_bare_alias_matches() -> Result<(), String> {
    let steps = "  - id: first\n    set: {output: 'n', value: '1'}\n  - id: calculate\n    set: {output: answer, value: '$vars.n + $n'}\n  - id: done\n    finish: {result: answer}\n";
    execute(
        &source("vars: {'n': 40}\n", steps),
        None,
        SlotValue::I64(41),
    )?;
    Ok(())
}

#[test]
fn wrong_var_type_is_rejected_statically() -> Result<(), String> {
    rejects(
        &scalar_source("vars: {enabled: true}\n", "$vars.enabled + 2"),
        |e| {
            matches!(
                e,
                CompileError::TypeMismatch {
                    expected: "number",
                    found: "boolean",
                    ..
                }
            )
        },
    )
}

#[test]
fn unknown_var_is_not_a_numeric_slot_error() -> Result<(), String> {
    rejects(&scalar_source("vars: {'n': 1}\n", "$vars.missing"), |e| {
        matches!(
            e,
            CompileError::UnknownReferenceName {
                kind: "declaration",
                ..
            }
        )
    })
}

#[test]
fn non_scalar_vars_are_not_faked_as_counts() -> Result<(), String> {
    rejects(&scalar_source("vars: {text: hello}\n", "1"), |e| {
        matches!(e, CompileError::UnsupportedConstantValue { step: 0 })
    })
}

fn choose_source(condition: &str, body: &str, finish: &str) -> String {
    source(
        "inputs: {enabled: boolean}\nvars: {amount: 11}\n",
        &format!(
            "  - id: first\n    set: {{output: 'n', value: '40'}}\n  - id: branch\n    choose:\n      branches:\n        - when: \"{condition}\"\n          steps:\n{body}      otherwise: done\n  - id: done\n    finish: {{result: '{finish}'}}\n"
        ),
    )
}

const TWO_STEP_BODY: &str = "            - id: first\n              set: {output: 'n', value: '$vars.amount'}\n            - id: second\n              set: {output: local, value: '$n + 31'}\n";

#[test]
fn choose_body_reads_parent_declarations_and_shadows_without_leaking() -> Result<(), String> {
    let (workflow, frame) = execute(
        &choose_source("$input.enabled", TWO_STEP_BODY, "$n"),
        Some(SlotValue::Bool(true)),
        SlotValue::I64(40),
    )?;
    let body_output = workflow
        .to_parts()
        .nodes
        .iter()
        .rev()
        .find_map(|node| {
            matches!(node.kind, CompiledNodeKind::EvalExpr { .. })
                .then_some(node.output)
                .flatten()
        })
        .ok_or("missing second body expression output")?;
    assert_eq!(
        *frame
            .read_slot(body_output)
            .map_err(|e| format!("body output: {e:?}"))?,
        SlotValue::I64(42)
    );
    assert_eq!(
        frame
            .read_taint(body_output)
            .map_err(|e| format!("body taint: {e:?}"))?,
        Taint::Clean
    );
    Ok(())
}

#[test]
fn false_choose_does_not_execute_its_body() -> Result<(), String> {
    let (workflow, frame) = execute(
        &choose_source("$input.enabled", TWO_STEP_BODY, "$n"),
        Some(SlotValue::Bool(false)),
        SlotValue::I64(40),
    )?;
    for node in &workflow.to_parts().nodes {
        if matches!(node.kind, CompiledNodeKind::EvalExpr { .. }) {
            let slot = node.output.ok_or("expression output missing")?;
            assert!(
                frame.read_slot(slot).is_err(),
                "unselected body wrote an output"
            );
        }
    }
    Ok(())
}

#[test]
fn branch_local_names_cannot_escape() -> Result<(), String> {
    rejects(&choose_source("true", TWO_STEP_BODY, "$local"), |e| {
        matches!(e, CompileError::UnknownReferenceName { .. })
    })
}

#[test]
fn choose_rejects_a_known_non_boolean_expression() -> Result<(), String> {
    rejects(
        &choose_source("$vars.amount + 1", TWO_STEP_BODY, "$n"),
        |e| {
            matches!(
                e,
                CompileError::TypeMismatch {
                    field: "choose.branches[].when",
                    expected: "boolean",
                    ..
                }
            )
        },
    )
}

#[test]
fn branch_bool_arithmetic_is_type_checked() -> Result<(), String> {
    let body = "            - id: local\n              set: {output: value, value: '$input.enabled + 1'}\n";
    rejects(&choose_source("true", body, "$n"), |e| {
        matches!(
            e,
            CompileError::TypeMismatch {
                expected: "number",
                found: "boolean",
                ..
            }
        )
    })
}

#[test]
fn ignored_body_controls_remain_rejected() -> Result<(), String> {
    let body = "            - id: local\n              if: 'false'\n              set: {output: value, value: '1'}\n";
    rejects(&choose_source("true", body, "$n"), |e| {
        matches!(e,
            CompileError::UnsupportedStepControlField { field, .. } if field.as_ref() == "if"
        )
    })
}

fn action_contract(id: vb_core::ActionId) -> Result<vb_core::action::ActionContract, String> {
    use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
    Ok(ActionContract {
        id,
        name: ActionName::new("lexical-action").map_err(|e| format!("action name: {e:?}"))?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    })
}

enum ExpectedActionFinish {
    ActionOutput,
    DeclaredInput,
}

fn execute_action(
    source: &str,
    expected: ExpectedActionFinish,
) -> Result<Vec<vb_runtime::journal::RuntimeJournalEvent>, String> {
    use std::sync::Arc;
    use vb_core::action::ActionOutputReady;
    use vb_runtime::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};
    use vb_runtime::runtime::Runtime;
    use vb_runtime::shard::{InspectResponse, ShardConfig};
    let workflow = compile(source)?;
    let port = workflow
        .input_slots()
        .first()
        .ok_or("missing declared input")?
        .slot;
    let parts = workflow.to_parts();
    let node = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Do { .. }))
        .ok_or("missing action")?;
    let CompiledNodeKind::Do { action, input } = node.kind else {
        return Err("not Do".to_owned());
    };
    let output = node.output.ok_or("missing action output")?;
    assert_eq!(input, port);
    assert_ne!(input, output);
    let step = node.id;
    let finish = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or("missing Finish result")?;
    let expected_result = match expected {
        ExpectedActionFinish::ActionOutput => output,
        ExpectedActionFinish::DeclaredInput => port,
    };
    assert_eq!(
        finish, expected_result,
        "compiled Finish must match the source fixture's result intent"
    );
    let contracts = (0..=action.get())
        .map(|id| action_contract(vb_core::ActionId::new(id)))
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let shared: Arc<dyn RuntimeJournal> = journal.clone();
    let mut runtime = Runtime::new(
        std::num::NonZeroUsize::new(1).ok_or("zero shards")?,
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 128,
            max_active_runs: 4,
            policy: vb_core::policy::RuntimePolicy::Relaxed,
        },
        shared,
    );
    let run = RunId::new(50);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(port, SlotValue::I64(40))]),
            CapabilitySet::empty(),
            contracts,
        )
        .map_err(|e| format!("submit: {e:?}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("initial tick: {e:?}"))?;
    let ticket = runtime
        .lookup_pending_action_ticket(run, u64::from(step.get()))
        .ok_or("source Do did not suspend with a real ticket")?;
    assert_eq!(ticket.step, step);
    assert_eq!(ticket.action, action);
    let value = SlotValue::I64(99);
    let bytes = postcard::to_allocvec(&value).map_err(|e| format!("encode: {e:?}"))?;
    runtime
        .complete_action_with_output(
            ticket,
            ActionOutputReady {
                output_slot: output,
                value,
                taint: Taint::Clean,
                encoded_len: u32::try_from(bytes.len()).map_err(|e| format!("length: {e:?}"))?,
            },
        )
        .map_err(|e| format!("completion: {e:?}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("completion tick: {e:?}"))?;
    let events = journal.snapshot().map_err(|e| format!("journal: {e:?}"))?;
    assert_eq!(events.iter().filter(|event| matches!(event,
        RuntimeJournalEvent::ActionScheduledTicket { ticket: actual, input: actual_input, output: actual_output, .. }
            if *actual == ticket && *actual_input == port && *actual_output == output
    )).count(), 1);
    assert_eq!(events.iter().filter(|event| matches!(event,
        RuntimeJournalEvent::ActionCompletedEnvelope { ticket: actual, output: actual_output, value: actual_value, taint: Taint::Clean, .. }
            if *actual == ticket && *actual_output == output && *actual_value == bytes
    )).count(), 1);
    assert!(events.iter().any(|event| matches!(event,
        RuntimeJournalEvent::RunFinished { run: actual, result, .. } if *actual == run && *result == expected_result
    )));
    assert!(matches!(
        runtime
            .snapshot_run(run, 2)
            .map_err(|e| format!("inspect: {e:?}"))?,
        InspectResponse::NotFound { .. }
    ));
    Ok(events)
}

#[test]
fn declared_input_flows_through_real_do_completion() -> Result<(), String> {
    execute_action(&source("inputs: {'n': number}\n",
        "  - id: invoke\n    do: {action: '7', input: '$input.n'}\n  - id: done\n    finish: {result: '$invoke'}\n"), ExpectedActionFinish::ActionOutput).map(|_| ())
}

#[test]
fn do_after_repeat_cannot_alias_private_counter_or_input() -> Result<(), String> {
    let text = source(
        "inputs: {'n': number}\n",
        "  - id: retry\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: body\n          set: {output: ignored, value: '8'}\n  - id: invoke\n    do: {action: '7', input: '$input.n'}\n  - id: done\n    finish: {result: '$invoke'}\n",
    );
    let workflow = compile(&text)?;
    let parts = workflow.to_parts();
    let counter = parts
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, CompiledNodeKind::RepeatStart { .. })
                .then_some(node.output)
                .flatten()
        })
        .ok_or("no Repeat counter")?;
    let action = parts
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, CompiledNodeKind::Do { .. })
                .then_some(node.output)
                .flatten()
        })
        .ok_or("no Do output")?;
    assert_ne!(counter, action);
    assert_ne!(
        counter,
        workflow.input_slots().first().ok_or("no input")?.slot
    );
    execute_action(&text, ExpectedActionFinish::ActionOutput).map(|_| ())
}

#[test]
fn output_alias_cannot_forge_a_declaration_root() -> Result<(), String> {
    let text = source(
        "",
        "  - id: first\n    set: {output: 'input.missing', value: '99'}\n  - id: done\n    finish: {result: '$input.missing'}\n",
    );
    rejects(&text, |e| {
        matches!(
            e,
            CompileError::InvalidName {
                field: "set.output",
                ..
            }
        )
    })
}

#[test]
fn branch_cannot_shadow_input_or_var_declarations() -> Result<(), String> {
    for output in [
        "input.enabled",
        "vars.amount",
        "$input.enabled",
        "$vars.amount",
    ] {
        let body = format!(
            "            - id: first\n              set: {{output: '{output}', value: '99'}}\n"
        );
        rejects(&choose_source("true", &body, "$n"), |e| {
            matches!(
                e,
                CompileError::InvalidName {
                    field: "set.output",
                    ..
                }
            )
        })?;
    }
    Ok(())
}

#[test]
fn numeric_choose_slot_with_known_integer_type_is_rejected() -> Result<(), String> {
    rejects(&choose_source("0", TWO_STEP_BODY, "$n"), |e| {
        matches!(
            e,
            CompileError::TypeMismatch {
                field: "choose.branches[].when",
                expected: "boolean",
                ..
            }
        )
    })
}

fn action_branch_source(condition: &str) -> String {
    source(
        "inputs: {'n': number}\n",
        &format!(
            "  - id: branch\n    choose:\n      branches:\n        - when: '{condition}'\n          steps:\n            - id: invoke\n              do: {{action: '7', input: '$input.n'}}\n            - id: copy\n              set: {{output: local, value: '$step.invoke + 1'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$input.n'}}\n"
        ),
    )
}

#[test]
fn selected_branch_do_completes_before_local_expression() -> Result<(), String> {
    let text = action_branch_source("true");
    let parts = compile(&text)?.to_parts();
    let local = parts
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, CompiledNodeKind::EvalExpr { .. })
                .then_some(node.output)
                .flatten()
        })
        .ok_or("missing body expression output")?;
    let events = execute_action(&text, ExpectedActionFinish::DeclaredInput)?;
    let encoded =
        postcard::to_allocvec(&SlotValue::I64(100)).map_err(|e| format!("encode: {e:?}"))?;
    assert!(events.iter().any(|event| matches!(event,
        vb_runtime::journal::RuntimeJournalEvent::SlotWritten { slot, value, taint: Taint::Clean, .. }
            if *slot == local && *value == encoded
    )));
    Ok(())
}

#[test]
fn unselected_branch_never_dispatches_do() -> Result<(), String> {
    // With no contracts, reaching Do would fail rather than silently succeed.
    let (workflow, frame) = execute(
        &action_branch_source("false"),
        Some(SlotValue::I64(40)),
        SlotValue::I64(40),
    )?;
    for node in &workflow.to_parts().nodes {
        if let CompiledNodeKind::Do { .. } = node.kind {
            assert!(
                frame
                    .read_slot(node.output.ok_or("missing action output")?)
                    .is_err()
            );
        }
    }
    Ok(())
}

#[test]
fn sibling_branch_names_are_not_visible() -> Result<(), String> {
    let text = source(
        "",
        "  - id: first\n    set: {output: outer, value: '40'}\n  - id: branch\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: left\n              set: {output: hidden, value: '1'}\n        - when: 'false'\n          steps:\n            - id: right\n              set: {output: copy, value: '$hidden'}\n      otherwise: done\n  - id: done\n    finish: {result: outer}\n",
    );
    rejects(
        &text,
        |e| matches!(e, CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "hidden"),
    )
}

#[test]
fn genuine_alias_and_step_id_ambiguity_is_rejected() -> Result<(), String> {
    let text = source(
        "",
        "  - id: 'n'\n    set: {output: left, value: '1'}\n  - id: second\n    set: {output: 'n', value: '2'}\n  - id: copy\n    set: {output: answer, value: '$n'}\n  - id: done\n    finish: {result: answer}\n",
    );
    rejects(
        &text,
        |e| matches!(e, CompileError::ExpressionLoweringUnsupported { feature } if feature.contains("ambiguous")),
    )
}

fn drive_frame(
    workflow: &CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
) -> Result<RuntimeSignal, String> {
    drive_deterministic_full(
        workflow,
        frame,
        &mut StepBudget::new(128),
        &mut ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|e| format!("source drive: {e:?}"))
}

#[test]
fn two_differently_typed_inputs_resolve_to_their_own_ports() -> Result<(), String> {
    let workflow = compile(&scalar_source(
        "inputs: {enabled: boolean, 'n': number}\n",
        "$input.enabled and $input.n > 0",
    ))?;
    let original = workflow.to_parts();
    let mut frame = new_run_frame(RunId::new(1), &workflow).map_err(|e| format!("frame: {e:?}"))?;
    let ports = workflow.input_slots();
    assert_eq!(ports.len(), 2);
    let enabled = ports.first().ok_or("missing enabled port")?;
    let n = ports.get(1).ok_or("missing number port")?;
    assert_ne!(enabled.slot, n.slot);
    assert_eq!(enabled.kind, vb_core::InputSlotKind::Bool);
    assert_eq!(n.kind, vb_core::InputSlotKind::I64);
    frame
        .write_slot(enabled.slot, SlotValue::Bool(true))
        .map_err(|e| format!("bool input: {e:?}"))?;
    frame
        .write_slot(n.slot, SlotValue::I64(40))
        .map_err(|e| format!("number input: {e:?}"))?;
    assert_eq!(
        drive_frame(&workflow, &mut frame)?,
        RuntimeSignal::Finished(SlotValue::Bool(true))
    );
    assert_eq!(
        *frame
            .read_slot(n.slot)
            .map_err(|e| format!("number preserved: {e:?}"))?,
        SlotValue::I64(40)
    );
    assert_eq!(
        *frame
            .read_slot(enabled.slot)
            .map_err(|e| format!("bool preserved: {e:?}"))?,
        SlotValue::Bool(true)
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}

#[test]
fn declared_input_expression_preserves_derived_taint() -> Result<(), String> {
    let text = source(
        "inputs: {'n': number}\n",
        "  - id: calculate\n    set: {output: derived, value: '$input.n + 2'}\n  - id: public\n    set: {output: clean, value: '99'}\n  - id: done\n    finish: {result: clean}\n",
    );
    let workflow = compile(&text)?;
    let original = workflow.to_parts();
    let input = workflow
        .input_slots()
        .first()
        .ok_or("missing input port")?
        .slot;
    let derived = original
        .nodes
        .iter()
        .find_map(|node| {
            matches!(node.kind, CompiledNodeKind::EvalExpr { .. })
                .then_some(node.output)
                .flatten()
        })
        .ok_or("missing expression result")?;
    let mut frame = new_run_frame(RunId::new(1), &workflow).map_err(|e| format!("frame: {e:?}"))?;
    frame
        .write_slot_with_taint(input, SlotValue::I64(40), Taint::DerivedFromSecret)
        .map_err(|e| format!("tainted declared input: {e:?}"))?;
    assert_eq!(
        drive_frame(&workflow, &mut frame)?,
        RuntimeSignal::Finished(SlotValue::I64(99))
    );
    assert_eq!(
        *frame
            .read_slot(derived)
            .map_err(|e| format!("derived value: {e:?}"))?,
        SlotValue::I64(42)
    );
    assert_eq!(
        frame
            .read_taint(derived)
            .map_err(|e| format!("derived taint: {e:?}"))?,
        Taint::DerivedFromSecret
    );
    assert_eq!(
        frame
            .read_taint(input)
            .map_err(|e| format!("input taint: {e:?}"))?,
        Taint::DerivedFromSecret
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}
