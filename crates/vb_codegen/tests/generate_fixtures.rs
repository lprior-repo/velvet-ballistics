#![forbid(unsafe_code)]
#![cfg(not(miri))]
//! Generate trybuild fixtures from compiled workflows.

use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compile-fail")
        .join("pass")
}

#[test]
fn generate_minimal_workflow_fixture() -> Result<(), String> {
    let fixture_path = fixtures_dir().join("minimal_workflow.rs");

    // Create a minimal workflow
    let ops = vec![vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(0))];
    let expr =
        vb_core::ExprProgram::try_from_ops(ops.into_boxed_slice()).map_err(|e| e.to_string())?;

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("test_codegen"),
        digest: vb_core::WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: vec![
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(0),
                output: Some(vb_core::SlotIdx::new(0)),
                next: Some(vb_core::StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: vb_core::CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
            },
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
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
        symbols_count: 0,
        entry: vb_core::StepIdx::new(0),
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let workflow = vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;

    // Emit generated Rust
    let source = vb_codegen::emit_rust_workflow(&workflow).map_err(|e| e.to_string())?;

    // Append a main function so trybuild can compile it as a binary
    let mut source = source;
    source.push_str("\nfn main() {\n");
    source.push_str("    let slots = [None; WORKFLOW_SLOT_COUNT];\n");
    source.push_str("    if let Err(error) = drive(slots) {\n");
    source.push_str("        eprintln!(\"{error:?}\");\n");
    source.push_str("        std::process::exit(1);\n");
    source.push_str("    }\n");
    source.push_str("}\n");

    std::fs::create_dir_all(fixtures_dir()).map_err(|e| e.to_string())?;
    std::fs::write(&fixture_path, source).map_err(|e| e.to_string())?;

    println!("Generated fixture: {}", fixture_path.display());
    Ok(())
}
