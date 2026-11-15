#![forbid(unsafe_code)]

use vb_core::{ConstIdx, ConstValue, ExprOp, SymbolId};

#[test]
fn actual_source_metadata_retains_ordered_empty_multibyte_and_reused_literals()
-> Result<(), Box<dyn std::error::Error>> {
    let workflow = vb_compile::compile_workflow(
        r#"version: velvet-ballistics/v1
name: source_payload_order
when: {manual: {}}
vars:
  first: "λ"
  empty: ""
  repeated: "λ"
  object: {κ: "λ"}
steps:
  - id: literal
    set: {output: echoed, value: '"λ"'}
  - id: check
    set: {output: present, value: 'has($vars.object, "κ")'}
  - id: done
    finish: {result: present}
"#
        .as_bytes(),
    )?;
    let metadata = workflow
        .source_metadata()
        .ok_or("source metadata missing")?;
    let symbols: Vec<_> = metadata.symbols().iter().map(Box::as_ref).collect();
    assert_eq!(symbols, ["λ", "", "κ"]);
    assert_eq!(metadata.symbols_count(), 3);
    assert_eq!(metadata.symbol_bytes(), 4);
    assert!(metadata.secret_requirements().is_empty());
    let parts = workflow.to_parts();
    assert_eq!(
        parts.constants.as_ref(),
        &[
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(2)),
        ]
    );
    let receiver = parts
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            vb_core::CompiledNodeKind::BuildObject { .. } => node.output.slot(),
            _ => None,
        })
        .ok_or("missing declaration object")?;
    let expressions: Vec<_> = parts
        .expressions
        .iter()
        .map(|program| program.ops.as_ref())
        .collect();
    assert_eq!(
        expressions,
        [
            &[ExprOp::LoadConst(ConstIdx::new(4))][..],
            &[
                ExprOp::LoadSlot(receiver),
                ExprOp::LoadConst(ConstIdx::new(5)),
                ExprOp::Has
            ][..],
        ]
    );
    Ok(())
}
