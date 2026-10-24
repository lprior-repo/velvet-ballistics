use super::*;
use vb_compile::compile_workflow;
use vb_runtime::shard::PendingTimerKind;

pub(super) const SOURCE: &str = r#"version: velvet-ballistics/v1
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

fn run_named_wait(event: bool) -> Result<(), String> {
    let source = if event {
        SOURCE.replace("ask:", "wait:").replace("prompt:", "event:")
    } else {
        let s = SOURCE.replace("ask:", "wait:");
        // Remove the entire prompt line and its newline for WaitUntil
        s.replace("      prompt: \"$alias\"\n", "")
    };

    let compiled =
        compile_workflow(source.as_bytes()).map_err(|e| format!("compile failed: {e:?}"))?;

    assert_eq!(compiled.node_count(), 4);

    let node = compiled.node(StepIdx::new(2)).ok_or("node 2 missing")?;
    assert_eq!(node.id, StepIdx::new(2));
    assert_eq!(node.next, Some(StepIdx::new(3)));

    if event {
        let CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } = &node.kind
        else {
            return Err("node 2 should be WaitEvent".into());
        };
        assert_eq!(*event, SlotIdx::new(0));
        assert_eq!(*timeout_slot, Some(SlotIdx::new(1)));
    } else {
        let CompiledNodeKind::WaitUntil { deadline_slot } = &node.kind else {
            return Err("node 2 should be WaitUntil".into());
        };
        assert_eq!(*deadline_slot, SlotIdx::new(1));
    }

    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config(), journal.clone());
    let run = RunId::new(2001);
    runtime
        .submit_direct(run, compiled)
        .map_err(|e| format!("submit failed: {e:?}"))?;
    run_one_tick(&mut runtime)?;

    let entry = runtime
        .capture_timer_entry(run)
        .map_err(|e| format!("capture_timer_entry failed: {e:?}"))?;
    assert_eq!(entry.run, run);
    assert_eq!(entry.kind, PendingTimerKind::Wait);

    assert_eq!(runtime.counters_snapshot().runs_completed, 0);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);

    runtime
        .timer_entry_fired(entry)
        .map_err(|e| format!("timer_entry_fired failed: {e:?}"))?;
    run_one_tick(&mut runtime)?;

    assert_eq!(runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);

    let events = journal_events(&journal)?;
    assert!(events.contains(&RuntimeJournalEvent::WaitScheduled {
        run,
        step: StepIdx::new(2),
    }));
    assert!(events.contains(&RuntimeJournalEvent::WaitResolved {
        run,
        step: StepIdx::new(2),
    }));

    Ok(())
}

#[test]
fn wait_event_timeout_fires() -> Result<(), String> {
    run_named_wait(true)
}

#[test]
fn wait_until_deadline_fires() -> Result<(), String> {
    run_named_wait(false)
}
