use crate::guard_boundary::{complete, f1_contracts_through};
use crate::guard_journal::verify;
use crate::guard_support::{compile_f1_workflow, test_runtime_with_journal};
use vb_core::{ActionId, CapabilitySet, RunId, SlotValue};

const GUARDED_ROLLBACK_YAML: &str = "version: velvet-ballistics/v1\nname: guarded_rollback\nwhen: { manual: {} }\nsteps:\n  - id: seed\n    set:\n      output: target\n      value: \"42\"\n  - id: deploy\n    do:\n      action: \"1\"\n      input: \"$seed\"\n  - id: smoke\n    do:\n      action: \"2\"\n      input: \"$deploy\"\n  - id: rollback\n    if: \"$steps.smoke == false\"\n    do:\n      action: \"3\"\n      input: \"$deploy\"\n  - id: done\n    finish:\n      result: \"$steps.deploy\"\n";

pub(super) fn run_scenario(smoke_passed: bool) -> Result<(), String> {
    let workflow = compile_f1_workflow(GUARDED_ROLLBACK_YAML)?;
    let (mut runtime, journal) = test_runtime_with_journal()?;

    let run = if smoke_passed {
        RunId::new(2001)
    } else {
        RunId::new(2002)
    };

    let contracts = f1_contracts_through(ActionId::new(3))
        .map_err(|e| format!("failed to build contracts: {e:?}"))?;

    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::new([]),
            CapabilitySet::empty(),
            contracts,
        )
        .map_err(|e| format!("failed to submit workflow: {e:?}"))?;

    runtime
        .tick_all()
        .map_err(|e| format!("failed to tick after submit: {e:?}"))?;

    let mut tickets = Vec::new();

    let t1 = complete(&mut runtime, run, 1, 1, 1, SlotValue::I64(42))?;
    tickets.push(t1);

    let t2 = complete(&mut runtime, run, 2, 2, 2, SlotValue::Bool(smoke_passed))?;
    tickets.push(t2);

    if !smoke_passed {
        let t3 = complete(&mut runtime, run, 4, 3, 3, SlotValue::I64(7))?;
        tickets.push(t3);
    }

    if runtime.lookup_pending_action_ticket(run, 4).is_some() {
        return Err(format!("unexpected pending ticket for run {run:?} step 4"));
    }

    let counters = runtime.counters_snapshot();
    if counters.runs_completed != 1 {
        return Err(format!(
            "expected 1 completed run, got {:?}",
            counters.runs_completed
        ));
    }
    if counters.runs_failed != 0 {
        return Err(format!(
            "expected 0 failed runs, got {:?}",
            counters.runs_failed
        ));
    }

    let events = journal
        .snapshot()
        .map_err(|e| format!("failed to snapshot journal: {e:?}"))?;

    verify(&events, run, &tickets, smoke_passed)?;

    Ok(())
}
