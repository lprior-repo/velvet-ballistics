use vb_core::{CompiledWorkflow, RunId, SlotIdx, SlotValue, StepIdx, Taint};
use vb_storage::{DecodedSlotWrittenExtra, JournalEvent, decode_slot_written_extra};

use crate::support::TestResult;

type ExpectedWrite = (SlotValue, Taint);

const FIRST_RUN_ATTEMPT: u16 = 1;

struct ExpectedEffect {
    name: &'static str,
    lowered_name: &'static str,
    expression: bool,
    attempt: u16,
    writes: Vec<ExpectedWrite>,
}

pub(super) fn assert_family_effects(
    events: &[JournalEvent],
    run: RunId,
    workflow: &CompiledWorkflow,
    family: &str,
) -> TestResult {
    let effects = expected_effects(family)?;
    let mut previous_last = None;
    for effect in effects {
        let step = step_id(workflow, family, &effect)?;
        let slot = output_slot(workflow, step)?;
        let writes = writes_for_step(events, run, step, slot, effect.attempt)?;
        let attempts: Vec<u16> = writes.iter().map(|(_, attempt, ..)| *attempt).collect();
        assert_eq!(
            attempts,
            vec![effect.attempt; effect.writes.len()],
            "{family} {} write attempts",
            effect.name
        );
        let values: Vec<ExpectedWrite> = writes
            .iter()
            .map(|(_, _, value, taint)| (*value, *taint))
            .collect();
        assert_eq!(values, effect.writes, "{family} {} writes", effect.name);
        assert_eq!(
            step_count(events, run, step, true),
            effect.writes.len(),
            "{family} {} starts",
            effect.name
        );
        assert_eq!(
            step_count(events, run, step, false),
            effect.writes.len(),
            "{family} {} successes",
            effect.name
        );
        let first = writes
            .first()
            .map(|(seq, ..)| *seq)
            .ok_or("effect has no first write")?;
        let last = writes
            .last()
            .map(|(seq, ..)| *seq)
            .ok_or("effect has no last write")?;
        if previous_last.is_some_and(|previous| first <= previous) {
            return Err(format!("{family} {} output order regressed", effect.name).into());
        }
        previous_last = Some(last);
    }
    Ok(())
}

fn expected_effects(family: &str) -> TestResult<Vec<ExpectedEffect>> {
    let clean = Taint::Clean;
    let integer = |value| (SlotValue::I64(value), clean);
    let effect = |name, lowered_name, expression, writes| ExpectedEffect {
        name,
        lowered_name,
        expression,
        attempt: FIRST_RUN_ATTEMPT,
        writes,
    };
    match family {
        "choose" => Ok(vec![effect("took_a", "took_a", false, vec![integer(7)])]),
        "for_each" => Ok(vec![effect(
            "body",
            "body",
            true,
            vec![integer(1), integer(2), integer(3)],
        )]),
        "together" => Ok(vec![
            effect("left_value", "left_value", false, vec![integer(7)]),
            effect("right_value", "right_value", false, vec![integer(9)]),
        ]),
        "collect" => Ok(vec![effect(
            "body",
            "body",
            true,
            vec![integer(3), integer(3)],
        )]),
        "reduce" => Ok(vec![effect(
            "body",
            "folded",
            true,
            vec![integer(1), integer(2), integer(3)],
        )]),
        "repeat" => Ok(vec![effect(
            "body",
            "body",
            false,
            vec![integer(7), integer(7)],
        )]),
        "on_error" => Ok(vec![effect(
            "recovered",
            "recovered",
            false,
            vec![integer(7)],
        )]),
        _ => Err(format!("missing effect oracle for {family}").into()),
    }
}

fn step_id(
    workflow: &CompiledWorkflow,
    family: &str,
    effect: &ExpectedEffect,
) -> TestResult<StepIdx> {
    (0..workflow.node_count())
        .map(StepIdx::new)
        .find(|step| {
            workflow.step_name(*step) == Some(effect.lowered_name)
                && workflow
                    .node(*step)
                    .is_some_and(|node| match effect.expression {
                        true => matches!(node.kind, vb_core::CompiledNodeKind::EvalExpr { .. }),
                        false => matches!(node.kind, vb_core::CompiledNodeKind::SetConst { .. }),
                    })
        })
        .ok_or_else(|| format!("missing {} effect step {}", family, effect.name).into())
}

fn output_slot(workflow: &CompiledWorkflow, step: StepIdx) -> TestResult<SlotIdx> {
    workflow
        .node(step)
        .and_then(|node| node.output.slot())
        .ok_or_else(|| format!("effect step {step:?} has no output slot").into())
}

fn step_count(events: &[JournalEvent], run: RunId, step: StepIdx, started: bool) -> usize {
    events
        .iter()
        .filter(|event| match (started, event) {
            (
                true,
                JournalEvent::StepStarted {
                    run: actual,
                    step: found,
                    ..
                },
            ) => *actual == run && *found == step,
            (
                false,
                JournalEvent::StepSucceeded {
                    run: actual,
                    step: found,
                    ..
                },
            ) => *actual == run && *found == step,
            _ => false,
        })
        .count()
}

fn writes_for_step(
    events: &[JournalEvent],
    run: RunId,
    step: StepIdx,
    slot: SlotIdx,
    expected_attempt: u16,
) -> TestResult<Vec<(u64, u16, SlotValue, Taint)>> {
    let mut active_attempt = None;
    let mut writes = Vec::new();
    for event in events {
        track_step(event, run, step, expected_attempt, &mut active_attempt)?;
        if let Some(write) = slot_write(event, run, slot, active_attempt, expected_attempt) {
            writes.push(write?);
        }
    }
    if active_attempt.is_some() {
        return Err(format!("effect step {step:?} did not complete").into());
    }
    Ok(writes)
}

fn track_step(
    event: &JournalEvent,
    run: RunId,
    step: StepIdx,
    expected_attempt: u16,
    active_attempt: &mut Option<u16>,
) -> TestResult {
    match event {
        JournalEvent::StepStarted {
            run: actual,
            step: started,
            attempt,
            ..
        } if *actual == run && *started == step => {
            if *attempt != expected_attempt {
                return Err(format!(
                    "effect step {step:?} started with attempt {attempt}, expected {expected_attempt}"
                )
                .into());
            }
            if active_attempt.replace(*attempt).is_some() {
                return Err(format!("effect step {step:?} started before prior completion").into());
            }
        }
        JournalEvent::StepSucceeded {
            run: actual,
            step: completed,
            ..
        } if *actual == run && *completed == step => {
            let Some(attempt) = active_attempt.take() else {
                return Err(format!("effect step {step:?} completed without start").into());
            };
            if attempt != expected_attempt {
                return Err(format!(
                    "effect step {step:?} completed attempt {attempt}, expected {expected_attempt}"
                )
                .into());
            }
        }
        JournalEvent::StepFailed {
            run: actual,
            step: failed,
            ..
        } if *actual == run && *failed == step => {
            return Err(format!("effect step {step:?} failed").into());
        }
        _ => {}
    }
    Ok(())
}

fn slot_write(
    event: &JournalEvent,
    run: RunId,
    slot: SlotIdx,
    active_attempt: Option<u16>,
    expected_attempt: u16,
) -> Option<TestResult<(u64, u16, SlotValue, Taint)>> {
    match event {
        JournalEvent::SlotWrittenEvent {
            run: actual,
            seq,
            slot: written,
            attempt,
            value: Some(bytes),
            extra: Some(extra),
        } if *actual == run && *written == slot && active_attempt.is_some() => {
            let result = if *attempt != expected_attempt {
                Err(format!(
                    "effect slot {slot:?} written with attempt {attempt}, expected {expected_attempt}"
                )
                .into())
            } else if active_attempt != Some(*attempt) {
                Err(format!(
                    "effect slot {slot:?} attempt {attempt} does not match active attempt {active_attempt:?}"
                )
                .into())
            } else {
                decode_write(slot, seq.get(), *attempt, bytes, extra)
            };
            Some(result)
        }
        _ => None,
    }
}

fn decode_write(
    slot: SlotIdx,
    seq: u64,
    attempt: u16,
    bytes: &[u8],
    extra: &[u8],
) -> TestResult<(u64, u16, SlotValue, Taint)> {
    let value = postcard::from_bytes::<SlotValue>(bytes)?;
    let decoded = decode_slot_written_extra(extra)
        .map_err(|error| format!("effect slot {slot:?} extra: {error:?}"))?;
    let taint = match decoded {
        DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
        DecodedSlotWrittenExtra::LegacyFrameExtra(_) => {
            return Err(format!("effect slot {slot:?} has legacy taint envelope").into());
        }
    };
    Ok((seq, attempt, value, taint))
}
