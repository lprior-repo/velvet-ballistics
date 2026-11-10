//! Cold F3 acceptance agrees with actual runtime-issued Ask authority.
use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx,
    Taint, WorkflowParts,
};
use vb_runtime::{
    journal::{RuntimeJournalEvent, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::{AskAnswer, ShardConfig},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn legacy_parts() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    use vb_core::workflow::{ResourceContract, node, source_metadata::WorkflowSourceMetadata};
    use vb_core::{ConstIdx, ConstValue, SymbolId, WorkflowDigest};
    Ok(WorkflowParts {
        name: "legacy-host-answer".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: Some(Box::new(WorkflowSourceMetadata::try_new(
            Box::from([Box::<str>::from("Continue?")]),
            Box::new([]),
        )?)),
        nodes: Box::from([
            node(
                StepIdx::ZERO,
                K::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::ZERO),
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                K::Ask {
                    prompt: SlotIdx::ZERO,
                    timeout_slot: None,
                },
                None,
                Some(StepIdx::new(2)),
            ),
            node(
                StepIdx::new(2),
                K::AskResume {
                    answer: SlotIdx::new(1),
                },
                Some(SlotIdx::new(2)),
                Some(StepIdx::new(3)),
            ),
            node(
                StepIdx::new(3),
                K::Finish {
                    result: SlotIdx::new(2),
                },
                None,
                None,
            ),
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::from([ConstValue::Symbol(SymbolId::new(0))]),
        slot_count: 3,
        symbols_count: 1,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    })
}

fn host_resume(workflow: CompiledWorkflow, answer_is_result: bool) -> TestResult {
    let parts = workflow.to_parts();
    vb_validate::shared::validate(&parts)?;
    let (ask_step, resume_step, answer) = parts
        .nodes
        .iter()
        .find_map(|n| {
            if !matches!(n.kind, K::Ask { .. }) {
                return None;
            }
            let next = n.routing.next.step()?;
            match workflow.node(next)?.kind {
                K::AskResume { answer } => Some((n.id, next, answer)),
                _ => None,
            }
        })
        .ok_or("missing Ask boundary")?;
    let finish = parts
        .nodes
        .iter()
        .find_map(|n| match n.kind {
            K::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or("missing Finish")?;
    let resume_output = workflow
        .node(resume_step)
        .and_then(|n| n.output.slot())
        .ok_or("missing AskResume output")?;
    let run = RunId::new(1);
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            ..ShardConfig::default()
        },
        journal.clone(),
    );
    runtime.submit_direct(run, workflow)?;
    let mut ticket = None;
    for _ in 0..8 {
        runtime.tick_all()?;
        ticket = runtime.lookup_pending_ask_ticket(run, ask_step);
        if ticket.is_some() {
            break;
        }
    }
    let ticket = ticket.ok_or_else(|| {
        format!(
            "Ask never suspended: {:?}; {:?}",
            runtime.counters_snapshot(),
            journal.snapshot()
        )
    })?;
    assert_eq!(ticket.resume_step(), resume_step);
    assert_eq!(ticket.answer_slot(), answer);
    assert_eq!(
        runtime.inspect_run_state(run)?.frame.read_slot(answer),
        Err(vb_core::EngineError::SlotUninitialized { slot: answer })
    );
    assert_eq!(runtime.counters_snapshot().runs_completed, 0);
    let expected_result = if answer_is_result {
        (SlotValue::I64(42), Taint::Clean)
    } else {
        let state = runtime.inspect_run_state(run)?;
        (
            *state.frame.read_slot(finish)?,
            state.frame.read_taint(finish)?,
        )
    };
    runtime.answer_ask(AskAnswer::new(
        ticket,
        answer,
        SlotValue::I64(42),
        Taint::Clean,
    ))?;
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    let events = journal.snapshot()?;
    for slot in [answer, resume_output] {
        assert!(events.contains(&RuntimeJournalEvent::SlotWritten {
            run,
            slot,
            value: postcard::to_allocvec(&SlotValue::I64(42))?,
            taint: Taint::Clean,
            extra: None,
        }));
    }
    assert!(events.contains(&RuntimeJournalEvent::SlotWritten {
        run,
        slot: finish,
        value: postcard::to_allocvec(&expected_result.0)?,
        taint: expected_result.1,
        extra: None,
    }));
    assert!(events.contains(&RuntimeJournalEvent::AskAnswered {
        run,
        step: ask_step,
        slot: answer
    }));
    assert!(events.contains(&RuntimeJournalEvent::StepSucceeded {
        run,
        step: resume_step,
        output: resume_output,
        attempt: 1,
    }));
    assert!(events.contains(&RuntimeJournalEvent::RunFinished {
        run,
        result: finish,
        attempt: 1
    }));
    assert_eq!(runtime.lookup_pending_ask_ticket(run, ask_step), None);
    Ok(())
}

#[test]
fn hand_ir_output_none_reaches_real_host_answer_and_finish() -> TestResult {
    host_resume(CompiledWorkflow::try_from_parts(legacy_parts()?)?, true)
}

#[test]
fn yaml_lowered_ask_reaches_real_host_answer_and_finish() -> TestResult {
    let workflow = vb_compile::compile_workflow(
        br#"version: velvet-ballistics/v1
name: yaml_host_answer
when: {manual: {}}
steps:
  - id: prompt
    set: {output: question, value: '"Continue?"'}
  - id: response
    ask: {prompt: '$prompt'}
  - id: done
    finish: {result: '$prompt'}
"#,
    )?;
    host_resume(workflow, false)
}

#[test]
fn public_verifier_rejects_ask_error_and_resume_bypass() -> TestResult {
    let mut error = legacy_parts()?;
    error.nodes.get_mut(1).ok_or("missing Ask")?.routing.error =
        vb_core::workflow::ErrorRouting::Handler {
            handler: StepIdx::new(2),
            error_slot: None,
        };
    let mut bypass = legacy_parts()?;
    bypass.nodes.get_mut(0).ok_or("missing branch")?.kind = K::ChooseSlot {
        branches: Box::from([vb_core::workflow::SlotBranch {
            condition: SlotIdx::ZERO,
            target: StepIdx::new(1),
        }]),
        otherwise: Some(StepIdx::new(2)),
    };
    bypass.nodes.get_mut(0).ok_or("missing branch")?.output = None.into();
    bypass
        .nodes
        .get_mut(0)
        .ok_or("missing branch")?
        .routing
        .next = None.into();
    bypass.nodes.get_mut(1).ok_or("missing Ask")?.kind = K::Ask {
        prompt: SlotIdx::new(3),
        timeout_slot: None,
    };
    bypass.slot_count = 4;
    bypass.input_slots = Box::from([
        vb_core::CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: vb_core::InputSlotKind::Bool,
            constraint: vb_core::CompiledInputConstraint::required(),
        },
        vb_core::CompiledInputSlot {
            slot: SlotIdx::new(3),
            kind: vb_core::InputSlotKind::I64,
            constraint: vb_core::CompiledInputConstraint::required(),
        },
    ]);
    for p in [error, bypass] {
        let expected = vb_core::workflow::SlotFlowError {
            step: StepIdx::new(2),
            slot: Some(SlotIdx::new(1)),
            reason: vb_core::workflow::SlotFlowFailure::UninitializedRead,
        };
        assert_eq!(
            vb_core::workflow::validate_slot_flow(&p),
            Err(expected.clone())
        );
        assert_eq!(
            vb_validate::shared::validate_gate_14_slot_type_consistency(&p),
            Err(vb_validate::ValidationError::TypeMismatch {
                expected: "valid compiled slot flow".into(),
                found: expected.to_string(),
            })
        );
    }
    Ok(())
}
