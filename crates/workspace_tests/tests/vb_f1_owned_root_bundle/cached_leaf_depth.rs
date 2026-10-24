use super::graph_depth::chain;
use super::support::{TestResult, fixture};
use bytes::Bytes;
use vb_core::ids::{BlobId, ListId, SlotIdx, SymbolId};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

fn cached_leaf_depth_case(
    leaf_kind: InputSlotKind,
    leaf_value: SlotValue,
    case_counter: u32,
) -> TestResult<u32> {
    let wf = fixture(
        2,
        vec![
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: leaf_kind,
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::List,
            },
        ],
        128,
        4,
    )?;

    let mut cp = chain(64)?;

    match leaf_kind {
        InputSlotKind::Symbol => {
            cp.symbols.push(Box::from("leaf"));
        }
        InputSlotKind::Blob => {
            cp.blobs.push(Bytes::from_static(b"x"));
        }
        _ => return Err("unexpected leaf kind".into()),
    }

    let last_list = cp.lists.last_mut().ok_or("missing last list")?;
    *last_list = Box::from([leaf_value]);

    let last_taints = cp.list_taints.last_mut().ok_or("missing last taints")?;
    *last_taints = Box::from([Taint::Clean]);

    let roots: Box<[(SlotIdx, SlotValue, Taint)]> = Box::new([
        (SlotIdx::new(0), leaf_value, Taint::Clean),
        (
            SlotIdx::new(1),
            SlotValue::List(ListId::new(0)),
            Taint::Clean,
        ),
    ]);

    let saved_roots = roots.clone();
    let saved_cp = cp.clone();

    let bundle = OwnedRootBundle::new(roots, cp, &wf)?;

    assert_eq!(bundle.roots(), saved_roots.as_ref());
    assert_eq!(bundle.checkpoint(), &saved_cp);
    assert_eq!(bundle.input_slots(), wf.input_slots());
    assert_eq!(bundle.resource_contract(), &wf.resource_contract());
    assert!(bundle.matches_workflow(&wf));

    let next = case_counter.checked_add(1).ok_or("counter overflow")?;
    Ok(next)
}

#[test]
fn cached_leaf_depth_64_boundary() -> TestResult<()> {
    let mut counter = 0u32;

    counter = cached_leaf_depth_case(
        InputSlotKind::Symbol,
        SlotValue::Symbol(SymbolId::new(0)),
        counter,
    )?;

    counter = cached_leaf_depth_case(
        InputSlotKind::Blob,
        SlotValue::Blob(BlobId::new(0)),
        counter,
    )?;

    assert_eq!(counter, 2);
    Ok(())
}
