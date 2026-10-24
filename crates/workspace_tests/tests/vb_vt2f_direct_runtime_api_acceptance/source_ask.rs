use super::*;
use vb_compile::compile_workflow;

#[test]
fn canonical_named_ask_ticket_and_answer() -> Result<(), String> {
    let source = r#"
version: velvet-ballistics/v1
name: suspend
when:
  manual: {}
steps:
  - id: seed
    set:
      output: alias
      value: "42"
  - id: clock
    set:
      output: duration
      value: "600000"
  - id: question
    ask:
      prompt: "$alias"
      timeout: "$steps.clock"
  - id: done
    finish:
      result: "$seed"
"#;

    let compiled =
        compile_workflow(source.as_bytes()).map_err(|e| format!("compile failed: {e:?}"))?;

    assert_eq!(compiled.node_count(), 5);

    let node_question = compiled.node(StepIdx::new(2)).ok_or("node 2 missing")?;
    assert_eq!(node_question.id, StepIdx::new(2));
    assert_eq!(node_question.next, Some(StepIdx::new(3)));

    let CompiledNodeKind::Ask {
        prompt,
        timeout_slot,
    } = &node_question.kind
    else {
        return Err("node 2 should be Ask".into());
    };
    assert_eq!(*prompt, SlotIdx::new(0));
    assert_eq!(*timeout_slot, Some(SlotIdx::new(1)));

    let node_resume = compiled.node(StepIdx::new(3)).ok_or("node 3 missing")?;
    assert_eq!(node_resume.id, StepIdx::new(3));
    assert_eq!(node_resume.next, Some(StepIdx::new(4)));
    let CompiledNodeKind::AskResume { answer } = &node_resume.kind else {
        return Err("node 3 should be AskResume".into());
    };
    let answer_slot = *answer;
    assert_eq!(node_question.output, Some(answer_slot));
    assert_eq!(node_resume.output, Some(answer_slot));

    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(2001);
    runtime
        .submit_direct(run, compiled)
        .map_err(|e| e.to_string())?;
    run_one_tick(&mut runtime)?;

    let ticket = runtime
        .lookup_pending_ask_ticket(run, StepIdx::new(2))
        .ok_or("expected pending ticket")?;
    assert_eq!(ticket.run(), run);
    assert_eq!(ticket.ask_step(), StepIdx::new(2));
    assert_eq!(ticket.resume_step(), StepIdx::new(3));
    assert_eq!(ticket.answer_slot(), answer_slot);

    let answer = AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(99),
        Taint::Clean,
    );
    runtime.answer_ask(answer).map_err(|e| e.to_string())?;
    run_one_tick(&mut runtime)?;

    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);

    let events = journal_events(&journal)?;
    assert!(events.contains(&RuntimeJournalEvent::AskScheduled {
        run,
        step: StepIdx::new(2)
    }));
    assert!(events.contains(&RuntimeJournalEvent::AskAnswered {
        run,
        step: StepIdx::new(2),
        slot: answer_slot
    }));
    assert!(events.contains(&RuntimeJournalEvent::SlotWritten {
        run,
        slot: answer_slot,
        value: encoded(&SlotValue::I64(99))?,
        taint: Taint::Clean,
        extra: None
    }));

    Ok(())
}
