use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::{BlobId, ListId, ObjectId, SlotIdx, SymbolId};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError, ValueKind};
use vb_core::value::{FiniteF64, SlotValue, Taint};
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

fn cases() -> TestResult<[(InputSlotKind, SlotValue, ValueKind); 8]> {
    Ok([
        (InputSlotKind::Null, SlotValue::Null, ValueKind::Null),
        (InputSlotKind::Bool, SlotValue::Bool(true), ValueKind::Bool),
        (InputSlotKind::I64, SlotValue::I64(-7), ValueKind::I64),
        (
            InputSlotKind::F64,
            SlotValue::F64(FiniteF64::new(1.25)?),
            ValueKind::F64,
        ),
        (
            InputSlotKind::Symbol,
            SlotValue::Symbol(SymbolId::new(0_u32)),
            ValueKind::Symbol,
        ),
        (
            InputSlotKind::List,
            SlotValue::List(ListId::new(0_u32)),
            ValueKind::List,
        ),
        (
            InputSlotKind::Object,
            SlotValue::Object(ObjectId::new(0_u32)),
            ValueKind::Object,
        ),
        (
            InputSlotKind::Blob,
            SlotValue::Blob(BlobId::new(0_u64)),
            ValueKind::Blob,
        ),
    ])
}

fn checkpoint_for(value: SlotValue) -> ValueStoreCheckpointV1 {
    let mut cp = checkpoint(1);
    match value {
        SlotValue::Symbol(_) => {
            cp.symbols.push(Box::from("s"));
        }
        SlotValue::List(_) => {
            cp.lists.push(Box::from([]));
            cp.list_taints.push(Box::from([]));
        }
        SlotValue::Object(_) => {
            cp.objects.push(Box::from([]));
        }
        SlotValue::Blob(_) => {
            cp.blobs.push(bytes::Bytes::from_static(b"x"));
        }
        _ => {}
    }
    cp
}

#[test]
fn all_kinds_and_any_accept_exactly() -> TestResult<()> {
    let test_cases = cases()?;
    let mut checked = 0_u64;

    for (kind, value, _vk) in test_cases {
        for declared_kind in [kind, InputSlotKind::Any] {
            checked = checked.checked_add(1).ok_or("case count overflow")?;

            let wf = fixture(
                1,
                vec![CompiledInputSlot {
                    slot: SlotIdx::new(0),
                    kind: declared_kind,
                }],
                1,
                4,
            )?;

            let cp = checkpoint_for(value);
            let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
                Box::from([(SlotIdx::new(0), value, Taint::Clean)]);

            let bundle = OwnedRootBundle::new(roots.clone(), cp.clone(), &wf)?;

            assert_eq!(bundle.roots(), roots.as_ref());
            assert_eq!(bundle.checkpoint(), &cp);
            assert_eq!(bundle.input_slots(), wf.input_slots());
            assert!(bundle.matches_workflow(&wf));
        }
    }

    assert_eq!(checked, 16);
    Ok(())
}

#[test]
fn every_strict_kind_mismatch_is_typed() -> TestResult<()> {
    let cases = cases()?;
    let mut count: u64 = 0;
    for expected in cases.iter() {
        for actual in cases.iter() {
            if expected.0 == actual.0 {
                continue;
            }
            let wf = fixture(
                1,
                vec![CompiledInputSlot {
                    slot: SlotIdx::new(0),
                    kind: expected.0,
                }],
                1,
                4,
            )?;
            let roots: Box<[(SlotIdx, SlotValue, Taint)]> =
                Box::new([(SlotIdx::new(0), actual.1, Taint::Clean)]);
            let result = OwnedRootBundle::new(roots, checkpoint_for(actual.1), &wf);
            let err = result.err().ok_or("expected kind mismatch")?;
            match err {
                OwnedRootBundleError::RootKindMismatch {
                    slot,
                    expected: found_expected,
                    actual: found_actual,
                } => {
                    assert_eq!(slot, SlotIdx::new(0));
                    assert_eq!(found_expected, expected.0);
                    assert_eq!(found_actual, actual.2);
                }
                other => {
                    return Err(format!("wrong error: {other:?}").into());
                }
            }
            count = count.checked_add(1).ok_or("counter overflow")?;
        }
    }
    assert_eq!(count, 56);
    Ok(())
}
