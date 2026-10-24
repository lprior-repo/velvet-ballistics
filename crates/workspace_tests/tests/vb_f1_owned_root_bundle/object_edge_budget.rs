use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::{ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::{ObjectField, ValueStoreCheckpointV1};
use vb_core::workflow::{CompiledInputSlot, CompiledWorkflow, InputSlotKind};

fn object_fixture(
    add_list: bool,
) -> TestResult<(
    CompiledWorkflow,
    ValueStoreCheckpointV1,
    Box<[(SlotIdx, SlotValue, Taint)]>,
)> {
    let mut cp = checkpoint(65535);
    let mut roots = Vec::new();
    let mut input_slots = Vec::new();

    for key in 0_u32..16_u32 {
        cp.symbols.push(format!("key_{key}").into_boxed_str());
    }

    for object in 0_u32..31_250_u32 {
        let mut fields = Vec::new();
        for key in 0_u32..16_u32 {
            fields.push(ObjectField::clean(SymbolId::new(key), SlotValue::I64(7)));
        }
        cp.objects.push(fields.into_boxed_slice());
        let slot = SlotIdx::new(u16::try_from(object)?);
        roots.push((slot, SlotValue::Object(ObjectId::new(object)), Taint::Clean));
        input_slots.push(CompiledInputSlot {
            slot,
            kind: InputSlotKind::Object,
        });
    }

    if add_list {
        let slot = SlotIdx::new(31_250);
        cp.lists.push(Box::from([SlotValue::Null]));
        cp.list_taints.push(Box::from([Taint::Clean]));
        roots.push((slot, SlotValue::List(ListId::new(0)), Taint::Clean));
        input_slots.push(CompiledInputSlot {
            slot,
            kind: InputSlotKind::List,
        });
    }

    let slot_count = u16::try_from(roots.len())?;
    let wf = fixture(slot_count, input_slots, 65535, 4)?;
    Ok((wf, cp, roots.into_boxed_slice()))
}

use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};

#[test]
fn owned_root_bundle_accepts_one_million_edges() -> TestResult<()> {
    let (wf, cp, roots) = object_fixture(false)?;
    let cp_clone = cp.clone();
    let roots_clone = roots.clone();
    let bundle = OwnedRootBundle::new(roots_clone, cp_clone, &wf)?;
    assert_eq!(bundle.roots(), roots.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert_eq!(bundle.input_slots(), wf.input_slots());
    assert_eq!(bundle.resource_contract(), &wf.resource_contract());
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}

#[test]
fn owned_root_bundle_rejects_one_million_plus_one_edges() -> TestResult<()> {
    let (wf, cp, roots) = object_fixture(true)?;
    let err = OwnedRootBundle::new(roots, cp, &wf)
        .err()
        .ok_or("expected quota error")?;
    match err {
        OwnedRootBundleError::GraphEdgeWorkExceeded { edges, limit } => {
            assert_eq!(edges, 1_000_001);
            assert_eq!(limit, 1_000_000);
        }
        other => return Err(format!("wrong error: {other:?}").into()),
    }
    Ok(())
}
