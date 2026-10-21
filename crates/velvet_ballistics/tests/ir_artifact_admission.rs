#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::path::Path;
use std::process::Output;

use vb_core::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::value::SlotValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    ResourceContract, WorkflowParts,
};

fn finish_node(step: u16, result: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(step),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
    }
}

fn nop_node(step: u16, next: Option<u16>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(step),
        output: None,
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn base_parts(nodes: Box<[CompiledNode]>) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("handcrafted-admission"),
        digest: WorkflowDigest::from_bytes([0xA7; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    }
}

fn valid_input_payload(value: SlotValue) -> Vec<u8> {
    let values: Box<[SlotValue]> = Box::from([value]);
    let encoded = postcard::to_allocvec(&values);
    assert!(encoded.is_ok(), "test input payload encodes: {encoded:?}");
    encoded.unwrap_or_default()
}

fn write_bytes(path: &Path, bytes: &[u8]) {
    let written = std::fs::write(path, bytes);
    assert!(written.is_ok(), "test fixture write succeeds: {written:?}");
}

fn write_parts(path: &Path, parts: &WorkflowParts) {
    let encoded = postcard::to_allocvec(parts);
    assert!(encoded.is_ok(), "test WorkflowParts encodes: {encoded:?}");
    let payload = encoded.unwrap_or_default();
    write_bytes(path, &payload);
}

fn run_vb(args: &[&OsStr]) -> Output {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(args)
        .output();
    assert!(output.is_ok(), "vb test binary executes: {output:?}");
    match output {
        Ok(output) => output,
        Err(_) => std::process::abort(),
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn run_compiled(ir_path: &Path, input_path: &Path) -> Output {
    run_vb(&[
        OsStr::new("run-compiled"),
        ir_path.as_os_str(),
        OsStr::new("--input-bin"),
        input_path.as_os_str(),
        OsStr::new("--durability"),
        OsStr::new("none"),
    ])
}

fn run_yaml(workflow_path: &Path, input_path: &Path) -> Output {
    run_vb(&[
        OsStr::new("run"),
        workflow_path.as_os_str(),
        OsStr::new("--input-bin"),
        input_path.as_os_str(),
        OsStr::new("--durability"),
        OsStr::new("none"),
    ])
}

fn compile_postcard(workflow_path: &Path, ir_path: &Path) -> Output {
    run_vb(&[
        OsStr::new("compile"),
        workflow_path.as_os_str(),
        OsStr::new("--emit"),
        OsStr::new("postcard"),
        OsStr::new("--out"),
        ir_path.as_os_str(),
    ])
}

fn assert_run_compiled_rejects(parts: WorkflowParts, expected: &str) {
    let dir = must_tempdir();
    let ir_path = dir.path().join("malformed.vbir");
    let input_path = dir.path().join("input.bin");
    write_parts(&ir_path, &parts);
    write_bytes(&input_path, &[]);

    let output = run_compiled(&ir_path, &input_path);
    assert!(
        !output.status.success(),
        "malformed artifact should fail: stdout={} stderr={}",
        stdout(&output),
        stderr(&output)
    );
    let stderr = stderr(&output);
    assert!(
        stderr.contains("compiled IR validation error"),
        "stderr should identify compiled IR validation: {stderr}"
    );
    assert!(
        stderr.contains(expected),
        "stderr should contain {expected:?}: {stderr}"
    );
}

#[test]
fn run_compiled_accepts_valid_handcrafted_ir_artifact() {
    let dir = must_tempdir();
    let ir_path = dir.path().join("valid.vbir");
    let input_path = dir.path().join("input.bin");
    write_parts(&ir_path, &base_parts(Box::from([finish_node(0, 0)])));
    write_bytes(&input_path, &valid_input_payload(SlotValue::I64(42)));

    let output = run_compiled(&ir_path, &input_path);
    assert!(
        output.status.success(),
        "valid artifact should run: stdout={} stderr={}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("run completed"),
        "stdout should report completion: {}",
        stdout(&output)
    );
}

#[test]
fn run_compiled_for_each_corpus_artifact_reaches_runtime_semantics() {
    let dir = must_tempdir();
    let ir_path = dir.path().join("for_each.vbir");
    let input_path = dir.path().join("empty-input.bin");
    let workflow_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fuzz/corpus/vb_f04l_yaml_compiler_compile/for_each.yaml");
    write_bytes(&input_path, &[]);

    let compile = compile_postcard(&workflow_path, &ir_path);
    assert!(
        compile.status.success(),
        "for_each corpus workflow should compile: stdout={} stderr={}",
        stdout(&compile),
        stderr(&compile)
    );

    let yaml_run = run_yaml(&workflow_path, &input_path);
    let compiled_run = run_compiled(&ir_path, &input_path);

    assert_eq!(
        yaml_run.status.code(),
        compiled_run.status.code(),
        "run and run-compiled should agree after compilation: run stdout={} run stderr={} compiled stdout={} compiled stderr={}",
        stdout(&yaml_run),
        stderr(&yaml_run),
        stdout(&compiled_run),
        stderr(&compiled_run)
    );
    assert!(
        !stderr(&compiled_run).contains("compiled IR validation error"),
        "compiled artifact should reach runtime semantics, not IR validation: {}",
        stderr(&compiled_run)
    );
}

#[test]
fn run_compiled_rejects_non_postcard_ir_artifact() {
    let dir = must_tempdir();
    let ir_path = dir.path().join("not-postcard.vbir");
    let input_path = dir.path().join("input.bin");
    write_bytes(&ir_path, b"not-postcard");
    write_bytes(&input_path, &[]);

    let output = run_compiled(&ir_path, &input_path);
    assert!(
        !output.status.success(),
        "non-postcard artifact should fail: stdout={} stderr={}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("error deserializing compiled IR"),
        "stderr should identify postcard decode failure: {}",
        stderr(&output)
    );
}

#[test]
fn run_compiled_rejects_handcrafted_unreachable_node_ir() {
    let parts = base_parts(Box::from([finish_node(0, 0), finish_node(1, 0)]));

    assert_run_compiled_rejects(parts, "not reachable");
}

#[test]
fn run_compiled_rejects_handcrafted_backward_edge_ir() {
    let parts = base_parts(Box::from([nop_node(0, Some(1)), nop_node(1, Some(0))]));

    assert_run_compiled_rejects(parts, "backward edge");
}

#[test]
fn run_compiled_rejects_handcrafted_bad_constant_reference_ir() {
    let mut parts = base_parts(Box::from([
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        finish_node(1, 0),
    ]));
    parts.constants = Box::from([]);

    assert_run_compiled_rejects(parts, "constant");
}

#[test]
fn run_compiled_rejects_handcrafted_bad_accessor_reference_ir() {
    let expression =
        ExprProgram::try_from_ops(Box::from([ExprOp::LoadAccessor(AccessorIdx::new(0))]));
    assert!(
        expression.is_ok(),
        "single load-accessor expression is stack-valid: {expression:?}"
    );
    let expression = match expression {
        Ok(expression) => expression,
        Err(_) => std::process::abort(),
    };
    let mut parts = base_parts(Box::from([
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
        },
        finish_node(1, 0),
    ]));
    parts.expressions = Box::from([expression]);
    parts.accessors = Box::from([]);

    assert_run_compiled_rejects(parts, "accessor");
}

#[test]
fn run_compiled_rejects_handcrafted_bad_accessor_path_ir() {
    let mut parts = base_parts(Box::from([finish_node(0, 0)]));
    parts.accessors = Box::from([AccessorProgram {
        root: SlotIdx::ZERO,
        path: Box::from([PathSegment::Field(SymbolId::new(0))]),
    }]);
    parts.symbols_count = 0;

    assert_run_compiled_rejects(parts, "symbol");
}

fn must_tempdir() -> tempfile::TempDir {
    let temp_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ir-artifact-tmp");
    let created = std::fs::create_dir_all(&temp_root);
    assert!(
        created.is_ok(),
        "test temp root creation succeeds: {created:?}"
    );
    let dir = tempfile::Builder::new()
        .prefix("ir-artifact-")
        .tempdir_in(&temp_root);
    assert!(dir.is_ok(), "tempdir succeeds: {dir:?}");
    match dir {
        Ok(dir) => dir,
        Err(_) => std::process::abort(),
    }
}

#[test]
fn run_compiled_rejects_handcrafted_bad_build_object_symbol_ir() {
    let mut parts = base_parts(Box::from([
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::BuildObject {
                fields: Box::from([(SymbolId::new(0), SlotIdx::ZERO)]),
            },
        },
        finish_node(1, 0),
    ]));
    parts.symbols_count = 0;

    assert_run_compiled_rejects(parts, "symbol");
}
