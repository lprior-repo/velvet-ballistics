use std::error::Error;

use vb_core::SlotValue;
use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, SlotIdx};

use super::{Shard, ShardCommand, small_config, suspended_workflow};
use crate::journal::{RuntimeJournalEvent, SharedRuntimeJournal, VolatileRuntimeJournal};

#[test]
fn submit_empty_caps_no_fabrication() -> Result<(), Box<dyn Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared); shard.configure_live_policy(crate::policy::test_permit_all_policy()).unwrap();

    let run = RunId::new(1);
    let workflow = suspended_workflow().ok_or("missing fixture")?;

    shard.enqueue(ShardCommand::Submit {
        run,
        workflow,
        caps: CapabilitySet::empty(),
    })?;

    assert_eq!(shard.tick(), Ok(true));

    let events = journal.snapshot()?;
    let admissions: Vec<CapabilitySet> = events
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::RunAdmission { admission } if admission.run_id() == run => {
                Some(admission.granted_capabilities().clone())
            }
            _ => None,
        })
        .collect();

    assert_eq!(admissions, vec![CapabilitySet::empty()]);

    Ok(())
}

#[test]
fn submit_with_inputs_empty_caps_no_fabrication() -> Result<(), Box<dyn Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared); shard.configure_live_policy(crate::policy::test_permit_all_policy()).unwrap();

    let run = RunId::new(2);
    let workflow = suspended_workflow().ok_or("missing fixture")?;

    shard.enqueue(ShardCommand::SubmitWithInputs {
        run,
        workflow,
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(7))]),
        caps: CapabilitySet::empty(),
    })?;

    assert_eq!(shard.tick(), Ok(true));

    let events = journal.snapshot()?;
    let admissions: Vec<CapabilitySet> = events
        .iter()
        .filter_map(|event| match event {
            RuntimeJournalEvent::RunAdmission { admission } if admission.run_id() == run => {
                Some(admission.granted_capabilities().clone())
            }
            _ => None,
        })
        .collect();

    assert_eq!(admissions, vec![CapabilitySet::empty()]);

    Ok(())
}

#[test]
fn submit_with_contracts_empty_caps_denied() -> Result<(), Box<dyn Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();
    let mut shard = Shard::new_with_journal(small_config(), shared); shard.configure_live_policy(crate::policy::test_permit_all_policy()).unwrap();

    let run = RunId::new(3);
    let workflow = suspended_workflow().ok_or("missing fixture")?;

    shard.enqueue(ShardCommand::SubmitWithInputsAndContracts {
        run,
        workflow,
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(7))]),
        caps: CapabilitySet::empty(),
        action_contracts: super::test_fixtures::contracts_through(ActionId::new(0)),
    })?;

    assert_eq!(
        shard.tick(),
        Err(crate::RuntimeError::AdmissionCapabilityDenied {
            action: ActionId::new(0),
            required: vb_core::capability::Capability::new(
                "__contract_required__".into(),
                ActionId::new(0),
            ),
            granted: CapabilitySet::empty(),
        }),
    );

    let events = journal.snapshot()?;
    assert!(events.is_empty());
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.counters().snapshot().runs_submitted, 0);

    Ok(())
}

fn nop_then_do() -> Result<vb_core::workflow::CompiledWorkflow, Box<dyn Error>> {
    let nop_node = super::CompiledNode {
        id: vb_core::ids::StepIdx::ZERO,
        output: None,
        next: Some(vb_core::ids::StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: super::CompiledNodeKind::Nop,
    };
    let do_node = super::CompiledNode {
        id: vb_core::ids::StepIdx::new(1),
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: super::CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
    };
    let parts = super::WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("nop_then_do"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([nop_node, do_node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::ids::StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: super::ResourceContract::DEFAULT,
        input_slots: Box::from([]),
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).map_err(Into::into)
}

#[test]
fn bare_submit_no_input_fabrication() -> Result<(), Box<dyn Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();

    let mut config = small_config();
    config.step_budget_per_tick = 1;

    let mut shard = Shard::new_with_journal(config, shared); shard.configure_live_policy(crate::policy::test_permit_all_policy()).unwrap();

    let run = RunId::new(10);
    let workflow = nop_then_do()?;

    let caps = super::test_fixtures::contract_capability_for(ActionId::new(0));

    shard.enqueue(ShardCommand::Submit {
        run,
        workflow,
        caps: caps.clone(),
    })?;

    assert_eq!(shard.tick(), Ok(true));

    let state = shard.run_state_get(run).ok_or("run state missing")?;

    assert_eq!(
        state.frame.read_slot(SlotIdx::ZERO),
        Err(vb_core::errors::CoreError::SlotUninitialized {
            slot: SlotIdx::ZERO
        })
    );

    assert!(state.action_contracts.is_empty());

    assert_eq!(
        state
            .admission
            .as_ref()
            .ok_or("missing admission")?
            .granted_capabilities(),
        &caps
    );

    Ok(())
}

#[test]
fn submit_with_inputs_no_fabrication() -> Result<(), Box<dyn Error>> {
    let journal = std::sync::Arc::new(VolatileRuntimeJournal::new());
    let shared: SharedRuntimeJournal = journal.clone();

    let mut config = small_config();
    config.step_budget_per_tick = 1;

    let mut shard = Shard::new_with_journal(config, shared); shard.configure_live_policy(crate::policy::test_permit_all_policy()).unwrap();

    let run = RunId::new(11);
    let workflow = nop_then_do()?;

    let caps = super::test_fixtures::contract_capability_for(ActionId::new(0));

    shard.enqueue(ShardCommand::SubmitWithInputs {
        run,
        workflow,
        inputs: Box::from([(SlotIdx::ZERO, SlotValue::I64(7))]),
        caps: caps.clone(),
    })?;

    assert_eq!(shard.tick(), Ok(true));

    let state = shard.run_state_get(run).ok_or("run state missing")?;

    assert_eq!(state.frame.read_slot(SlotIdx::ZERO), Ok(&SlotValue::I64(7)));

    assert!(state.action_contracts.is_empty());

    assert_eq!(
        state
            .admission
            .as_ref()
            .ok_or("missing admission")?
            .granted_capabilities(),
        &caps
    );

    Ok(())
}
