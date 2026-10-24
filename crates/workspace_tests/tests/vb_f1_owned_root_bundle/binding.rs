use vb_core::ids::{SlotIdx, WorkflowDigest};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, CompiledWorkflow, InputSlotKind, ResourceContract};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn resource_contract_field_independence() -> TestResult<()> {
    // Given: base workflow and matching bundle
    let base_wf = fixture(
        2,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        1024,
    )?;
    let cp = checkpoint(8);
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &base_wf)?;

    // When: independently change each ResourceContract field
    let base_contract = base_wf.resource_contract();
    let variants = [
        (
            "max_steps",
            ResourceContract {
                max_steps: 9999,
                ..base_contract
            },
        ),
        (
            "max_slots",
            ResourceContract {
                max_slots: 7,
                ..base_contract
            },
        ),
        (
            "max_constants",
            ResourceContract {
                max_constants: 65534,
                ..base_contract
            },
        ),
        (
            "max_accessors",
            ResourceContract {
                max_accessors: 8191,
                ..base_contract
            },
        ),
        (
            "max_expressions",
            ResourceContract {
                max_expressions: 4095,
                ..base_contract
            },
        ),
        (
            "max_expr_stack",
            ResourceContract {
                max_expr_stack: 63,
                ..base_contract
            },
        ),
        (
            "max_step_budget_per_tick",
            ResourceContract {
                max_step_budget_per_tick: 9999,
                ..base_contract
            },
        ),
        (
            "max_transitions_per_tick",
            ResourceContract {
                max_transitions_per_tick: 9999,
                ..base_contract
            },
        ),
        (
            "max_input_bytes",
            ResourceContract {
                max_input_bytes: 1048575,
                ..base_contract
            },
        ),
        (
            "max_output_bytes",
            ResourceContract {
                max_output_bytes: 262143,
                ..base_contract
            },
        ),
        (
            "max_blob_bytes",
            ResourceContract {
                max_blob_bytes: 1023,
                ..base_contract
            },
        ),
        (
            "max_ipc_payload_bytes",
            ResourceContract {
                max_ipc_payload_bytes: 1048575,
                ..base_contract
            },
        ),
        (
            "max_retry_attempts",
            ResourceContract {
                max_retry_attempts: 2,
                ..base_contract
            },
        ),
        (
            "max_fanout",
            ResourceContract {
                max_fanout: 63,
                ..base_contract
            },
        ),
        (
            "max_collect_items",
            ResourceContract {
                max_collect_items: 1023,
                ..base_contract
            },
        ),
        (
            "max_queue_depth",
            ResourceContract {
                max_queue_depth: 1023,
                ..base_contract
            },
        ),
        (
            "max_journal_batch_bytes",
            ResourceContract {
                max_journal_batch_bytes: 1048575,
                ..base_contract
            },
        ),
        (
            "allows_secret_results",
            ResourceContract {
                allows_secret_results: true,
                ..base_contract
            },
        ),
    ];

    for (label, contract) in variants {
        let mut parts = base_wf.to_parts();
        parts.resource_contract = contract;
        let changed = CompiledWorkflow::try_from_parts(parts)?;
        assert_eq!(
            base_wf.digest(),
            changed.digest(),
            "digest should be same for {label}"
        );
        assert!(!bundle.matches_workflow(&changed), "{label}");
    }

    Ok(())
}

#[test]
fn exact_projection_probes() -> TestResult<()> {
    // Given: base workflow and matching bundle
    let base_wf = fixture(
        2,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        1024,
    )?;
    let cp = checkpoint(8);
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &base_wf)?;

    // When: change digest
    let mut parts = base_wf.to_parts();
    parts.digest = WorkflowDigest::from_bytes([43; 32]);
    let digest_changed = CompiledWorkflow::try_from_parts(parts)?;
    assert_ne!(base_wf.digest(), digest_changed.digest());
    assert!(!bundle.matches_workflow(&digest_changed));

    // When: change frame slot_count
    let mut parts = base_wf.to_parts();
    parts.slot_count = 3;
    let slot_changed = CompiledWorkflow::try_from_parts(parts)?;
    assert_eq!(base_wf.digest(), slot_changed.digest());
    assert!(!bundle.matches_workflow(&slot_changed));

    // When: move input slot
    let mut parts = base_wf.to_parts();
    parts.input_slots = Box::from([CompiledInputSlot {
        slot: SlotIdx::new(1),
        kind: InputSlotKind::I64,
    }]);
    let slot_moved = CompiledWorkflow::try_from_parts(parts)?;
    assert_eq!(base_wf.digest(), slot_moved.digest());
    assert!(!bundle.matches_workflow(&slot_moved));

    // When: change input kind
    let mut parts = base_wf.to_parts();
    parts.input_slots = Box::from([CompiledInputSlot {
        slot: SlotIdx::new(0),
        kind: InputSlotKind::Bool,
    }]);
    let kind_changed = CompiledWorkflow::try_from_parts(parts)?;
    assert_eq!(base_wf.digest(), kind_changed.digest());
    assert!(!bundle.matches_workflow(&kind_changed));

    Ok(())
}

#[test]
fn name_change_retains_match() -> TestResult<()> {
    // Given: base workflow and matching bundle
    let base_wf = fixture(
        2,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        8,
        1024,
    )?;
    let cp = checkpoint(8);
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &base_wf)?;

    // When: change only workflow name
    let mut parts = base_wf.to_parts();
    parts.name = Box::from("different-name");
    let renamed = CompiledWorkflow::try_from_parts(parts)?;

    // Then: same digest, matches_workflow returns true
    assert_eq!(base_wf.digest(), renamed.digest());
    assert!(bundle.matches_workflow(&renamed));

    Ok(())
}
