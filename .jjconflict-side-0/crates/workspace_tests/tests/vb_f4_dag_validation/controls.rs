use super::*;
use vb_core::input_validation::InputValueIssue;
use vb_core::root_bundle::{OwnedRootBundleError, errors::ArenaNode};
use vb_core::{CoreError, ListId, ObjectField};

fn check(
    workflow: &CompiledWorkflow,
    store: &ValueStore,
    root: SlotValue,
) -> TestResult<Result<(), InputValueIssue>> {
    let declaration = workflow.input_slots().first().ok_or("input")?;
    Ok(validate_input_value(
        declaration,
        root,
        workflow.symbols_count(),
        Some(&store.checkpoint_v1()),
    ))
}

#[test]
fn shared_handle_is_rechecked_under_a_different_schema() -> TestResult {
    let text = SOURCE.replace("{is: list, of: any, max: 2}", "{is: object, fields: {strict: {is: object, fields: {count: {is: number, min: 2}}}, loose: {is: object, fields: {count: {is: number, min: 0}}}}}");
    let workflow = vb_compile::compile_workflow(text.as_bytes())?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let count = store.insert_symbol("count")?;
    let strict = store.insert_symbol("strict")?;
    let loose = store.insert_symbol("loose")?;
    let shared = SlotValue::Object(
        store.insert_object(Box::from([ObjectField::clean(count, SlotValue::I64(1))]))?,
    );
    // LIFO traversal completes loose first, before visiting the strict alias.
    let root = SlotValue::Object(store.insert_object(Box::from([
        ObjectField::clean(strict, shared),
        ObjectField::clean(loose, shared),
    ]))?);
    assert_eq!(
        check(&workflow, &store, root)?,
        Err(InputValueIssue::Bounds)
    );
    let result = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    );
    let Err(OwnedRootBundleError::RootConstraintMismatch { slot, issue }) = result else {
        return Err("strict alias must reject with its bound error".into());
    };
    assert_eq!(slot, declaration.slot);
    assert_eq!(issue, InputValueIssue::Bounds);
    Ok(())
}

fn depth_fixture(wrappers: usize) -> TestResult<(CompiledWorkflow, ValueStore, SlotValue)> {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let shared = SlotValue::List(store.insert_list(Box::from([SlotValue::I64(7)]))?);
    let mut deep = shared;
    for _ in 0..wrappers {
        deep = SlotValue::List(store.insert_list(Box::from([deep]))?);
    }
    // The shallow alias is completed before the same handle's deeper occurrence.
    let root = SlotValue::List(store.insert_list(Box::from([deep, shared]))?);
    Ok((workflow, store, root))
}

#[test]
fn shallow_completion_does_not_hide_deeper_overflow() -> TestResult {
    for wrappers in [62, 63] {
        let (workflow, store, root) = depth_fixture(wrappers)?;
        let declaration = workflow.input_slots().first().ok_or("input")?;
        let bundle = OwnedRootBundle::new(
            Box::new([(declaration.slot, root, Taint::Clean)]),
            store.checkpoint_v1(),
            &workflow,
        );
        if wrappers == 62 {
            assert_eq!(check(&workflow, &store, root)?, Ok(()));
            let bundle = bundle?;
            assert_eq!(bundle.root(declaration.slot), Some((root, Taint::Clean)));
        } else {
            assert_eq!(
                check(&workflow, &store, root)?,
                Err(InputValueIssue::DepthExceeded)
            );
            let Err(OwnedRootBundleError::GraphDepth { depth, .. }) = bundle else {
                return Err("graph depth must reject".into());
            };
            assert_eq!(depth, 65);
        }
    }
    Ok(())
}

#[test]
fn cycles_and_dangling_references_are_not_completed() -> TestResult {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let id = store.insert_list(Box::from([SlotValue::List(ListId::new(0))]))?;
    let root = SlotValue::List(id);
    assert_eq!(
        check(&workflow, &store, root)?,
        Err(InputValueIssue::DepthExceeded)
    );
    let result = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    );
    let Err(OwnedRootBundleError::GraphCycle { node }) = result else {
        return Err("cycle must reject".into());
    };
    assert_eq!(node, ArenaNode::List(id));
    let dangling = ListId::new(u32::MAX);
    let root = SlotValue::List(dangling);
    assert_eq!(
        check(&workflow, &store, root)?,
        Err(InputValueIssue::Payload)
    );
    let result = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    );
    let Err(OwnedRootBundleError::Checkpoint(CoreError::ListOutOfBounds { list })) = result else {
        return Err("dangling reference must reject".into());
    };
    assert_eq!(list, dangling);
    Ok(())
}

#[test]
fn element_kind_and_descendant_taint_checks_remain_active() -> TestResult {
    let text = SOURCE.replace("of: any", "of: number");
    let workflow = vb_compile::compile_workflow(text.as_bytes())?;
    let declaration = workflow.input_slots().first().ok_or("input")?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let root = SlotValue::List(
        store
            .insert_list_with_taint(Box::new([SlotValue::Bool(true)]), Box::new([Taint::Secret]))?,
    );
    assert_eq!(
        check(&workflow, &store, root)?,
        Err(InputValueIssue::Element)
    );
    let result = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Clean)]),
        store.checkpoint_v1(),
        &workflow,
    );
    let Err(OwnedRootBundleError::TaintMismatch {
        slot,
        expected,
        actual,
    }) = result
    else {
        return Err("descendant taint must reject".into());
    };
    assert_eq!(slot, declaration.slot);
    assert_eq!(expected, Taint::Secret);
    assert_eq!(actual, Taint::Clean);
    let result = OwnedRootBundle::new(
        Box::new([(declaration.slot, root, Taint::Secret)]),
        store.checkpoint_v1(),
        &workflow,
    );
    let Err(OwnedRootBundleError::RootConstraintMismatch { slot, issue }) = result else {
        return Err("tainted wrong element must still reject".into());
    };
    assert_eq!(slot, declaration.slot);
    assert_eq!(issue, InputValueIssue::Element);
    Ok(())
}

#[test]
fn scalar_and_cached_handle_work_still_exhaust_the_original_cap() -> TestResult {
    let workflow = vb_compile::compile_workflow(SOURCE.as_bytes())?;
    let mut declaration = workflow.input_slots().first().ok_or("input")?.clone();
    declaration.constraint.max = None;
    for cached in [false, true] {
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        let leaf = if cached {
            SlotValue::List(store.insert_list(Box::<[SlotValue]>::default())?)
        } else {
            SlotValue::I64(7)
        };
        let mut children = Vec::new();
        for _ in 0..16 {
            children.push(SlotValue::List(
                store.insert_list(vec![leaf; 65_535].into_boxed_slice())?,
            ));
        }
        let root = SlotValue::List(store.insert_list(children.into_boxed_slice())?);
        assert_eq!(
            validate_input_value(
                &declaration,
                root,
                workflow.symbols_count(),
                Some(&store.checkpoint_v1())
            ),
            Err(InputValueIssue::Capacity)
        );
    }
    Ok(())
}
