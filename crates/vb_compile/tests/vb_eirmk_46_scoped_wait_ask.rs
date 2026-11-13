//! vb-eirmk.4.6: scoped Wait/Ask lowering with evidence-aware error/retry controls.
//!
//! Covers the bead contract through the registered `compile_workflow` entry
//! point: initial suspension/resume IR, `on_error` handler dispatch and
//! `try_again` retry controls via the shared dispatcher, timeout/deadline
//! binding, duplicate/stale answer rejection, lexical scope confinement, and
//! taint rejection of possibly-skipped suspension inputs. Named lexical
//! references reuse vb-8eocj.8; retry bounds reuse vb-sfhyj resource
//! contracts. Runtime ticket fencing (duplicate/stale Ask tickets) is owned
//! and tested by vb_runtime; this suite pins the compile-time IR contract.
//!
//! No code here emits filler `Nop` nodes: suspension flows lower directly to
//! `WaitUntil`/`WaitEvent` and `Ask`/`AskResume` pairs.

use vb_compile::{CompileError, CompileErrors, compile_workflow};
use vb_core::workflow::{CompiledRetryPolicy, ErrorRouting, NodeContinuation};
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx};

const HEADER: &str = "version: velvet-ballistics/v1\nname: scoped_wait_ask\nwhen: {manual: {}}\n";

fn steps(body: &str) -> String {
    format!("{HEADER}steps:\n{body}")
}

fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn node(workflow: &CompiledWorkflow, id: u16) -> Result<&CompiledNode, String> {
    workflow
        .node(StepIdx::new(id))
        .ok_or_else(|| format!("missing node {id}"))
}

fn out(workflow: &CompiledWorkflow, id: u16) -> Result<SlotIdx, String> {
    node(workflow, id)?
        .output
        .slot()
        .ok_or_else(|| format!("missing output {id}"))
}

fn rejects(source: &str, error: CompileError) -> Result<(), String> {
    match compile_workflow(source.as_bytes()) {
        Ok(_) => Err("unexpected source acceptance".to_owned()),
        Err(errors) => {
            if errors == CompileErrors(vec![error]) {
                Ok(())
            } else {
                Err(format!("wrong diagnostic: {errors:?}"))
            }
        }
    }
}

fn assert_handler(node: &CompiledNode, handler: u16) -> Result<(), String> {
    if node.routing.error
        == (ErrorRouting::Handler {
            handler: StepIdx::new(handler),
            error_slot: None,
        })
    {
        Ok(())
    } else {
        Err(format!(
            "expected handler {handler}, got {:?}",
            node.routing.error
        ))
    }
}

fn assert_no_nop(workflow: &CompiledWorkflow) -> Result<(), String> {
    if workflow
        .to_parts()
        .nodes
        .iter()
        .any(|n| matches!(n.kind, CompiledNodeKind::Nop))
    {
        return Err("filler Nop node emitted".to_owned());
    }
    Ok(())
}

const SEED: &str = "  - id: seed\n    set: {output: saved, value: '42'}\n";
const DELAY: &str = "  - id: tel\n    set: {output: delay, value: '5'}\n";
const DONE: &str = "  - id: done\n    finish: {result: 0}\n";

#[test]
fn ask_named_prompt_binds_prior_set_slot() -> Result<(), String> {
    let workflow = compile(&steps(&format!(
        "{SEED}  - id: q\n    ask: {{prompt: '$saved'}}\n{DONE}"
    )))?;
    let seed_slot = out(&workflow, 0)?;
    let CompiledNodeKind::Ask {
        prompt,
        timeout_slot,
    } = &node(&workflow, 1)?.kind
    else {
        return Err("expected Ask at node 1".to_owned());
    };
    if *prompt != seed_slot || timeout_slot.slot().is_some() {
        return Err(format!(
            "wrong ask slots prompt={prompt:?} timeout={timeout_slot:?}"
        ));
    }
    let answer = out(&workflow, 1)?;
    if node(&workflow, 1)?.routing.next != NodeContinuation::Step(StepIdx::new(2)) {
        return Err("Ask must fall through to AskResume".to_owned());
    }
    match &node(&workflow, 2)?.kind {
        CompiledNodeKind::AskResume { answer: resumed } if *resumed == answer => Ok(()),
        other => Err(format!("expected AskResume carrying answer, got {other:?}")),
    }
}

#[test]
fn ask_timeout_binds_lexical_deadline() -> Result<(), String> {
    let workflow = compile(&steps(&format!(
        "{SEED}{DELAY}  - id: q\n    ask: {{prompt: '$saved', timeout: '$delay'}}\n{DONE}"
    )))?;
    let (prompt, timeout) = match &node(&workflow, 2)?.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => (*prompt, timeout_slot.slot()),
        other => return Err(format!("expected Ask, got {other:?}")),
    };
    let (seed, delay) = (out(&workflow, 0)?, out(&workflow, 1)?);
    if prompt == seed && timeout == Some(delay) {
        Ok(())
    } else {
        Err(format!(
            "wrong ask slots prompt={prompt:?} timeout={timeout:?}"
        ))
    }
}

#[test]
fn wait_until_binds_deadline_slot() -> Result<(), String> {
    let workflow = compile(&steps(&format!(
        "{DELAY}  - id: w\n    wait: {{timeout: '$delay'}}\n{DONE}"
    )))?;
    match &node(&workflow, 1)?.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } if *deadline_slot == out(&workflow, 0)? => {
            if node(&workflow, 1)?.routing.next == NodeContinuation::Step(StepIdx::new(2)) {
                Ok(())
            } else {
                Err("WaitUntil must fall through to finish".to_owned())
            }
        }
        other => Err(format!(
            "expected WaitUntil bound to seed slot, got {other:?}"
        )),
    }
}

#[test]
fn wait_event_binds_event_and_timeout_slots() -> Result<(), String> {
    let body =
        format!("{SEED}{DELAY}  - id: w\n    wait: {{event: '$saved', timeout: '$delay'}}\n{DONE}");
    let workflow = compile(&steps(&body))?;
    match &node(&workflow, 2)?.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } if *event == out(&workflow, 0)? && timeout_slot.slot() == Some(out(&workflow, 1)?) => {
            Ok(())
        }
        other => Err(format!(
            "expected WaitEvent bound to lexical slots, got {other:?}"
        )),
    }
}

#[test]
fn wait_event_without_timeout_leaves_deadline_unset() -> Result<(), String> {
    let workflow = compile(&steps(&format!(
        "{SEED}  - id: w\n    wait: {{event: '$saved'}}\n{DONE}"
    )))?;
    match &node(&workflow, 1)?.kind {
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } if *event == out(&workflow, 0)? && timeout_slot.slot().is_none() => Ok(()),
        other => Err(format!("expected unbounded WaitEvent, got {other:?}")),
    }
}

#[test]
fn numeric_slots_keep_working_for_suspension_fields() -> Result<(), String> {
    let workflow = compile(&steps(&format!(
        "  - id: q\n    ask: {{prompt: '0', timeout: '1'}}\n{DONE}"
    )))?;
    match &node(&workflow, 0)?.kind {
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } if *prompt == SlotIdx::new(0) && timeout_slot.slot() == Some(SlotIdx::new(1)) => Ok(()),
        other => Err(format!("expected numeric Ask slots, got {other:?}")),
    }
}

#[test]
fn two_asks_allocate_distinct_answers() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: a1\n    ask: {{prompt: '$saved'}}\n  - id: a2\n    ask: {{prompt: '$saved'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    let (first, second) = (out(&workflow, 1)?, out(&workflow, 3)?);
    if first != second {
        Ok(())
    } else {
        Err(format!("duplicate answer slot {first:?}"))
    }
}

#[test]
fn ask_answer_feeds_wait_deadline() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: q\n    ask: {{prompt: '$saved'}}\n  - id: w\n    wait: {{timeout: '$q'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    let answer = out(&workflow, 1)?;
    match &node(&workflow, 3)?.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } if *deadline_slot == answer => Ok(()),
        other => Err(format!("expected WaitUntil on ask answer, got {other:?}")),
    }
}

#[test]
fn ask_on_error_routes_both_nodes_to_same_handler() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: q\n    on_error: {{handler: done}}\n    ask: {{prompt: '$saved'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    assert_handler(node(&workflow, 1)?, 3)?;
    assert_handler(node(&workflow, 2)?, 3)?;
    assert_no_nop(&workflow)
}

#[test]
fn wait_on_error_routes_to_handler() -> Result<(), String> {
    let body = format!(
        "{DELAY}  - id: w\n    on_error: {{handler: done}}\n    wait: {{timeout: '$delay'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    assert_handler(node(&workflow, 1)?, 2)?;
    assert_no_nop(&workflow)
}

#[test]
fn on_error_inside_body_binds_body_local_handler() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: q\n          on_error: {{handler: bail}}\n          ask: {{prompt: '$saved'}}\n        - id: bail\n          set: {{output: b, value: '0'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    assert_handler(node(&workflow, 2)?, 4)?;
    assert_handler(node(&workflow, 3)?, 4)?;
    assert_no_nop(&workflow)
}

#[test]
fn on_error_outside_body_is_rejected() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: q\n          on_error: {{handler: done}}\n          ask: {{prompt: '$saved'}}\n{DONE}"
    );
    rejects(
        &steps(&body),
        CompileError::UnknownStepLabel {
            step: 2,
            label: "done".into(),
        },
    )
}

#[test]
fn on_error_unknown_handler_is_rejected() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: q\n    on_error: {{handler: nope}}\n    ask: {{prompt: '$saved'}}\n{DONE}"
    );
    rejects(
        &steps(&body),
        CompileError::UnknownStepLabel {
            step: 1,
            label: "nope".into(),
        },
    )
}

#[test]
fn try_again_rejected_on_ask_and_wait() -> Result<(), String> {
    for source in [
        steps(&format!(
            "{SEED}  - id: q\n    try_again: {{max_attempts: 2}}\n    ask: {{prompt: '$saved'}}\n{DONE}"
        )),
        steps(&format!(
            "{DELAY}  - id: w\n    try_again: {{max_attempts: 2}}\n    wait: {{timeout: '$delay'}}\n{DONE}"
        )),
    ] {
        rejects(
            &source,
            CompileError::UnsupportedStepControlField {
                step: 1,
                field: "try_again".into(),
            },
        )?;
    }
    Ok(())
}

#[test]
fn try_again_rejected_on_wait_in_body() -> Result<(), String> {
    let body = format!(
        "{DELAY}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: w\n          try_again: {{max_attempts: 2}}\n          wait: {{timeout: '$delay'}}\n{DONE}"
    );
    rejects(
        &steps(&body),
        CompileError::UnsupportedStepControlField {
            step: 0,
            field: "try_again".into(),
        },
    )
}

#[test]
fn try_again_on_do_attaches_validated_policy() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: work\n    try_again: {{max_attempts: 3, delay: '100ms'}}\n    do: {{action: '0', input: '$saved'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    let expected = Some(CompiledRetryPolicy {
        max_attempts: 3,
        delay_ms: 100,
    });
    if node(&workflow, 1)?.retry == expected {
        Ok(())
    } else {
        Err(format!(
            "wrong retry policy: {:?}",
            node(&workflow, 1)?.retry
        ))
    }
}

#[test]
fn try_again_bounds_are_rejected() -> Result<(), String> {
    for attempts in [0, 4] {
        let body = format!(
            "{SEED}  - id: work\n    try_again: {{max_attempts: {attempts}}}\n    do: {{action: '0', input: '$saved'}}\n{DONE}"
        );
        rejects(
            &steps(&body),
            CompileError::StepFieldShape {
                step: 1,
                field: "try_again",
                expected: "an action with 1..=3 total attempts and a bounded constant delay (ms, s, m, h, d)",
            },
        )?;
    }
    Ok(())
}

#[test]
fn wait_without_event_or_timeout_rejected() -> Result<(), String> {
    let body = format!("  - id: w\n    wait: {{}}\n{DONE}");
    rejects(
        &steps(&body),
        CompileError::StepFieldShape {
            step: 0,
            field: "wait",
            expected: "until without timeout, event with optional timeout, or timeout only",
        },
    )
}

#[test]
fn ask_unknown_reference_rejected() -> Result<(), String> {
    let body = format!("  - id: q\n    ask: {{prompt: '$missing'}}\n{DONE}");
    rejects(
        &steps(&body),
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$missing".into(),
            name: "missing".into(),
        },
    )
}

#[test]
fn ask_forward_and_self_references_rejected() -> Result<(), String> {
    let forward = steps(&format!(
        "  - id: q\n    ask: {{prompt: '$saved'}}\n{SEED}{DONE}"
    ));
    rejects(
        &forward,
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$saved".into(),
            name: "saved".into(),
        },
    )?;
    let self_ref = steps(&format!("  - id: q\n    ask: {{prompt: '$q'}}\n{DONE}"));
    rejects(
        &self_ref,
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$q".into(),
            name: "q".into(),
        },
    )
}

#[test]
fn duplicate_ask_step_ids_rejected() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: q\n    ask: {{prompt: '$saved'}}\n  - id: q\n    ask: {{prompt: '$saved'}}\n{DONE}"
    );
    rejects(
        &steps(&body),
        CompileError::DuplicateStepId { id: "q".into() },
    )
}

#[test]
fn answer_does_not_escape_scoped_body() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: q\n          ask: {{prompt: '$saved'}}\n  - id: use\n    set: {{output: got, value: '$q'}}\n{DONE}"
    );
    rejects(
        &steps(&body),
        CompileError::UnknownReferenceName {
            kind: "output",
            reference: "$q".into(),
            name: "q".into(),
        },
    )
}

#[test]
fn ask_in_repeat_body_suspends_resumes_and_retries() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: q\n          ask: {{prompt: '$saved'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    let (prompt, answer) = match &node(&workflow, 2)?.kind {
        CompiledNodeKind::Ask { prompt, .. } => (*prompt, out(&workflow, 2)?),
        other => return Err(format!("expected Ask in body, got {other:?}")),
    };
    if prompt != out(&workflow, 0)? {
        return Err("body ask prompt must bind the outer seed slot".to_owned());
    }
    match &node(&workflow, 3)?.kind {
        CompiledNodeKind::AskResume { answer: resumed } if *resumed == answer => {}
        other => return Err(format!("expected AskResume in body, got {other:?}")),
    }
    match &node(&workflow, 4)?.kind {
        CompiledNodeKind::Copy { source } if *source == answer => {}
        other => return Err(format!("expected captured answer copy, got {other:?}")),
    }
    match &node(&workflow, 5)?.kind {
        CompiledNodeKind::RepeatCheck { .. } => {}
        other => return Err(format!("expected RepeatCheck retry edge, got {other:?}")),
    }
    if node(&workflow, 5)?.routing.next == NodeContinuation::Step(StepIdx::new(2)) {
        assert_no_nop(&workflow)
    } else {
        Err("RepeatCheck must re-enter body at Ask".to_owned())
    }
}

#[test]
fn wait_in_choose_body_uses_outer_deadline() -> Result<(), String> {
    let body = format!(
        "{DELAY}  - id: c\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: w\n              wait: {{timeout: '$delay'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    match &node(&workflow, 1)?.kind {
        CompiledNodeKind::Choose { branches, .. }
            if branches
                .first()
                .is_some_and(|b| b.target == StepIdx::new(2)) => {}
        other => {
            return Err(format!(
                "expected Choose targeting wait body, got {other:?}"
            ));
        }
    }
    match &node(&workflow, 2)?.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } if *deadline_slot == out(&workflow, 0)? => {
            assert_no_nop(&workflow)
        }
        other => Err(format!(
            "expected WaitUntil on outer deadline, got {other:?}"
        )),
    }
}

#[test]
fn ask_in_for_each_body_uses_loop_variable() -> Result<(), String> {
    let source = format!(
        "inputs:\n  items: {{is: list, of: any, max: 4}}\n{HEADER}steps:\n  - id: f\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: q\n          ask: {{prompt: '$entry'}}\n{DONE}"
    );
    let workflow = compile(&source)?;
    let CompiledNodeKind::ForEachStart { item_slot, .. } = &node(&workflow, 2)?.kind else {
        return Err("expected ForEachStart".to_owned());
    };
    let item_slot = *item_slot;
    match &node(&workflow, 3)?.kind {
        CompiledNodeKind::Ask { prompt, .. } if *prompt == item_slot => assert_no_nop(&workflow),
        other => Err(format!("expected Ask on loop variable, got {other:?}")),
    }
}

#[test]
fn wait_in_for_each_body_uses_loop_variable() -> Result<(), String> {
    let source = format!(
        "inputs:\n  items: {{is: list, of: any, max: 4}}\n{HEADER}steps:\n  - id: f\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: w\n          wait: {{timeout: '$entry'}}\n{DONE}"
    );
    let workflow = compile(&source)?;
    let CompiledNodeKind::ForEachStart { item_slot, .. } = &node(&workflow, 2)?.kind else {
        return Err("expected ForEachStart".to_owned());
    };
    let item_slot = *item_slot;
    match &node(&workflow, 3)?.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } if *deadline_slot == item_slot => {
            assert_no_nop(&workflow)
        }
        other => Err(format!(
            "expected WaitUntil on loop variable, got {other:?}"
        )),
    }
}

#[test]
fn ask_in_together_branch_pairs_with_resume() -> Result<(), String> {
    let body = format!(
        "{SEED}  - id: t\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: q\n              ask: {{prompt: '$saved'}}\n        - label: b\n          steps:\n            - id: s\n              set: {{output: o, value: '1'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    let parts = workflow.to_parts();
    let ask = parts
        .nodes
        .iter()
        .find(|n| matches!(n.kind, CompiledNodeKind::Ask { .. }))
        .ok_or("missing branch Ask")?;
    let (ask_id, answer) = (ask.id, ask.output.slot().ok_or("missing ask answer")?);
    let resumed = ask_id.get().checked_add(1).ok_or("ask step overflow")?;
    match &node(&workflow, resumed)?.kind {
        CompiledNodeKind::AskResume { answer: got } if *got == answer => assert_no_nop(&workflow),
        other => Err(format!(
            "expected AskResume with branch answer, got {other:?}"
        )),
    }
}

#[test]
fn wait_in_collect_body_waits_on_outer_deadline() -> Result<(), String> {
    let source = format!(
        "inputs:\n  pages: {{is: list, of: any, max: 4}}\n{HEADER}steps:\n{DELAY}  - id: c\n    collect:\n      variable: batch\n      source: '$input.pages'\n      pages: 2\n      items: 1\n      steps:\n        - id: w\n          wait: {{timeout: '$delay'}}\n{DONE}"
    );
    let workflow = compile(&source)?;
    let parts = workflow.to_parts();
    let wait = parts
        .nodes
        .iter()
        .find(|n| matches!(n.kind, CompiledNodeKind::WaitUntil { .. }))
        .ok_or("missing collect WaitUntil")?;
    match &wait.kind {
        CompiledNodeKind::WaitUntil { deadline_slot } if *deadline_slot == out(&workflow, 0)? => {
            assert_no_nop(&workflow)
        }
        other => Err(format!(
            "expected WaitUntil on outer deadline, got {other:?}"
        )),
    }
}

#[test]
fn conditional_slot_rejected_in_suspension_inputs() -> Result<(), String> {
    let guard =
        format!("{SEED}  - id: g\n    if: 'true'\n    do: {{action: '0', input: '$saved'}}\n");
    let cases = [
        (
            "ask.prompt",
            "  - id: q\n    ask: {prompt: '$g'}\n",
            "ask.prompt",
        ),
        (
            "ask.timeout",
            "  - id: q\n    ask: {prompt: '$saved', timeout: '$g'}\n",
            "ask.timeout",
        ),
        (
            "waituntil.deadline",
            "  - id: w\n    wait: {timeout: '$g'}\n",
            "waituntil.deadline",
        ),
        (
            "waitevent.event",
            "  - id: w\n    wait: {event: '$g'}\n",
            "waitevent.event",
        ),
        (
            "waitevent.timeout",
            "  - id: w\n    wait: {event: '$saved', timeout: '$g'}\n",
            "waitevent.timeout",
        ),
    ];
    for (_, leaf, consumer) in cases {
        let body = format!("{guard}{leaf}{DONE}");
        rejects(
            &steps(&body),
            CompileError::PossiblySkippedReference {
                slot: SlotIdx::new(1),
                consumer,
            },
        )?;
    }
    Ok(())
}

#[test]
fn scoped_flows_emit_no_nop_nodes() -> Result<(), String> {
    let body = format!(
        "{SEED}{DELAY}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: q\n          ask: {{prompt: '$saved'}}\n  - id: c\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: w\n              wait: {{timeout: '$delay'}}\n{DONE}"
    );
    let workflow = compile(&steps(&body))?;
    let parts = workflow.to_parts();
    let kinds: Vec<_> = parts.nodes.iter().map(|n| &n.kind).collect();
    for required in ["Ask", "AskResume", "WaitUntil"] {
        if !kinds.iter().any(|k| format!("{k:?}").starts_with(required)) {
            return Err(format!("missing {required} node"));
        }
    }
    assert_no_nop(&workflow)
}
