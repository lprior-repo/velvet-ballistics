use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::{ListId, SlotIdx};
use vb_core::root_bundle::{OwnedRootBundle, OwnedRootBundleError};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

const LIST_COUNT: usize = 16;
const PER_LIST: usize = 65535;
const LAST_LIST: usize = 16975;

fn build_lists() -> (Vec<Box<[SlotValue]>>, Vec<Box<[Taint]>>) {
    let mut lists = Vec::with_capacity(LIST_COUNT);
    let mut taints = Vec::with_capacity(LIST_COUNT);
    for i in 0..LIST_COUNT {
        let len = if i == 15 { LAST_LIST } else { PER_LIST };
        let values: Box<[SlotValue]> = vec![SlotValue::Null; len].into_boxed_slice();
        let t: Box<[Taint]> = vec![Taint::Clean; len].into_boxed_slice();
        lists.push(values);
        taints.push(t);
    }
    (lists, taints)
}

fn build_roots(list_ids: &[ListId]) -> TestResult<Box<[(SlotIdx, SlotValue, Taint)]>> {
    let mut roots = Vec::with_capacity(LIST_COUNT);
    for (i, id) in list_ids.iter().enumerate() {
        let slot = SlotIdx::new(u16::try_from(i)?);
        roots.push((slot, SlotValue::List(*id), Taint::Clean));
    }
    Ok(roots.into_boxed_slice())
}

#[test]
fn exact_one_million_edges_accepted() -> TestResult<()> {
    // Given: 15 lists of 65535 + 1 list of 16975 = 1,000,000 edges
    let (lists, taints) = build_lists();
    let mut cp = checkpoint(16);
    cp.lists = lists;
    cp.list_taints = taints;
    let inputs: Vec<CompiledInputSlot> = (0..16_u16)
        .map(|i| CompiledInputSlot {
            slot: SlotIdx::new(i),
            kind: InputSlotKind::List,
        })
        .collect();
    let wf = fixture(16, inputs, 16, 4)?;
    let list_ids: Vec<ListId> = (0..16_u32).map(|i| ListId::new(i)).collect();
    let roots = build_roots(&list_ids)?;
    let roots_clone = roots.clone();
    let cp_clone = cp.clone();
    // When
    let bundle = OwnedRootBundle::new(roots, cp, &wf)?;
    // Then: full retention verified
    assert_eq!(bundle.roots().len(), LIST_COUNT);
    for (i, root) in bundle.roots().iter().enumerate() {
        let expected = roots_clone.get(i).ok_or("missing root")?;
        assert_eq!(root.0, expected.0);
        assert_eq!(root.1, expected.1);
        assert_eq!(root.2, expected.2);
    }
    assert_eq!(bundle.checkpoint(), &cp_clone);
    assert_eq!(bundle.roots(), roots_clone.as_ref());
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}

#[test]
fn one_million_plus_one_edges_rejected() -> TestResult<()> {
    // Given: 15 lists of 65535 + 1 list of 16976 = 1,000,001 edges
    let mut lists = Vec::with_capacity(LIST_COUNT);
    let mut taints = Vec::with_capacity(LIST_COUNT);
    for _ in 0..15 {
        let values: Box<[SlotValue]> = vec![SlotValue::Null; PER_LIST].into_boxed_slice();
        let t: Box<[Taint]> = vec![Taint::Clean; PER_LIST].into_boxed_slice();
        lists.push(values);
        taints.push(t);
    }
    let last_values: Box<[SlotValue]> = vec![SlotValue::Null; 16976].into_boxed_slice();
    let last_taints: Box<[Taint]> = vec![Taint::Clean; 16976].into_boxed_slice();
    lists.push(last_values);
    taints.push(last_taints);
    let mut cp = checkpoint(16);
    cp.lists = lists;
    cp.list_taints = taints;
    let inputs: Vec<CompiledInputSlot> = (0..16_u16)
        .map(|i| CompiledInputSlot {
            slot: SlotIdx::new(i),
            kind: InputSlotKind::List,
        })
        .collect();
    let wf = fixture(16, inputs, 16, 4)?;
    let list_ids: Vec<ListId> = (0..16_u32).map(|i| ListId::new(i)).collect();
    let roots = build_roots(&list_ids)?;
    // When
    let result = OwnedRootBundle::new(roots, cp, &wf);
    // Then
    let err = result.err().ok_or("expected error")?;
    match err {
        OwnedRootBundleError::GraphEdgeWorkExceeded { edges, limit } => {
            assert_eq!(edges, 1_000_001);
            assert_eq!(limit, 1_000_000);
        }
        other => return Err(format!("wrong: {:?}", other).into()),
    }
    Ok(())
}
