use std::num::NonZeroUsize;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;
use vb_core::ids::RunId;
use vb_core::value::Taint;
use vb_core::workflow::CompiledWorkflow;
use vb_ipc::server::IpcServer;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskTicket, ShardConfig};
use vb_runtime::trace::TraceEvent;
use vb_storage::{DecodedSlotWrittenExtra, FjallJournal, JournalEvent, decode_slot_written_extra};

struct AnswerCliFixture {
    server: IpcServer,
    runtime: Runtime,
    journal: std::sync::Arc<FjallJournal>,
    run: RunId,
    asks: Vec<StepIdx>,
    db: std::path::PathBuf,
    _directory: tempfile::TempDir,
}

fn answer_cli_fixture() -> Result<AnswerCliFixture, Box<dyn std::error::Error>> {
    let workflow = compiled_answer_workflow()?;
    let asks = compiled_ask_steps(&workflow);
    let directory = cli_tempdir()?;
    let db = directory.path().canonicalize()?.join("answer.db");
    let server = IpcServer::bind(&db.with_extension("sock"))?;
    let journal = std::sync::Arc::new(FjallJournal::open(&db, None)?);
    vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)?;
    let runtime = Runtime::new(NonZeroUsize::MIN, ShardConfig::default(), vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal.clone())).expect("valid test configuration");
    let run = RunId::new(48_001);
    runtime.submit_compiled(run, workflow)?;
    Ok(AnswerCliFixture {
        server,
        runtime,
        journal,
        run,
        asks,
        db,
        _directory: directory,
    })
}

fn compiled_answer_workflow() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let source = "version: velvet-ballistics/v1\nname: answer_selector_cli\nwhen: {manual: {}}\nsteps:\n  - id: first\n    set: {output: first_value, value: '7'}\n  - id: first_question\n    ask: {prompt: '$first'}\n  - id: second_question\n    ask: {prompt: '$first'}\n  - id: third_question\n    ask: {prompt: '$first'}\n  - id: done\n    finish: {result: '$third_question'}\n";
    vb_compile::compile_workflow(source.as_bytes())
        .map_err(|error| std::io::Error::other(format!("workflow compile failed: {error:?}")))
        .map_err(Into::into)
}

fn compiled_ask_steps(workflow: &CompiledWorkflow) -> Vec<StepIdx> {
    (0..workflow.node_count())
        .filter_map(|step| {
            let index = StepIdx::new(step);
            workflow
                .node(index)
                .filter(|node| matches!(&node.kind, CompiledNodeKind::Ask { .. }))
                .map(|_| index)
        })
        .collect()
}

fn wait_for_ask(
    runtime: &mut Runtime,
    run: RunId,
    step: StepIdx,
) -> Result<AskTicket, Box<dyn std::error::Error>> {
    for _ in 0..64 {
        runtime.tick_all()?;
        if let Some(ticket) = runtime.lookup_pending_ask_ticket(run, step) {
            return Ok(ticket);
        }
    }
    Err(std::io::Error::other("run did not reach the expected Ask").into())
}
