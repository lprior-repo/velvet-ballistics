use super::*;
use std::{num::NonZeroUsize, process::Command};
type TestResult = Result<(), Box<dyn std::error::Error>>;
const FIRST: &str = "VB_F1_REGISTRY_FIRST_20261110";
const SECOND: &str = "VB_F1_REGISTRY_SECOND_20261110";

fn workflow(name: &str) -> Result<CompiledWorkflow, vb_compile::CompileErrors> {
    vb_compile::compile_workflow(format!("version: velvet-ballistics/v1\nname: registry_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {name:?}}}\nsteps:\n  - id: done\n    finish: {{result: '$secrets.token'}}\n").as_bytes())
}
fn runtime() -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig {
            policy: vb_core::RuntimePolicy::Relaxed,
            ..Default::default()
        },
        vb_runtime::journal::NoopRuntimeJournal::shared(),
    )
}

#[test]
fn available_names_survive_other_workflows_and_are_deduplicated() -> TestResult {
    if !super::super::present(FIRST) || !super::super::present(SECOND) {
        let output = Command::new(std::env::current_exe()?)
            .args(["--exact", "run_compiled_runtime::secret_presence::registry::tests::available_names_survive_other_workflows_and_are_deduplicated", "--nocapture"])
            .env(FIRST, "fixture-only-first-value").env(SECOND, "fixture-only-second-value").output()?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        return Ok(());
    }
    let mut names = AvailableNames::default();
    let mut runtime = runtime();
    let first = workflow(FIRST)?;
    let second = workflow(SECOND)?;
    names.install(&mut runtime, &first);
    names.install(&mut runtime, &second);
    names.install(&mut runtime, &second);
    assert_eq!(names.names.len(), 2);
    // B's preparation must not revoke A before A reaches admission/queued execution.
    runtime.submit_compiled(vb_core::RunId::new(1), first)?;
    runtime.submit_compiled(vb_core::RunId::new(2), second)?;
    runtime.tick_all()?;
    runtime.tick_all()?;
    assert_eq!(runtime.counters_snapshot().runs_completed, 2);
    assert_eq!(runtime.counters_snapshot().runs_failed, 0);
    Ok(())
}

#[test]
fn missing_declared_names_do_not_accumulate() -> TestResult {
    let mut names = AvailableNames::default();
    let mut runtime = runtime();
    for id in 0..32 {
        let missing = format!("VB_F1_MISSING_REGISTRY_20261110_{id}");
        assert!(!super::super::present(&missing));
        let workflow = workflow(&missing)?;
        names.install(&mut runtime, &workflow);
        assert_eq!(
            runtime.submit_compiled(vb_core::RunId::new(id), workflow),
            Err(vb_runtime::RuntimeError::SecretUnavailable)
        );
    }
    assert_eq!(names.names.len(), 0);
    assert_eq!(runtime.counters_snapshot().runs_submitted, 0);
    Ok(())
}
