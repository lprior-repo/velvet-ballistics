use vb_core::ids::{ListId, SlotIdx};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, CompiledWorkflow, InputSlotKind};

use super::support::{TestResult, checkpoint, fixture};

#[test]
fn scalar_i64_root_clean_exact_retention() -> TestResult<()> {
    // Given: workflow with 1 slot, 1 I64 input slot, checkpoint cap=1
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        1,
        1024,
    )?;
    let cp = checkpoint(1);
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);

    // When: construct owned root bundle
    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf)?;

    // Then: exact retention of all projection fields
    assert_eq!(bundle.digest(), wf.digest());
    assert_eq!(bundle.frame_slot_count(), 1);
    assert_eq!(bundle.input_slots(), wf.input_slots());
    assert_eq!(bundle.resource_contract(), &wf.resource_contract());
    assert_eq!(bundle.input_slot_count(), 1);
    assert_eq!(bundle.roots(), roots.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert!(bundle.has_root(SlotIdx::new(0)));
    assert!(!bundle.has_root(SlotIdx::new(1)));
    match bundle.root(SlotIdx::new(0)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::I64(42));
            assert_eq!(taint, Taint::Clean);
        }
        None => return Err("root should be Some".into()),
    }
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}

#[test]
fn scalar_list_mixed_roots_valid() -> TestResult<()> {
    // Given: workflow with 2 slots, I64 and List input slots
    let wf = fixture(
        2,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::List,
            },
        ],
        4,
        1024,
    )?;
    let mut cp = checkpoint(4);
    cp.lists.push(Box::from([SlotValue::I64(99)]));
    cp.list_taints.push(Box::from([Taint::Clean]));
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
        (SlotIdx::new(0), SlotValue::I64(42), Taint::Clean),
        (
            SlotIdx::new(1),
            SlotValue::List(ListId::new(0)),
            Taint::Clean,
        ),
    ]);

    // When: construct owned root bundle
    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf)?;

    // Then: exact retention, both roots present, list reachable
    assert_eq!(bundle.input_slot_count(), 2);
    assert_eq!(bundle.roots(), roots.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert!(bundle.has_root(SlotIdx::new(0)));
    assert!(bundle.has_root(SlotIdx::new(1)));
    match bundle.root(SlotIdx::new(0)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::I64(42));
            assert_eq!(taint, Taint::Clean);
        }
        None => return Err("root 0 should be Some".into()),
    }
    match bundle.root(SlotIdx::new(1)) {
        Some((value, taint)) => {
            assert_eq!(value, SlotValue::List(ListId::new(0)));
            assert_eq!(taint, Taint::Clean);
        }
        None => return Err("root 1 should be Some".into()),
    }
    assert_eq!(bundle.checkpoint().lists.len(), 1);
    assert_eq!(bundle.checkpoint().list_taints.len(), 1);
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}

#[test]
fn unsorted_root_permutations_same_bundle() -> TestResult<()> {
    // Given: workflow with 2 I64 input slots
    let wf = fixture(
        2,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::I64,
            },
        ],
        2,
        1024,
    )?;
    let cp1 = checkpoint(2);
    let cp2 = checkpoint(2);
    let roots_a: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
        (SlotIdx::new(0), SlotValue::I64(10), Taint::Clean),
        (SlotIdx::new(1), SlotValue::I64(20), Taint::Clean),
    ]);
    let roots_b: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
        (SlotIdx::new(1), SlotValue::I64(20), Taint::Clean),
        (SlotIdx::new(0), SlotValue::I64(10), Taint::Clean),
    ]);

    // When: construct bundles from both orderings
    let bundle_a = OwnedRootBundle::new(roots_a.clone(), cp1.clone(), &wf)?;
    let bundle_b = OwnedRootBundle::new(roots_b.clone(), cp2.clone(), &wf)?;

    // Then: same sorted roots, projection, checkpoint, root count
    let expected: Box<[(SlotIdx, SlotValue, Taint)]> = Box::from([
        (SlotIdx::new(0), SlotValue::I64(10), Taint::Clean),
        (SlotIdx::new(1), SlotValue::I64(20), Taint::Clean),
    ]);
    assert_eq!(bundle_a.roots(), expected.as_ref());
    assert_eq!(bundle_b.roots(), expected.as_ref());
    assert_eq!(bundle_a.input_slots(), bundle_b.input_slots());
    assert_eq!(bundle_a.resource_contract(), bundle_b.resource_contract());
    assert_eq!(bundle_a.checkpoint(), bundle_b.checkpoint());
    assert_eq!(bundle_a.input_slot_count(), 2);
    assert_eq!(bundle_b.input_slot_count(), 2);
    assert!(bundle_a.has_root(SlotIdx::new(0)));
    assert!(bundle_a.has_root(SlotIdx::new(1)));
    assert!(bundle_b.has_root(SlotIdx::new(0)));
    assert!(bundle_b.has_root(SlotIdx::new(1)));
    Ok(())
}

#[test]
fn different_resource_contract_fails_matches() -> TestResult<()> {
    // Given: workflow and matching bundle
    let wf = fixture(
        1,
        vec![CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64,
        }],
        1,
        1024,
    )?;
    let cp = checkpoint(1);
    let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
        Box::from([(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)]);
    let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf)?;

    // When: create workflow with same digest but different max_slots
    let mut parts = wf.to_parts();
    parts.resource_contract.max_slots = 2;
    let wf2 = CompiledWorkflow::try_from_parts(parts)?;

    // Then: same digest, different resource contract, matches_workflow returns false
    assert_eq!(wf.digest(), wf2.digest());
    assert!(!bundle.matches_workflow(&wf2));
    Ok(())
}
