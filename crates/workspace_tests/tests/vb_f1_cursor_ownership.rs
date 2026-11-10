#![forbid(unsafe_code)]

use vb_core::workflow::{ErrorRouting, NodeOutput, node};
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, ConstIdx, ConstValue, SlotIdx, StepIdx, WorkflowError,
    WorkflowParts,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn s(value: u16) -> SlotIdx {
    SlotIdx::new(value)
}
fn t(value: u16) -> StepIdx {
    StepIdx::new(value)
}

fn fixture(reduce: bool, nested: bool) -> TestResult<WorkflowParts> {
    let mut parts = vb_compile::compile_workflow(b"version: velvet-ballistics/v1\nname: cursor_ownership\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: answer, value: '42'}\n  - id: done\n    finish: {result: answer}\n")?.to_parts();
    parts.nodes = Box::from([
        node(
            t(0),
            K::SetConst {
                value: ConstIdx::new(0),
            },
            Some(s(0)),
            Some(t(1)),
        ),
        node(
            t(1),
            K::SetConst {
                value: ConstIdx::new(1),
            },
            Some(s(1)),
            Some(t(2)),
        ),
        node(
            t(2),
            K::SetConst {
                value: ConstIdx::new(2),
            },
            Some(s(2)),
            Some(t(3)),
        ),
        node(
            t(3),
            K::BuildList {
                items: Box::from([s(1), s(2)]),
            },
            Some(s(3)),
            Some(t(4)),
        ),
        node(
            t(4),
            K::BuildList {
                items: Box::from([s(0), s(1)]),
            },
            Some(s(4)),
            Some(t(5)),
        ),
        node(t(5), start(reduce, 6, 8), Some(s(6)), None),
        node(t(6), K::Copy { source: s(4) }, Some(s(6)), Some(t(7))),
        node(
            t(7),
            next(reduce, 6, 8),
            Some(if reduce { s(3) } else { s(5) }),
            None,
        ),
        node(
            t(8),
            K::Finish {
                result: if reduce { s(5) } else { s(3) },
            },
            None,
            None,
        ),
    ]);
    if nested {
        let prefix = parts.nodes.iter().take(5).cloned().collect::<Vec<_>>();
        let mut nodes = prefix;
        nodes.extend([
            node(
                t(5),
                K::RepeatStart {
                    max_attempts: 2,
                    body: t(6),
                    done: t(11),
                },
                Some(s(7)),
                None,
            ),
            node(t(6), start(reduce, 7, 9), Some(s(6)), None),
            node(t(7), K::Copy { source: s(4) }, Some(s(6)), Some(t(8))),
            node(
                t(8),
                next(reduce, 7, 9),
                Some(if reduce { s(3) } else { s(5) }),
                None,
            ),
            node(t(9), K::Nop, None, Some(t(10))),
            node(
                t(10),
                K::RepeatCheck {
                    attempt_slot: s(7),
                    done: t(11),
                },
                Some(s(7)),
                Some(t(6)),
            ),
            node(
                t(11),
                K::Finish {
                    result: if reduce { s(5) } else { s(3) },
                },
                None,
                None,
            ),
        ]);
        parts.nodes = nodes.into_boxed_slice();
    }
    parts.step_names = (0..parts.nodes.len())
        .map(|id| format!("n{id}").into_boxed_str())
        .collect();
    parts.slot_count = if nested { 8 } else { 7 };
    parts.resource_contract.max_slots = if nested { 9 } else { 7 };
    parts.constants = Box::from([ConstValue::I64(0), ConstValue::I64(1), ConstValue::I64(2)]);
    parts.expressions = Box::new([]);
    parts.accessors = Box::new([]);
    parts.input_slots = Box::new([]);
    parts.source_metadata = Some(Box::new(
        vb_core::workflow::source_metadata::WorkflowSourceMetadata::try_new(
            Box::from([Box::<str>::from("prefix")]),
            Box::new([]),
        )?,
    ));
    parts.symbols_count = 1;
    Ok(parts)
}

fn start(reduce: bool, body: u16, done: u16) -> K {
    if reduce {
        K::ReduceStart {
            limit: 2,
            input: s(3),
            accumulator: s(5),
            initial: ConstIdx::new(0),
            body: t(body),
            done: t(done),
        }
    } else {
        K::ForEachStart {
            input: s(3),
            item_slot: s(5),
            limit: 2,
            body: t(body),
            done: t(done),
        }
    }
}
fn next(reduce: bool, body: u16, done: u16) -> K {
    if reduce {
        K::ReduceNext {
            iterator_slot: s(6),
            accumulator: s(5),
            body: t(body),
            done: t(done),
        }
    } else {
        K::ForEachNext {
            iterator_slot: s(6),
            body: t(body),
            done: t(done),
        }
    }
}
fn edit(parts: &mut WorkflowParts, id: u16) -> TestResult<&mut vb_core::CompiledNode> {
    parts
        .nodes
        .get_mut(usize::from(id))
        .ok_or_else(|| "node missing".into())
}
fn reject(parts: WorkflowParts, owner: u16, writer: u16) -> TestResult {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::LoopCursorForeignWrite {
            owner: actual_owner,
            writer: actual_writer,
            slot,
        }) => {
            assert_eq!(
                (actual_owner, actual_writer, slot),
                (t(owner), t(writer), s(6))
            );
            Ok(())
        }
        Err(error) => Err(format!("wrong rejection: {error:?}").into()),
        Ok(_) => Err(format!("accepted cursor reset: owner={owner}, writer={writer}").into()),
    }
}

#[test]
fn public_constructor_rejects_reviewed_foreach_reduce_and_nested_copy_resets() -> TestResult {
    for reduce in [false, true] {
        for nested in [false, true] {
            reject(
                fixture(reduce, nested)?,
                if nested { 6 } else { 5 },
                if nested { 7 } else { 6 },
            )?;
        }
    }
    Ok(())
}

#[test]
fn own_start_temporary_input_item_and_accumulator_aliases_preserve_progress() -> TestResult {
    for reduce in [false, true] {
        for alias in [0, 1] {
            let mut parts = fixture(reduce, false)?;
            edit(&mut parts, 6)?.kind = K::Copy { source: s(6) };
            edit(&mut parts, 6)?.output = NodeOutput::Slot(s(4));
            edit(&mut parts, 8)?.kind = K::Finish { result: s(0) };
            if alias == 0 {
                edit(&mut parts, 3)?.output = NodeOutput::Slot(s(6));
            }
            match &mut edit(&mut parts, 5)?.kind {
                K::ForEachStart {
                    input, item_slot, ..
                } => {
                    if alias == 0 {
                        *input = s(6);
                    } else {
                        *item_slot = s(6);
                    }
                }
                K::ReduceStart {
                    input, accumulator, ..
                } => {
                    if alias == 0 {
                        *input = s(6);
                    } else {
                        *accumulator = s(6);
                    }
                }
                _ => return Err("missing start".into()),
            }
            if reduce && alias == 1 {
                if let K::ReduceNext { accumulator, .. } = &mut edit(&mut parts, 7)?.kind {
                    *accumulator = s(6);
                }
            }
            assert_finished(parts, vb_core::SlotValue::I64(0), 2)?;
        }
    }
    Ok(())
}

#[test]
fn paired_next_temporary_output_and_exit_error_aliases_preserve_progress() -> TestResult {
    for reduce in [false, true] {
        for error_route in [false, true] {
            let mut parts = fixture(reduce, false)?;
            edit(&mut parts, 6)?.kind = K::Copy { source: s(6) };
            edit(&mut parts, 6)?.output = NodeOutput::Slot(s(4));
            edit(&mut parts, 8)?.kind = K::Finish { result: s(0) };
            let control = edit(&mut parts, 7)?;
            if error_route {
                control.routing.error = ErrorRouting::Handler {
                    handler: t(8),
                    error_slot: Some(s(6)),
                };
            } else {
                control.output = NodeOutput::Slot(s(6));
            }
            assert_finished(parts, vb_core::SlotValue::I64(0), 2)?;
        }
    }
    Ok(())
}

#[test]
fn owned_start_output_and_next_cursor_updates_execute_exactly_two_iterations() -> TestResult {
    for reduce in [false, true] {
        let mut parts = fixture(reduce, false)?;
        let body = edit(&mut parts, 6)?;
        body.kind = K::Copy { source: s(6) };
        body.output = NodeOutput::Slot(s(4));
        let expected = if reduce {
            vb_core::SlotValue::I64(0)
        } else {
            vb_core::SlotValue::List(vb_core::ListId::new(0))
        };
        assert_finished(parts, expected, 2)?;
    }
    Ok(())
}

#[test]
fn uniquely_matched_forward_only_next_controls_preserve_execution() -> TestResult {
    for reduce in [false, true] {
        let mut parts = fixture(reduce, false)?;
        let control = edit(&mut parts, 6)?;
        control.kind = next(reduce, 7, 8);
        control.output = NodeOutput::Slot(if reduce { s(3) } else { s(5) });
        control.routing.next = None.into();
        let finish = edit(&mut parts, 7)?;
        finish.kind = K::Finish { result: s(5) };
        finish.output = NodeOutput::Discard;
        let expected = vb_core::SlotValue::I64(if reduce { 0 } else { 2 });
        assert_finished(parts, expected, 1)?;
    }
    Ok(())
}

fn assert_finished(
    parts: WorkflowParts,
    expected: vb_core::SlotValue,
    body_visits: usize,
) -> TestResult {
    assert_finished_at(parts, expected, t(6), body_visits)
}

fn assert_finished_at(
    parts: WorkflowParts,
    expected: vb_core::SlotValue,
    body: StepIdx,
    body_visits: usize,
) -> TestResult {
    use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    let original = parts.clone();
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    let mut frame = new_run_frame(RunId::new(901), &workflow)?;
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    vb_core::workflow::source_metadata::seed_source_symbols(
        workflow.source_metadata().ok_or("metadata")?,
        &mut store,
    )?;
    let mut evidence = EvidenceCollector::new();
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(64),
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut vb_runtime::primitives::collect::CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )?;
    assert_eq!(signal, RuntimeSignal::Finished(expected));
    assert_eq!(frame.read_taint(s(6))?, Taint::Clean);
    let SlotValue::List(cursor) = frame.read_slot(s(6))? else {
        return Err("cursor is not a list".into());
    };
    assert_eq!(
        store.list(*cursor)?,
        &[SlotValue::I64(0), SlotValue::I64(2)]
    );
    assert_eq!(
        evidence
            .drain()
            .iter()
            .filter(
                |event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == body)
            )
            .count(),
        body_visits
    );
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}

#[test]
fn different_nested_starts_cannot_reuse_a_cursor_even_across_families() -> TestResult {
    for outer_reduce in [false, true] {
        for inner_reduce in [false, true] {
            let mut parts = fixture(inner_reduce, true)?;
            let outer = edit(&mut parts, 5)?;
            outer.kind = start(outer_reduce, 6, 11);
            outer.output = NodeOutput::Slot(s(6));
            let control = edit(&mut parts, 10)?;
            control.kind = next(outer_reduce, 6, 11);
            control.output = NodeOutput::Slot(if outer_reduce { s(3) } else { s(5) });
            control.routing.next = None.into();
            edit(&mut parts, 7)?.output = NodeOutput::Slot(s(4));
            reject(parts, 5, 6)?;
        }
    }
    Ok(())
}

#[test]
fn writes_before_and_after_cursor_activation_remain_valid() -> TestResult {
    for reduce in [false, true] {
        for writer in [4, 8] {
            let mut parts = fixture(reduce, false)?;
            edit(&mut parts, 6)?.kind = K::Nop;
            edit(&mut parts, 6)?.output = NodeOutput::Discard;
            let target = edit(&mut parts, writer)?;
            target.kind = K::Copy { source: s(3) };
            target.output = NodeOutput::Slot(s(6));
            if writer == 8 {
                target.routing.next = Some(t(9)).into();
                let mut nodes = parts.nodes.into_vec();
                nodes.push(node(
                    t(9),
                    K::Finish {
                        result: if reduce { s(5) } else { s(3) },
                    },
                    None,
                    None,
                ));
                parts.nodes = nodes.into_boxed_slice();
                let mut names = parts.step_names.into_vec();
                names.push("after".into());
                parts.step_names = names.into_boxed_slice();
            }
            let original = parts.clone();
            assert_eq!(
                CompiledWorkflow::try_from_parts(parts)?.to_parts(),
                original
            );
        }
    }
    Ok(())
}

#[test]
fn all_implicit_foreign_writers_are_checked_independently_of_node_output() -> TestResult {
    let kinds = [
        (
            K::ForEachStart {
                input: s(3),
                item_slot: s(6),
                limit: 2,
                body: t(7),
                done: t(8),
            },
            Some(s(7)),
        ),
        (
            K::ReduceStart {
                input: s(3),
                accumulator: s(6),
                initial: ConstIdx::new(0),
                limit: 2,
                body: t(7),
                done: t(8),
            },
            Some(s(7)),
        ),
        (
            K::ReduceStart {
                input: s(6),
                accumulator: s(5),
                initial: ConstIdx::new(0),
                limit: 2,
                body: t(7),
                done: t(8),
            },
            Some(s(7)),
        ),
        (
            K::CollectStart {
                source: s(6),
                limit: 2,
                page_size: 1,
                body: t(7),
                done: t(8),
            },
            None,
        ),
        (
            K::CollectNext {
                collector_slot: s(6),
                body: t(7),
                done: t(8),
            },
            None,
        ),
        (
            K::TogetherBranch {
                branch: 1,
                entry: t(7),
                join: t(8),
                accumulator: s(6),
            },
            Some(s(4)),
        ),
        (
            K::TogetherJoin {
                branch_count: 1,
                accumulator: s(6),
            },
            Some(s(4)),
        ),
        (
            K::RepeatCheck {
                attempt_slot: s(6),
                done: t(8),
            },
            None,
        ),
        (
            K::RetryCheck {
                policy_slot: s(6),
                body: t(7),
                exhausted: t(8),
            },
            None,
        ),
        (
            K::ForEachNext {
                iterator_slot: s(6),
                body: t(7),
                done: t(8),
            },
            Some(s(4)),
        ),
        (
            K::ReduceNext {
                iterator_slot: s(6),
                accumulator: s(5),
                body: t(7),
                done: t(8),
            },
            Some(s(4)),
        ),
    ];
    for reduce in [false, true] {
        for (kind, output) in &kinds {
            let mut parts = fixture(reduce, false)?;
            parts.slot_count = 8;
            parts.resource_contract.max_slots = 32;
            let writer = 6;
            let body = edit(&mut parts, writer)?;
            body.kind = kind.clone();
            body.output = (*output).into();
            reject(parts, 5, writer).map_err(|error| format!("{kind:?}: {error}"))?;
        }
    }
    Ok(())
}

#[test]
fn foreign_body_error_routes_cannot_write_an_active_cursor() -> TestResult {
    for reduce in [false, true] {
        for writer in [6] {
            let mut parts = fixture(reduce, false)?;
            edit(&mut parts, 6)?.output = NodeOutput::Slot(s(4));
            edit(&mut parts, writer)?.routing.error = ErrorRouting::Handler {
                handler: t(8),
                error_slot: Some(s(6)),
            };
            reject(parts, 5, writer)?;
        }
    }
    Ok(())
}

#[test]
fn ask_host_answer_boundary_cannot_replace_an_active_cursor() -> TestResult {
    for reduce in [false, true] {
        let mut parts = fixture(reduce, false)?;
        edit(&mut parts, 5)?.kind = start(reduce, 6, 9);
        let ask = edit(&mut parts, 6)?;
        ask.kind = K::Ask {
            prompt: s(0),
            timeout_slot: None,
        };
        ask.output = NodeOutput::Discard;
        let mut nodes = parts.nodes.iter().take(7).cloned().collect::<Vec<_>>();
        nodes.extend([
            node(t(7), K::AskResume { answer: s(6) }, Some(s(4)), Some(t(8))),
            node(
                t(8),
                next(reduce, 6, 9),
                Some(if reduce { s(3) } else { s(5) }),
                None,
            ),
            node(
                t(9),
                K::Finish {
                    result: if reduce { s(5) } else { s(3) },
                },
                None,
                None,
            ),
        ]);
        parts.nodes = nodes.into_boxed_slice();
        parts.step_names = (0..parts.nodes.len())
            .map(|id| format!("n{id}").into())
            .collect();
        reject(parts, 5, 6)?;
    }
    Ok(())
}

#[test]
fn nested_foreach_can_read_an_outer_cursor_without_writing_it() -> TestResult {
    let mut parts = fixture(false, true)?;
    parts.resource_contract.max_slots = 16;
    let outer = edit(&mut parts, 5)?;
    outer.kind = start(false, 6, 11);
    outer.output = NodeOutput::Slot(s(6));
    let inner = edit(&mut parts, 6)?;
    inner.kind = K::ForEachStart {
        input: s(6),
        item_slot: s(5),
        limit: 2,
        body: t(7),
        done: t(9),
    };
    inner.output = NodeOutput::Slot(s(7));
    let body = edit(&mut parts, 7)?;
    body.kind = K::Nop;
    body.output = NodeOutput::Discard;
    edit(&mut parts, 8)?.kind = K::ForEachNext {
        iterator_slot: s(7),
        body: t(7),
        done: t(9),
    };
    let control = edit(&mut parts, 10)?;
    control.kind = next(false, 6, 11);
    control.output = NodeOutput::Slot(s(5));
    control.routing.next = None.into();
    assert_finished_at(
        parts,
        vb_core::SlotValue::List(vb_core::ListId::new(0)),
        t(7),
        4,
    )
}

#[test]
fn read_only_and_identity_operations_do_not_reserve_their_input_cursor() -> TestResult {
    for kind in [
        K::Copy { source: s(6) },
        K::ForEachJoin { output: s(6) },
        K::ReduceFinish { accumulator: s(6) },
        K::RepeatFinish { result: s(6) },
        K::AskResume { answer: s(6) },
        K::Nop,
    ] {
        let mut parts = fixture(false, false)?;
        let body = edit(&mut parts, 6)?;
        body.kind = kind;
        body.output = NodeOutput::Slot(s(6));
        assert_finished(parts, vb_core::SlotValue::List(vb_core::ListId::new(0)), 2)?;
    }
    Ok(())
}

#[test]
fn eight_nested_foreach_and_reduce_levels_keep_distinct_cursors_and_finish() -> TestResult {
    use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, Taint, ValueStore, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    for reduce in [false, true] {
        let mut body = "- id: leaf\n  set: {output: leaf, value: '9'}\n".to_owned();
        for level in (0..8).rev() {
            let nested = body
                .lines()
                .map(|line| format!("      {line}\n"))
                .collect::<String>();
            body = if reduce {
                format!(
                    "- id: level_{level}\n  reduce:\n    variable: accumulator_{level}\n    input: $input.items\n    initial: '0'\n    steps:\n{nested}"
                )
            } else {
                format!(
                    "- id: level_{level}\n  for_each:\n    variable: item_{level}\n    input: $input.items\n    at_once: 1\n    steps:\n{nested}"
                )
            };
        }
        let steps = body
            .lines()
            .map(|line| format!("  {line}\n"))
            .collect::<String>();
        let source = format!(
            "version: velvet-ballistics/v1\nname: genuine_nested_cursors\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: number, max: 2}}}}\nvars: {{sentinel: 42, marker: source_payload}}\nsteps:\n{steps}  - id: done\n    finish: {{result: '$vars.sentinel'}}\n"
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let original = workflow.to_parts();
        let mut cursors = original
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, K::ForEachStart { .. } | K::ReduceStart { .. }))
            .filter_map(|node| node.output.slot())
            .collect::<Vec<_>>();
        assert_eq!(cursors.len(), 8);
        cursors.sort_unstable();
        cursors.dedup();
        assert_eq!(cursors.len(), 8);
        let leaf = original.nodes.iter().find(|node| matches!(node.kind, K::SetConst { value } if workflow.constant(value) == Some(&ConstValue::I64(9))))
            .ok_or("leaf")?.id;
        let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
        if let Some(metadata) = workflow.source_metadata() {
            vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)?;
        }
        let payload: Box<[SlotValue]> = Box::from([SlotValue::I64(3), SlotValue::I64(5)]);
        let input = store.insert_list(payload.clone())?;
        let mut frame = new_run_frame(RunId::new(902), &workflow)?;
        let input_slot = workflow.input_slots().first().ok_or("input")?.slot;
        frame.write_slot_with_taint(input_slot, SlotValue::List(input), Taint::Secret)?;
        let mut evidence = EvidenceCollector::new();
        let mut collect = vb_runtime::primitives::collect::CollectStates::new();
        let mut leaves = 0usize;
        let mut finished = None;
        for _ in 0..10_000 {
            let signal = drive_deterministic_full(
                &workflow,
                &mut frame,
                &mut StepBudget::new(1),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1,
            )?;
            leaves = leaves.checked_add(evidence.drain().iter().filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == leaf)).count()).ok_or("leaf count")?;
            match signal {
                RuntimeSignal::StepBudgetExhausted => {}
                RuntimeSignal::Finished(value) => {
                    finished = Some(value);
                    break;
                }
                other => return Err(format!("unexpected signal: {other:?}").into()),
            }
        }
        assert_eq!(finished, Some(SlotValue::I64(42)));
        assert_eq!(leaves, 2usize.checked_pow(8).ok_or("multiplicity")?);
        assert_eq!(store.list(input)?, payload.as_ref());
        assert_eq!(frame.read_slot(input_slot)?, &SlotValue::List(input));
        assert_eq!(frame.read_taint(input_slot)?, Taint::Secret);
        assert_eq!(workflow.to_parts(), original);
    }
    Ok(())
}
