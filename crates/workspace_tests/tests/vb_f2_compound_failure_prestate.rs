use vb_core::{
    CapabilitySet, CompiledNodeKind as K, RunId, SlotValue, StepBudget, StepState, Taint,
    ValueStore, new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn allocation_failure(family: &str, next: bool) -> TestResult {
    let compound = match family {
        "for_each" => {
            "for_each:\n      variable: entry_value\n      input: $input.items\n      at_once: 1"
        }
        "reduce" => {
            "reduce:\n      variable: accumulator\n      input: $input.items\n      initial: '0'"
        }
        "collect" => {
            "collect:\n      variable: batch_value\n      source: $input.items\n      pages: 2\n      items: 1"
        }
        _ => return Err("unknown family".into()),
    };
    let yaml = format!(
        "version: velvet-ballistics/v1\nname: compound_failure_prestate\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: 2}}}}\nsteps:\n  - id: seed\n    set: {{output: saved, value: '42'}}\n  - id: compound\n    on_error: {{handler: recovery}}\n    {compound}\n      steps:\n        - id: leaf\n          set: {{output: leaf, value: '7'}}\n  - id: recovery\n    finish: {{result: '$saved'}}\n"
    );
    let plan = vb_compile::compile_workflow(yaml.as_bytes())?;
    vb_validate::shared::validate(&plan.to_parts())?;
    let mut run = new_run_frame(RunId::new(1), &plan)?;
    let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
    let source = store.insert_list_with_taint(
        Box::from([SlotValue::I64(3), SlotValue::I64(5)]),
        Box::from([Taint::Secret, Taint::DerivedFromSecret]),
    )?;
    let input = plan.input_slots().first().ok_or("input")?.slot;
    run.write_slot_with_taint(input, SlotValue::List(source), Taint::Secret)?;
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    let mut found = false;
    for _ in 0..32 {
        let node = plan.node(run.pc()).ok_or("node")?;
        let target = matches!(
            (family, next, &node.kind),
            ("for_each", false, K::ForEachStart { .. })
                | ("for_each", true, K::ForEachNext { .. })
                | ("reduce", false, K::ReduceStart { .. })
                | ("reduce", true, K::ReduceNext { .. })
                | ("collect", false, K::CollectStart { .. })
                | ("collect", true, K::CollectNext { .. })
        );
        if target {
            let mut checkpoint = store.checkpoint_v1();
            checkpoint.max_arena_entries = store.total_arena_count();
            store = ValueStore::from_checkpoint_v1(checkpoint)?;
            assert_eq!(
                store.total_arena_count(),
                store.checkpoint_v1().max_arena_entries
            );
        }
        let before = run.initialized_slots()?;
        let stored = store.checkpoint_v1();
        let pages = collect.checkpoint_v1();
        let failed = run.pc();
        let signal = drive_deterministic_full(
            &plan,
            &mut run,
            &mut StepBudget::new(1),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?;
        assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
        if !target {
            assert!(
                !evidence
                    .drain()
                    .iter()
                    .any(|event| matches!(event, EvidenceEvent::StepFailed { .. }))
            );
            continue;
        }
        assert_eq!(run.step_state(failed)?, StepState::Failed);
        assert_eq!(run.pc(), node.routing.error.handler().ok_or("handler")?);
        assert_eq!(
            run.initialized_slots()?,
            before,
            "{family} next={next}: handler must see failed-node prestate"
        );
        assert_eq!(store.checkpoint_v1(), stored);
        assert_eq!(collect.checkpoint_v1(), pages);
        assert_eq!(*run.read_slot(input)?, SlotValue::List(source));
        assert_eq!(run.read_taint(input)?, Taint::Secret);
        let events = evidence.drain();
        assert_eq!(
            events
                .iter()
                .filter(
                    |event| matches!(event, EvidenceEvent::StepFailed { step } if *step == failed)
                )
                .count(),
            1
        );
        assert!(!events.iter().any(
            |event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == failed)
        ));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, EvidenceEvent::SlotWritten { .. }))
        );
        assert_eq!(
            drive_deterministic_full(
                &plan,
                &mut run,
                &mut StepBudget::new(1),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1
            )?,
            RuntimeSignal::Finished(SlotValue::I64(42))
        );
        found = true;
        break;
    }
    assert!(found, "{family} next={next}: failed node not reached");
    Ok(())
}

#[test]
fn for_each_start_handler_observes_prestate() -> TestResult {
    allocation_failure("for_each", false)
}
#[test]
fn for_each_next_handler_observes_prestate() -> TestResult {
    allocation_failure("for_each", true)
}
#[test]
fn reduce_start_handler_observes_prestate() -> TestResult {
    allocation_failure("reduce", false)
}
#[test]
fn reduce_next_handler_observes_prestate() -> TestResult {
    allocation_failure("reduce", true)
}
#[test]
fn collect_start_handler_observes_prestate() -> TestResult {
    allocation_failure("collect", false)
}
#[test]
fn collect_next_handler_observes_prestate() -> TestResult {
    allocation_failure("collect", true)
}
