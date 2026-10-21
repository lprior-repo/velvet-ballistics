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
    let mut shard = Shard::new_with_journal(small_config(), shared);

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
    let mut shard = Shard::new_with_journal(small_config(), shared);

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
    let mut shard = Shard::new_with_journal(small_config(), shared);

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
