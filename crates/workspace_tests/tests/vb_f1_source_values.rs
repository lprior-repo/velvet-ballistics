#![forbid(unsafe_code)]
use vb_compile::compile_workflow;
use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, ValueStore, new_run_frame};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

fn drive(
    source: &str,
) -> Result<(RuntimeSignal, vb_core::RunFrame, ValueStore), Box<dyn std::error::Error>> {
    let workflow = compile_workflow(source.as_bytes())?;
    let payload = postcard::to_allocvec(&workflow.to_parts())?;
    let decoded = vb_core::CompiledWorkflow::try_from_parts(postcard::from_bytes(&payload)?)?;
    assert_eq!(decoded, workflow);
    let mut frame = new_run_frame(RunId::new(1), &decoded)?;
    let mut store = ValueStore::new();
    if let Some(metadata) = decoded.source_metadata() {
        vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
    }
    let contracts = [vb_core::ActionContract {
        id: vb_core::ActionId::new(0),
        name: vb_core::action::ActionName::new("example.action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        timeout_ms: 1000,
        idempotency: vb_core::Idempotency::DeterministicPure,
        side_effect: vb_core::SideEffect::None,
        retry_safety: vb_core::RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }];
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    for _ in 0..64 {
        let signal = drive_deterministic_full(
            &decoded,
            &mut frame,
            &mut StepBudget::new(1),
            &mut store,
            &contracts,
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?;
        if signal != RuntimeSignal::StepBudgetExhausted {
            return Ok((signal, frame, store));
        }
    }
    Err("workflow failed to reach an observable outcome".into())
}

#[test]
fn composite_variables_accessors_text_and_result_survive_artifact_roundtrip()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "version: velvet-ballistics/v1\nname: source_values\nwhen: {manual: {}}\nvars:\n  config: {message: hello, items: [3, 7]}\nsteps:\n  - id: compare\n    set: {output: same, value: '$vars.config.message == \"hello\"'}\nresult:\n  matches: $same\n  answer: $vars.config.items.1\n  literal: world\n";
    let (signal, _, store) = drive(source)?;
    let RuntimeSignal::Finished(SlotValue::Object(id)) = signal else {
        return Err(format!("{signal:?}").into());
    };
    let fields = store.object(id)?;
    let mut values = std::collections::BTreeMap::new();
    for field in fields {
        values.insert(store.symbol(field.key)?, field.value);
    }
    assert_eq!(values.get("matches"), Some(&SlotValue::Bool(true)));
    assert_eq!(values.get("answer"), Some(&SlotValue::I64(7)));
    let Some(SlotValue::Symbol(id)) = values.get("literal") else {
        return Err("missing result text".into());
    };
    assert_eq!(store.symbol(*id)?, "world");
    Ok(())
}

#[test]
fn structured_with_materializes_action_input_and_guard_skips_it()
-> Result<(), Box<dyn std::error::Error>> {
    for (key, field) in [
        (
            "outer",
            "    with: {message: hello, value: '$answer'}\n    do: {action: '0'}",
        ),
        (
            "inner",
            "    do: {action: '0', with: {message: hello, value: '$answer'}}",
        ),
    ] {
        for guard in [true, false] {
            let source = format!(
                "version: velvet-ballistics/v1\nname: with_{key}\nwhen: {{manual: {{}}}}\nsteps:\n  - id: initial\n    set: {{output: answer, value: '42'}}\n  - id: action\n    if: '{guard}'\n{field}\n  - id: done\n    finish: {{result: answer}}\n"
            );
            let (signal, frame, store) = drive(&source)?;
            if guard {
                let RuntimeSignal::AwaitingAction(ticket) = signal else {
                    return Err(format!("{signal:?}").into());
                };
                let workflow = compile_workflow(source.as_bytes())?;
                let Some(vb_core::CompiledNodeKind::Do { input, .. }) =
                    workflow.node(ticket.step).map(|node| &node.kind)
                else {
                    return Err("missing action".into());
                };
                let SlotValue::Object(id) = frame.read_slot(*input)? else {
                    return Err("with input was not an object".into());
                };
                let fields = store.object(*id)?;
                assert_eq!(fields.len(), 2);
                for field in fields {
                    match store.symbol(field.key)? {
                        "message" => {
                            let SlotValue::Symbol(id) = field.value else {
                                return Err("missing message".into());
                            };
                            assert_eq!(store.symbol(id)?, "hello");
                        }
                        "value" => assert_eq!(field.value, SlotValue::I64(42)),
                        other => return Err(format!("unexpected field {other}").into()),
                    }
                }
            } else {
                assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(42)));
            }
        }
    }
    Ok(())
}
