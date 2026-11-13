//! Cold replay of completed terminals and ask-answer taint propagation.

#![forbid(unsafe_code)]

use vb_core::value::Taint;
use vb_core::{CompiledNodeKind, RunId, RuntimePolicy, SlotValue};

use crate::source_matrix::{HEADER, seed_and_finish};
use crate::support::{
    self, Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[test]
fn direct_api_terminal_replays_after_cold_reopen_matrix() -> TestResult {
    let yaml = seed_and_finish(
        "  - id: s\n    set: {output: value, value: '42'}\n",
        "$step.s",
    );
    let workflow = compile(&yaml)?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(8);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        tick_until_terminal(&mut open.runtime, run, 256)?;
        assert_eq!(open.runtime.counters_snapshot().runs_completed, 1);
    }

    let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let durable = events(&open.journal, run)?;
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(42));
    assert_eq!(result.taint, Taint::Clean);
    let product = open.runtime.recover_product(run)?;
    match product {
        vb_runtime::recovery::RuntimeRecoveryProduct::CannotResume(product) => {
            assert_eq!(product.reason(), "terminal_state");
        }
        other => return Err(format!("completed run recovered as {other:?}").into()),
    }
    assert_eq!(
        open.runtime.recover_and_resume(run),
        Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("terminal_state"),
        })
    );
    Ok(())
}

#[test]
fn derived_from_secret_answer_taint_survives_journal_and_replay() -> TestResult {
    let yaml = format!(
        "{HEADER}vars:\n  prompt_text: 'input?'\nsteps:\n  - id: ask_q\n    ask: {{prompt: '$vars.prompt_text'}}\n  - id: done\n    finish: {{result: '$step.ask_q'}}\n"
    );
    let workflow = compile(&yaml)?;
    let ask_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .map(|node| node.id)
        .ok_or("missing Ask")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(9);
    {
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
        open.runtime.submit_compiled(run, workflow.clone())?;
        let ticket = support::tick_until_ask(&mut open.runtime, run, ask_step, 64)?;
        open.runtime.answer_ask(vb_runtime::shard::AskAnswer::new(
            ticket,
            ticket.answer_slot(),
            SlotValue::I64(5),
            Taint::DerivedFromSecret,
        ))?;
        tick_until_terminal(&mut open.runtime, run, 64)?;
        let durable = events(&open.journal, run)?;
        let result = terminal(&durable, run)?;
        assert_eq!(result.value, SlotValue::I64(5));
        assert_eq!(result.taint, Taint::DerivedFromSecret);
        assert_ordering(&durable, run)?;
    }
    let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    let durable = events(&open.journal, run)?;
    let result = terminal(&durable, run)?;
    assert_eq!(result.value, SlotValue::I64(5));
    assert_eq!(result.taint, Taint::DerivedFromSecret);
    Ok(())
}

#[test]
fn secret_answer_without_contract_opt_in_is_typed_rejection() -> TestResult {
    let yaml = format!(
        "{HEADER}vars:\n  prompt_text: 'input?'\nsteps:\n  - id: ask_q\n    ask: {{prompt: '$vars.prompt_text'}}\n  - id: done\n    finish: {{result: '$step.ask_q'}}\n"
    );
    let workflow = compile(&yaml)?;
    let ask_step = workflow
        .to_parts()
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .map(|node| node.id)
        .ok_or("missing Ask")?;
    let dir = tempfile::tempdir()?;
    let run = RunId::new(10);
    let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
    persist_artifact(&open.journal, &workflow, RuntimePolicy::Strict, &[])?;
    open.runtime.submit_compiled(run, workflow)?;
    let ticket = support::tick_until_ask(&mut open.runtime, run, ask_step, 64)?;
    open.runtime.answer_ask(vb_runtime::shard::AskAnswer::new(
        ticket,
        ticket.answer_slot(),
        SlotValue::I64(5),
        Taint::Secret,
    ))?;
    let error = open
        .runtime
        .tick_all()
        .expect_err("Secret answer requires contract opt-in");
    assert_eq!(error, vb_runtime::RuntimeError::SecretResultNotAllowed);
    Ok(())
}
