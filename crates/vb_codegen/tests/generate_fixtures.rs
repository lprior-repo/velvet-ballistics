//! Generate trybuild fixtures from compiled workflows.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compile-fail")
        .join("pass")
}

#[test]
fn generate_minimal_workflow_fixture() {
    let fixture_path = fixtures_dir().join("minimal_workflow.rs");
    
    // Create a minimal workflow
    let ops = vec![vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(0))];
    let expr = vb_core::ExprProgram::try_from_ops(ops.into_boxed_slice())
        .expect("expression must compile");
    
    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("test_codegen"),
        digest: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: vec![
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(0),
                output: Some(vb_core::SlotIdx::new(0)),
                next: Some(vb_core::StepIdx::new(1)),
                kind: vb_core::CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
            },
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(1),
                output: None,
                next: None,
                kind: vb_core::CompiledNodeKind::Finish {
                    result: vb_core::SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![vb_core::ConstValue::I64(42)].into_boxed_slice(),
        slot_count: 1,
        entry: vb_core::StepIdx::new(0),
        resource_contract: vb_core::ResourceContract::DEFAULT,
    };
    
    let workflow = vb_core::CompiledWorkflow::try_from_parts(parts)
        .expect("workflow must compile");
    
    // Emit generated Rust
    let source = vb_codegen::emit_rust_workflow(&workflow)
        .expect("codegen must succeed for minimal workflow");
    
    // Append a main function so trybuild can compile it as a binary
    let mut source = source;
    source.push_str("\nfn main() {\n");
    source.push_str("    let slots = [None; WORKFLOW_SLOT_COUNT];\n");
    source.push_str("    let _result = drive(slots);\n");
    source.push_str("}\n");
    
    std::fs::create_dir_all(fixtures_dir()).expect("must create fixtures dir");
    std::fs::write(&fixture_path, source).expect("must write fixture");
    
    println!("Generated fixture: {}", fixture_path.display());
}
