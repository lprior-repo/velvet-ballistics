use super::*;

#[test]
fn strict_owned_foreach_partial_run_recovers_and_resumes_after_reopen() -> TestResult {
    let source = multi_source("$step.loop.0 * 100 + $step.loop.1 * 10 + $step.loop.2");
    let cut = cut_at(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Secret, Taint::Clean, Taint::DerivedFromSecret],
        (body_step(&source)?, StepState::Succeeded),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(312), Taint::Secret),
        "ordered materialization and taint must survive recovery"
    );
    Ok(())
}

#[test]
fn empty_and_single_owned_foreach_sources_recover_after_reopen() -> TestResult {
    for (values, taints, result, expected, taint) in [
        (
            Vec::new(),
            Vec::new(),
            "count($step.loop)",
            SlotValue::I64(0),
            Taint::Clean,
        ),
        (
            vec![SlotValue::I64(7)],
            vec![Taint::Secret],
            "$step.loop.0",
            SlotValue::I64(7),
            Taint::Secret,
        ),
    ] {
        let source = multi_source(result);
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let stop = if values.is_empty() {
            (
                step_with(&workflow, |kind| matches!(kind, K::ForEachStart { .. }))?,
                StepState::Succeeded,
            )
        } else {
            (body_step(&source)?, StepState::Succeeded)
        };
        let cut = cut_at(&source, &values, &taints, stop)?;
        replay_parity(&cut)?;
        let events = recover_and_finish(&cut)?;
        assert_eq!(terminal_result(&events)?, (expected, taint));
    }
    Ok(())
}

#[test]
fn nested_owned_foreach_recovers_outer_inner_activation_order() -> TestResult {
    let source = nested_source();
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    let inner_body = step_with(&workflow, |kind| matches!(kind, K::EvalExpr { .. }))?;
    let cut = cut_with(&source, (inner_body, StepState::Succeeded), |store| {
        let first = store.insert_list_with_taint(
            Box::from([SlotValue::I64(4), SlotValue::I64(1)]),
            Box::from([Taint::Secret, Taint::Secret]),
        )?;
        let second = store
            .insert_list_with_taint(Box::from([SlotValue::I64(9)]), Box::from([Taint::Secret]))?;
        let outer = store.insert_list_with_taint(
            Box::from([SlotValue::List(first), SlotValue::List(second)]),
            Box::from([Taint::Secret, Taint::Secret]),
        )?;
        Ok((outer, Taint::Secret))
    })?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::Bool(true), Taint::Secret)
    );
    Ok(())
}

#[test]
fn repeated_owned_foreach_activation_rearms_body_steps() -> TestResult {
    let source = multi_source("count($step.loop)");
    let cut = cut_at_nth(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(1), SlotValue::I64(2)],
        &[Taint::Secret, Taint::Clean, Taint::Secret],
        (body_step(&source)?, StepState::Succeeded),
        2,
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(3), Taint::Secret)
    );
    Ok(())
}

#[test]
fn owned_reduce_recovers_after_reopen() -> TestResult {
    let source = reduce_source();
    let cut = cut_at(
        &source,
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(1)],
        &[Taint::Secret, Taint::Clean, Taint::Clean],
        (
            control_step(&source, |kind| matches!(kind, K::ReduceFinish { .. }))?,
            StepState::Succeeded,
        ),
    )?;
    let replay = replay_parity(&cut)?;
    let input = cut
        .workflow
        .input_slots()
        .first()
        .ok_or("missing Reduce input")?
        .slot;
    assert_eq!(
        replay.frame.read_taint(input)?,
        Taint::Secret,
        "Reduce current item retains its element/container taint"
    );
    let accumulator = cut
        .workflow
        .node(control_step(&source, |kind| {
            matches!(kind, K::ReduceStart { .. })
        })?)
        .and_then(|node| match &node.kind {
            K::ReduceStart { accumulator, .. } => Some(*accumulator),
            _ => None,
        })
        .ok_or("missing Reduce accumulator")?;
    assert_eq!(
        replay.frame.read_taint(accumulator)?,
        Taint::Clean,
        "the accumulator expression reads only the clean initial value and increments it"
    );
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(13), Taint::Clean)
    );
    Ok(())
}

#[test]
fn owned_together_recovers_after_reopen() -> TestResult {
    let source = together_source();
    let cut = cut_at(
        &source,
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(1)],
        &[Taint::Clean, Taint::Clean, Taint::Clean],
        (
            control_step(&source, |kind| matches!(kind, K::TogetherJoin { .. }))?,
            StepState::Succeeded,
        ),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(
        terminal_result(&events)?,
        (SlotValue::I64(10), Taint::Clean)
    );
    Ok(())
}

#[test]
fn owned_repeat_recovers_after_reopen() -> TestResult {
    let source = repeat_source();
    let cut = cut_at(
        &source,
        &[SlotValue::I64(4), SlotValue::I64(7), SlotValue::I64(1)],
        &[Taint::Clean, Taint::Clean, Taint::Clean],
        (
            control_step(&source, |kind| matches!(kind, K::RepeatFinish { .. }))?,
            StepState::Succeeded,
        ),
    )?;
    replay_parity(&cut)?;
    let events = recover_and_finish(&cut)?;
    assert_eq!(terminal_result(&events)?, (SlotValue::I64(3), Taint::Clean));
    Ok(())
}

#[test]
fn owned_foreach_object_materialization_replays_handles() -> TestResult {
    let source = object_source();
    let join = control_step(&source, |kind| matches!(kind, K::ForEachJoin { .. }))?;
    let cut = cut_with(&source, (join, StepState::Succeeded), |store| {
        let key = store.insert_symbol("entry")?;
        let first = store.insert_object(Box::from([ObjectField::with_taint(
            key,
            SlotValue::I64(4),
            Taint::Secret,
        )]))?;
        let second = store.insert_object(Box::from([ObjectField::with_taint(
            key,
            SlotValue::I64(7),
            Taint::Clean,
        )]))?;
        let list = store.insert_list_with_taint(
            Box::from([SlotValue::Object(first), SlotValue::Object(second)]),
            Box::from([Taint::Secret, Taint::Clean]),
        )?;
        Ok((list, Taint::Secret))
    })?;
    let replay = replay_parity(&cut)?;
    let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
    let output = cut
        .workflow
        .node(join)
        .and_then(|node| node.output.slot())
        .ok_or("missing ForEachJoin output")?;
    let SlotValue::List(list) = *replay.frame.read_slot(output)? else {
        return Err("ForEachJoin output is not a list".into());
    };
    let items = store.list(list)?;
    assert_eq!(items.len(), 2);
    let mut values = Vec::new();
    for item in items {
        let SlotValue::Object(object) = item else {
            return Err("ForEach object result is not an object".into());
        };
        let fields = store.object(*object)?;
        assert_eq!(fields.len(), 1);
        values.push(fields.first().ok_or("missing object result field")?.value);
    }
    assert_eq!(values, [SlotValue::I64(4), SlotValue::I64(7)]);

    let events = recover_and_finish(&cut)?;
    let (result, result_taint) = terminal_result(&events)?;
    assert_eq!(result_taint, Taint::Secret);
    let SlotValue::List(result_list) = result else {
        return Err("recovered ForEach result is not a list".into());
    };
    let recovered =
        replay_owned_for_each(&events, &cut.workflow, &cut.roots, Some(&cut.input_store))?
            .ok_or("missing recovered object replay")?;
    let recovered_store = ValueStore::from_checkpoint_v1(recovered.store)?;
    let actual = recovered_store
        .list_items_with_taint(result_list)?
        .collect::<Vec<_>>();
    assert_eq!(actual.len(), 2);
    let expected = [
        (SlotValue::I64(4), Taint::Secret, Taint::Secret),
        (SlotValue::I64(7), Taint::Secret, Taint::Clean),
    ];
    for ((item, item_taint), (expected_value, expected_item_taint, expected_field_taint)) in
        actual.into_iter().zip(expected)
    {
        assert_eq!(item_taint, expected_item_taint);
        let SlotValue::Object(object) = item else {
            return Err("recovered ForEach item is not an object".into());
        };
        let fields = recovered_store.object(object)?;
        let field = fields.first().ok_or("missing recovered object field")?;
        assert_eq!(field.value, expected_value);
        assert_eq!(field.taint, expected_field_taint);
    }
    Ok(())
}
