#![forbid(unsafe_code)]

use vb_core::{
    CapabilitySet, FiniteF64, RunId, SlotValue as V, StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_expr::lexer::BinaryOp;
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

#[test]
fn numeric_coercion_agrees_between_yaml_runtime_and_expression_evaluator()
-> Result<(), Box<dyn std::error::Error>> {
    let float = |value| FiniteF64::new(value).map(V::F64);
    for (left, right) in [
        (V::I64(6), float(2.5)?),
        (float(6.0)?, V::I64(2)),
        (float(6.0)?, float(2.5)?),
    ] {
        for (op, text) in [
            (BinaryOp::Add, "+"),
            (BinaryOp::Sub, "-"),
            (BinaryOp::Mul, "*"),
            (BinaryOp::Div, "/"),
            (BinaryOp::Gt, ">"),
            (BinaryOp::Gte, ">="),
            (BinaryOp::Lt, "<"),
            (BinaryOp::Lte, "<="),
        ] {
            let expected = vb_expr::eval_binary_op(op, left, right)?;
            let source = format!(
                "version: velvet-ballistics/v1\nname: numeric_parity\nwhen: {{manual: {{}}}}\ninputs: {{left: any, right: any}}\nsteps:\n  - id: calculate\n    set: {{output: answer, value: '$input.left {text} $input.right'}}\n  - id: done\n    finish: {{result: answer}}\n"
            );
            let workflow = vb_compile::compile_workflow(source.as_bytes())?;
            let original = workflow.to_parts();
            let mut frame = new_run_frame(RunId::new(1), &workflow)?;
            for (input, value) in workflow.input_slots().iter().zip([left, right]) {
                frame.write_slot_with_taint(input.slot, value, Taint::DerivedFromSecret)?;
            }
            let mut replay = frame.clone();
            let mut store = ValueStore::new();
            let mut collect = CollectStates::new();
            let mut evidence = EvidenceCollector::new();
            let signal = drive_deterministic_full(
                &workflow,
                &mut frame,
                &mut StepBudget::new(8),
                &mut store,
                &[],
                RetryPolicy::NEVER,
                &mut evidence,
                &mut collect,
                &CapabilitySet::empty(),
                1,
            )?;
            assert_eq!(
                signal,
                RuntimeSignal::Finished(expected),
                "{left:?} {text} {right:?}"
            );
            let output = workflow
                .node(vb_core::StepIdx::ZERO)
                .and_then(|node| node.output.slot())
                .ok_or("missing output")?;
            assert_eq!(frame.read_taint(output)?, Taint::DerivedFromSecret);
            let node = workflow
                .node(vb_core::StepIdx::ZERO)
                .ok_or("missing expression node")?;
            let mut replay_store = ValueStore::new();
            vb_core::replay::replay_step(node, &mut replay, &mut replay_store, &workflow)
                .map_err(|error| format!("replay failed: {error:?}"))?;
            assert_eq!(*replay.read_slot(output)?, expected);
            assert_eq!(replay.read_taint(output)?, Taint::DerivedFromSecret);
            assert_eq!(workflow.to_parts(), original);
        }
    }
    Ok(())
}

#[test]
fn integer_promotion_rounds_at_ieee754_boundaries() {
    for (integer, expected) in [
        (0, 0.0),
        (-1, -1.0),
        (i64::MIN, -9_223_372_036_854_775_808.0),
        (i64::MAX, 9_223_372_036_854_775_808.0),
        (9_007_199_254_740_993, 9_007_199_254_740_992.0),
    ] {
        assert_eq!(FiniteF64::from_i64(integer).get(), expected);
    }
}
