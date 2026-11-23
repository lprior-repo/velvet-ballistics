include!("part_e_support.rs");

fn run_answer_cli(
    fixture: &mut AnswerCliFixture,
    selector_flag: &str,
    selector: u16,
    value_flag: &str,
    value: SlotValue,
    format: Option<&str>,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let value_file = fixture._directory.path().join("answer.postcard");
    std::fs::write(&value_file, postcard::to_allocvec(&value)?)?;
    let mut child = answer_process(
        fixture,
        selector_flag,
        selector,
        value_flag,
        &value_file,
        format,
    )?;
    for _ in 0..64 {
        fixture
            .server
            .poll_once(&mut fixture.runtime, Some(Duration::from_millis(5)))?;
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
    }
    child.kill()?;
    let _output = child.wait_with_output()?;
    Err(std::io::Error::other("answer CLI did not finish within 64 polls").into())
}

fn answer_process(
    fixture: &AnswerCliFixture,
    selector_flag: &str,
    selector: u16,
    value_flag: &str,
    value_file: &std::path::Path,
    format: Option<&str>,
) -> Result<std::process::Child, std::io::Error> {
    let mut command = ProcessCommand::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args(["answer", &fixture.run.get().to_string(), selector_flag]);
    command
        .arg(selector.to_string())
        .arg(value_flag)
        .arg(value_file);
    command.arg("--db").arg(&fixture.db);
    if let Some(format) = format {
        command.args(["--emit", format]);
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AnswerObservation {
    state: vb_runtime::shard::types::RunState,
    ticket: Option<AskTicket>,
    events: Vec<JournalEvent>,
    trace: Vec<TraceEvent>,
    counters: vb_runtime::counters::CounterSnapshot,
    queue_depth: usize,
}

fn observe_answer(
    fixture: &AnswerCliFixture,
    step: StepIdx,
) -> Result<AnswerObservation, Box<dyn std::error::Error>> {
    Ok(AnswerObservation {
        state: fixture.runtime.inspect_run_state(fixture.run)?.clone(),
        ticket: fixture.runtime.lookup_pending_ask_ticket(fixture.run, step),
        events: fixture.journal.events_for_run_full(fixture.run)?,
        trace: fixture.runtime.list_events(fixture.run)?,
        counters: fixture.runtime.counters_snapshot(),
        queue_depth: fixture
            .runtime
            .pending_boundary_snapshot(64)
            .command_queue_depth(),
    })
}

fn assert_answer_refusal_read_only(
    fixture: &mut AnswerCliFixture,
    ticket: AskTicket,
    selector_flag: &str,
    selector: u16,
    value_flag: &str,
    format: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = observe_answer(fixture, ticket.ask_step())?;
    let output = run_answer_cli(
        fixture,
        selector_flag,
        selector,
        value_flag,
        SlotValue::I64(5),
        format,
    )?;
    assert_cli_refusal(&output, format)?;
    fixture.runtime.tick_all()?;
    assert_eq!(observe_answer(fixture, ticket.ask_step())?, before);
    Ok(())
}

fn assert_legacy_answer_refusal_read_only(
    fixture: &mut AnswerCliFixture,
    ticket: AskTicket,
    format: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let before = observe_answer(fixture, ticket.ask_step())?;
    let output = run_answer_cli(
        fixture,
        "--step",
        ticket.ask_step().get(),
        "--value-file",
        SlotValue::I64(5),
        format,
    )?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert!(output_stderr(&output).starts_with("unknown flag for answer: --step\n\n"));
    assert!(output_stderr(&output).contains(
        "answer     <run_id> --slot <N> --value <file> --db <path>"
    ));
    fixture.runtime.tick_all()?;
    assert_eq!(observe_answer(fixture, ticket.ask_step())?, before);
    Ok(())
}

fn assert_cli_refusal(
    output: &std::process::Output,
    format: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output_stdout(output), "");
    let expected =
        serde_json::json!({"success": false, "error": "unexpected response: BadRequest"});
    match format {
        None => assert_eq!(output_stderr(output), "unexpected response: BadRequest\n"),
        Some("yaml") => assert_eq!(
            serde_saphyr::from_str::<serde_json::Value>(&output_stderr(output))?,
            expected
        ),
        Some("postcard") => assert_eq!(
            cli_postcard::decode_postcard_json(&output.stderr)?.1,
            expected
        ),
        Some(other) => return Err(format!("unexpected output format: {other}").into()),
    }
    Ok(())
}

fn assert_cli_answer(
    output: &std::process::Output,
    run: RunId,
    format: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output_stderr(output), "");
    match format {
        None => assert_eq!(
            output_stdout(output),
            format!("answer accepted for run {}\n", run.get())
        ),
        Some("yaml") => assert_eq!(
            serde_saphyr::from_str::<serde_json::Value>(&output_stdout(output))?,
            answer_success(run)
        ),
        Some("postcard") => assert_eq!(
            cli_postcard::decode_postcard_json(&output.stdout)?.1,
            answer_success(run)
        ),
        Some(other) => return Err(format!("unexpected output format: {other}").into()),
    }
    Ok(())
}

fn answer_success(run: RunId) -> serde_json::Value {
    serde_json::json!({"success": true, "run_id": run.get()})
}

fn assert_answer_write(
    events: &[JournalEvent],
    slot: SlotIdx,
    value: SlotValue,
) -> Result<(), Box<dyn std::error::Error>> {
    let writes = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot: actual,
                value: Some(bytes),
                extra: Some(extra),
                ..
            } if *actual == slot => Some((bytes, extra)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !writes.is_empty(),
        "slot {slot:?} has no retained answer write"
    );
    for (bytes, extra) in writes {
        assert_eq!(postcard::from_bytes::<SlotValue>(bytes)?, value);
        let taint = match decode_slot_written_extra(extra).map_err(|error| format!("{error:?}"))? {
            DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
            DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
                return Err("answer write lacks taint envelope".into());
            }
        };
        assert_eq!(taint, Taint::Clean);
    }
    Ok(())
}

fn assert_answer_effects(fixture: &AnswerCliFixture) -> Result<(), Box<dyn std::error::Error>> {
    let events = fixture.journal.events_for_run_full(fixture.run)?;
    assert_answer_write(&events, SlotIdx::new(1), SlotValue::I64(99))?;
    assert_answer_write(&events, SlotIdx::new(2), SlotValue::I64(101))?;
    assert_answer_write(&events, SlotIdx::new(3), SlotValue::I64(103))?;
    let answered = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::AskAnsweredEvent { run, step, .. } if *run == fixture.run => Some(*step),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(answered, fixture.asks);
    let trace = fixture.runtime.list_events(fixture.run)?;
    let answered_slots = trace
        .iter()
        .filter_map(|event| match event {
            TraceEvent::AskAnswered { step, slot, .. } => Some((*step, *slot)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        answered_slots,
        [
            (fixture.asks[0], SlotIdx::new(1)),
            (fixture.asks[1], SlotIdx::new(2)),
            (fixture.asks[2], SlotIdx::new(3))
        ]
    );
    let result_slot = events
        .iter()
        .find_map(|event| match event {
            JournalEvent::RunFinished { run, result, .. } if *run == fixture.run => Some(*result),
            _ => None,
        })
        .ok_or("missing completed run result")?;
    assert_answer_write(&events, result_slot, SlotValue::I64(103))?;
    Ok(())
}

#[test]
fn public_answer_cli_rejects_step_and_answers_by_slot_in_all_formats()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = answer_cli_fixture()?;
    let first = wait_for_ask(&mut fixture.runtime, fixture.run, fixture.asks[0])?;
    for format in [None, Some("yaml"), Some("postcard")] {
        assert_legacy_answer_refusal_read_only(&mut fixture, first, format)?;
    }
    let output = run_answer_cli(
        &mut fixture,
        "--slot",
        first.answer_slot().get(),
        "--value",
        SlotValue::I64(99),
        None,
    )?;
    assert_cli_answer(&output, fixture.run, None)?;
    let second = wait_for_ask(&mut fixture.runtime, fixture.run, fixture.asks[1])?;
    assert_eq!(
        (second.ask_step(), second.answer_slot()),
        (StepIdx::new(3), SlotIdx::new(2))
    );
    for format in [None, Some("yaml"), Some("postcard")] {
        assert_answer_refusal_read_only(&mut fixture, second, "--slot", 3, "--value", format)?;
    }
    let output = run_answer_cli(
        &mut fixture,
        "--slot",
        2,
        "--value",
        SlotValue::I64(101),
        Some("yaml"),
    )?;
    assert_cli_answer(&output, fixture.run, Some("yaml"))?;
    let third = wait_for_ask(&mut fixture.runtime, fixture.run, fixture.asks[2])?;
    let output = run_answer_cli(
        &mut fixture,
        "--slot",
        3,
        "--value",
        SlotValue::I64(103),
        Some("postcard"),
    )?;
    assert_cli_answer(&output, fixture.run, Some("postcard"))?;
    for _ in 0..64 {
        fixture.runtime.tick_all()?;
    }
    assert_answer_effects(&fixture)?;
    assert_eq!(
        fixture
            .runtime
            .pending_boundary_snapshot(64)
            .command_queue_depth(),
        0
    );
    assert_eq!(fixture.runtime.counters_snapshot().runs_completed, 1);
    assert_eq!(
        fixture
            .runtime
            .lookup_pending_ask_ticket(fixture.run, third.ask_step()),
        None
    );
    Ok(())
}
