use vb_compile::compile_workflow;
use vb_core::workflow::VersionedWorkflowParts;
use vb_core::{ConstIdx, ConstValue, ExprOp, ExprProgram, SlotIdx, SymbolId, WorkflowParts};

#[path = "support/source_text_binding_cases.rs"]
mod binding_cases;

fn literal_has_source() -> &'static str {
    "version: velvet-ballistics/v1\nname: literal_has\nwhen:\n  manual: {}\ninputs:\n  obj: any\nsteps:\n  - id: check\n    set: { output: result, value: 'has($input.obj, \"key\")' }\n  - id: done\n    finish: { result: result }\n"
}

#[test]
fn public_literal_has_binds_payload_and_receiver_first_bytecode() -> Result<(), String> {
    let workflow = compile_workflow(literal_has_source().as_bytes())
        .map_err(|error| format!("public literal compilation failed: {error:?}"))?;
    assert_eq!(
        workflow.symbol_payloads(),
        Some([Box::<str>::from("key")].as_slice())
    );
    assert!(matches!(
        workflow.to_versioned_parts(),
        VersionedWorkflowParts::V2(_)
    ));
    assert_literal_program(workflow.to_parts())
}

fn assert_literal_program(parts: WorkflowParts) -> Result<(), String> {
    assert_eq!(
        parts.constants.as_ref(),
        &[ConstValue::Symbol(SymbolId::new(0))]
    );
    assert_eq!(
        parts.expressions.as_ref(),
        &[ExprProgram {
            ops: Box::from([
                ExprOp::LoadSlot(SlotIdx::new(1)),
                ExprOp::LoadConst(ConstIdx::new(0)),
                ExprOp::Has,
            ]),
            max_stack: 2,
        }]
    );
    Ok(())
}
