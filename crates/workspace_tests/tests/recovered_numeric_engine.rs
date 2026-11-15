#![forbid(unsafe_code)]

use vb_core::{
    EngineError, EngineSignal, FiniteF64, RunId, SlotValue, StepIdx, Taint, ValueStore,
    new_run_frame,
};

#[test]
fn numeric_engine_dispatch_preserves_operand_order_taint_and_exact_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (
            "/",
            SlotValue::I64(i64::MIN),
            SlotValue::I64(-1),
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "integer division overflow",
            }),
        ),
        (
            "-",
            SlotValue::I64(21),
            SlotValue::I64(3),
            Ok(SlotValue::I64(18)),
        ),
        (
            "-",
            SlotValue::I64(3),
            SlotValue::I64(21),
            Ok(SlotValue::I64(-18)),
        ),
        (
            "-",
            SlotValue::Bool(true),
            SlotValue::I64(3),
            Err(EngineError::TypeMismatch {
                expected: "number",
                found: "boolean",
            }),
        ),
        (
            "-",
            SlotValue::I64(3),
            SlotValue::Bool(false),
            Err(EngineError::TypeMismatch {
                expected: "number",
                found: "boolean",
            }),
        ),
        (
            "-",
            SlotValue::I64(6),
            SlotValue::F64(FiniteF64::new(2.5)?),
            Ok(SlotValue::F64(FiniteF64::new(3.5)?)),
        ),
        (
            "<",
            SlotValue::I64(6),
            SlotValue::F64(FiniteF64::new(2.5)?),
            Ok(SlotValue::Bool(false)),
        ),
        (
            "<",
            SlotValue::F64(FiniteF64::new(2.5)?),
            SlotValue::I64(6),
            Ok(SlotValue::Bool(true)),
        ),
    ];
    for (operator, left, right, expected) in cases {
        let source = format!(
            "version: velvet-ballistics/v1\nname: numeric_adapter\nwhen: {{manual: {{}}}}\ninputs: {{left: any, right: any}}\nsteps:\n  - id: calculate\n    set: {{output: answer, value: '$input.left {operator} $input.right'}}\n  - id: done\n    finish: {{result: answer}}\n"
        );
        let workflow = vb_compile::compile_workflow(source.as_bytes())?;
        let mut frame = new_run_frame(RunId::new(1), &workflow)?;
        for (input, value) in workflow.input_slots().iter().zip([left, right]) {
            frame.write_slot_with_taint(input.slot, value, Taint::Secret)?;
        }
        let output = workflow
            .node(StepIdx::ZERO)
            .and_then(|node| node.output.slot())
            .ok_or("missing output")?;
        frame.write_slot_with_taint(output, SlotValue::I64(99), Taint::Clean)?;
        let mut store = ValueStore::new();
        let before = store.clone();
        let actual = vb_core::engine::step_once(&workflow, &mut frame, &mut store);
        match expected {
            Ok(value) => {
                assert_eq!(actual?, EngineSignal::Continue);
                assert_eq!(
                    *frame.read_slot(output)?,
                    value,
                    "{left:?} {operator} {right:?}"
                );
                assert_eq!(frame.read_taint(output)?, Taint::Secret);
            }
            Err(error) => {
                assert_eq!(actual, Err(error), "{left:?} {operator} {right:?}");
                assert_eq!(*frame.read_slot(output)?, SlotValue::I64(99));
                assert_eq!(frame.read_taint(output)?, Taint::Clean);
            }
        }
        assert_eq!(store, before);
    }
    Ok(())
}
