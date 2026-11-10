use super::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn bounded_physical_dags_preserve_aliases_and_secret_descendants(
        levels in 1_usize..32,
        fanout in 1_usize..3,
        secret in any::<bool>(),
    ) {
        let result = property(levels, fanout, secret);
        prop_assert_eq!(result.map_err(|error| error.to_string()), Ok(()));
    }
}

fn property(levels: usize, fanout: usize, secret: bool) -> TestResult {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let taint = if secret { Taint::Secret } else { Taint::Clean };
    let mut root = SlotValue::List(
        store.insert_list_with_taint(Box::new([SlotValue::I64(7)]), Box::new([taint]))?,
    );
    for _ in 0..levels {
        root = SlotValue::List(store.insert_list(vec![root; fanout].into_boxed_slice())?);
    }
    let before = store.checkpoint_v1();
    assert_eq!(
        validate_input_value(declaration, root, workflow.symbols_count(), Some(&before)),
        Ok(())
    );
    let bundle = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, taint)]),
        before.clone(),
        &workflow,
    )?;
    assert_eq!(bundle.root(declaration.slot), Some((root, taint)));
    assert_eq!(bundle.checkpoint().lists, before.lists);
    assert_eq!(bundle.checkpoint().list_taints, before.list_taints);
    Ok(())
}
