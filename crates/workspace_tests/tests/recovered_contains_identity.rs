#![forbid(unsafe_code)]

use vb_core::value_store::ObjectField;
use vb_core::{RunId, SlotValue, StepIdx, Taint, ValueStore, new_run_frame};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn compound_contains_uses_valid_handle_identity_in_expression_core_and_replay() -> TestResult {
    let source = b"version: velvet-ballistics/v1\nname: contains_identity\nwhen: {manual: {}}\ninputs: {haystack: any, needle: any}\nsteps:\n  - id: calculate\n    set: {output: answer, value: 'contains($input.haystack, $input.needle)'}\n  - id: done\n    finish: {result: answer}\n";
    let workflow = vb_compile::compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::ZERO)
        .ok_or("missing expression node")?;
    let output = node.output.slot().ok_or("missing output slot")?;
    let mut store = ValueStore::new();
    let key = store.insert_symbol("key")?;
    let list_a = store.insert_list(vec![SlotValue::I64(7)])?;
    let list_b = store.insert_list(vec![SlotValue::I64(7)])?;
    let object_a = store.insert_object(vec![ObjectField::clean(key, SlotValue::I64(7))])?;
    let object_b = store.insert_object(vec![ObjectField::clean(key, SlotValue::I64(7))])?;
    let blob_a = store.insert_blob(Vec::from(b"equal-content"))?;
    let blob_b = store.insert_blob(Vec::from(b"equal-content"))?;
    assert_ne!(list_a, list_b);
    assert_ne!(object_a, object_b);
    assert_ne!(blob_a, blob_b);
    assert_eq!(store.list(list_a)?, store.list(list_b)?);
    assert_eq!(store.object(object_a)?, store.object(object_b)?);
    assert_eq!(store.blob(blob_a)?, store.blob(blob_b)?);
    let members = [
        SlotValue::List(list_a),
        SlotValue::Object(object_a),
        SlotValue::Blob(blob_a),
        SlotValue::Null,
    ];
    let haystack = SlotValue::List(store.insert_list(members.to_vec())?);
    for (needle, expected) in [
        (SlotValue::List(list_a), true),
        (SlotValue::List(list_b), false),
        (SlotValue::Object(object_a), true),
        (SlotValue::Object(object_b), false),
        (SlotValue::Blob(blob_a), true),
        (SlotValue::Blob(blob_b), false),
        (SlotValue::Null, true),
    ] {
        let mut frame = new_run_frame(RunId::new(1), &workflow)?;
        for (input, value) in workflow.input_slots().iter().zip([haystack, needle]) {
            frame.write_slot_with_taint(input.slot, value, Taint::DerivedFromSecret)?;
        }
        let (program, constants) =
            vb_expr::compile_expr("contains($a, $b)", &|reference: &str| match reference {
                "$a" => Some(vb_core::SlotIdx::new(0)),
                "$b" => Some(vb_core::SlotIdx::new(1)),
                _ => None,
            })?;
        let before = store.clone();
        assert_eq!(
            vb_expr::eval_expr_program_with_store(
                &program,
                &[Some(haystack), Some(needle)],
                &constants,
                &mut store
            )?,
            SlotValue::Bool(expected)
        );
        assert_eq!(store, before);
        let mut replay = frame.clone();
        assert_eq!(
            vb_core::engine::step_once(&workflow, &mut frame, &mut store)?,
            vb_core::EngineSignal::Continue
        );
        assert_eq!(*frame.read_slot(output)?, SlotValue::Bool(expected));
        assert_eq!(frame.read_taint(output)?, Taint::DerivedFromSecret);
        assert_eq!(store, before);
        vb_core::replay::replay_step(node, &mut replay, &mut store, &workflow)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(*replay.read_slot(output)?, SlotValue::Bool(expected));
        assert_eq!(replay.read_taint(output)?, Taint::DerivedFromSecret);
        assert_eq!(store, before);
    }
    Ok(())
}

#[test]
fn contains_routes_preserve_exact_type_and_missing_operand_precedence() -> TestResult {
    use vb_core::replay::ReplayError;
    use vb_core::{EngineError, SlotIdx};
    use vb_expr::ExprError;
    let source = b"version: velvet-ballistics/v1\nname: contains_errors\nwhen: {manual: {}}\ninputs: {haystack: any, needle: any}\nsteps:\n  - id: calculate\n    set: {output: answer, value: 'contains($input.haystack, $input.needle)'}\n  - id: done\n    finish: {result: answer}\n";
    let workflow = vb_compile::compile_workflow(source)?;
    let node = workflow
        .node(StepIdx::ZERO)
        .ok_or("missing expression node")?;
    let vb_core::CompiledNodeKind::EvalExpr { expr } = node.kind else {
        return Err("expected expression node".into());
    };
    let output = node.output.slot().ok_or("missing output slot")?;
    let inputs = workflow.input_slots();
    let needle_slot = inputs.get(1).ok_or("missing second input")?.slot;
    let (program, constants) =
        vb_expr::compile_expr("contains($a, $b)", &|reference: &str| match reference {
            "$a" => Some(SlotIdx::ZERO),
            "$b" => Some(SlotIdx::new(1)),
            _ => None,
        })?;
    let mut store = ValueStore::new();
    let text = SlotValue::Symbol(store.insert_symbol("valid text")?);
    for (left, right, found) in [
        (SlotValue::Bool(true), SlotValue::Null, "boolean"),
        (text, SlotValue::I64(17), "number"),
        (SlotValue::Null, text, "null"),
    ] {
        let mut frame = new_run_frame(RunId::new(3), &workflow)?;
        for (input, value) in inputs.iter().zip([left, right]) {
            frame.write_slot_with_taint(input.slot, value, Taint::Secret)?;
        }
        frame.write_slot(output, SlotValue::I64(99))?;
        let frame_before = frame.checkpoint_v1();
        let store_before = store.clone();
        assert_eq!(
            vb_expr::eval_expr_program_with_store(
                &program,
                &[Some(left), Some(right)],
                &constants,
                &mut store
            ),
            Err(ExprError::TypeMismatch {
                expected: "text".into(),
                found: found.into()
            })
        );
        assert_eq!(
            vb_core::engine::eval_expr_with_store(&workflow, &frame, &mut store, expr),
            Err(EngineError::TypeMismatch {
                expected: "text",
                found
            })
        );
        assert_eq!(
            vb_core::replay::replay_step(node, &mut frame, &mut store, &workflow).err(),
            Some(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO
            })
        );
        assert_eq!(frame.checkpoint_v1(), frame_before);
        assert_eq!(store, store_before);
    }
    // Missing second operand is detected while loading bytecode, before the
    // invalid first operand can reach the Contains type check.
    let mut frame = new_run_frame(RunId::new(4), &workflow)?;
    frame.write_slot(
        inputs.first().ok_or("missing first input")?.slot,
        SlotValue::Bool(false),
    )?;
    frame.write_slot(output, SlotValue::I64(101))?;
    let frame_before = frame.checkpoint_v1();
    let store_before = store.clone();
    assert_eq!(
        vb_expr::eval_expr_program_with_store(
            &program,
            &[Some(SlotValue::Bool(false)), None],
            &constants,
            &mut store
        ),
        Err(ExprError::StackUnderflow)
    );
    assert_eq!(
        vb_core::engine::eval_expr_with_store(&workflow, &frame, &mut store, expr),
        Err(EngineError::SlotUninitialized { slot: needle_slot })
    );
    assert_eq!(
        vb_core::replay::replay_step(node, &mut frame, &mut store, &workflow).err(),
        Some(ReplayError::SlotNotAvailable { slot: needle_slot })
    );
    assert_eq!(frame.checkpoint_v1(), frame_before);
    assert_eq!(store, store_before);
    Ok(())
}
