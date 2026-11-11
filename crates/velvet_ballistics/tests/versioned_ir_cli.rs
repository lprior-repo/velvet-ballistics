use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

mod support {
    use vb_core::CompiledWorkflow;
    use vb_core::workflow::{VersionedWorkflowParts, WorkflowIrV2};
    pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
    pub(super) const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: callers\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set:\n      output: answer\n      value: \"42\"\n  - id: done\n    finish:\n      result: \"$seed\"\n";
    pub(super) fn legacy() -> TestResult<CompiledWorkflow> {
        Ok(vb_compile::compile_workflow(SOURCE)?)
    }
    pub(super) fn bound(values: &[&str]) -> TestResult<CompiledWorkflow> {
        let mut parts = legacy()?.to_parts();
        parts.symbols_count = u32::try_from(values.len())?;
        let payloads = values.iter().map(|s| Box::<str>::from(*s)).collect();
        let mut ir = WorkflowIrV2 {
            wire_version: 2,
            parts,
            symbol_payloads: payloads,
        };
        ir.parts.digest =
            vb_compile::compute_compiled_ir_digest(&VersionedWorkflowParts::V2(ir.clone()))?;
        Ok(CompiledWorkflow::try_from_parts_with_symbols(
            ir.parts,
            ir.symbol_payloads,
        )?)
    }
}
use support::TestResult;

fn run(ir: &Path, input: &Path) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args([
            OsStr::new("run-compiled"),
            ir.as_os_str(),
            OsStr::new("--input-bin"),
            input.as_os_str(),
            OsStr::new("--durability"),
            OsStr::new("none"),
        ])
        .output()
}

fn run_bound(values: &[&str]) -> TestResult {
    let dir = tempfile::tempdir()?;
    let ir = dir.path().join("run.vbir");
    let input = dir.path().join("input.bin");
    let workflow = support::bound(values)?;
    let bytes = vb_compile::encode_compiled_ir(&workflow)?;
    assert!(bytes.starts_with(&[0x01, 0xff, 0x56, 0x42]));
    std::fs::write(&ir, &bytes)?;
    std::fs::write(&input, [])?;
    let output = run(&ir, &input)?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn versioned_cli_runs_some_empty() -> TestResult {
    run_bound(&[])
}

#[test]
fn versioned_cli_runs_ordered_bound_payload() -> TestResult {
    run_bound(&["key", "é", " "])
}

fn compile(source: &Path, out: &Path, emit: &str) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args([
            OsStr::new("compile"),
            source.as_os_str(),
            OsStr::new("--emit"),
            OsStr::new(emit),
            OsStr::new("--out"),
            out.as_os_str(),
        ])
        .output()
}

fn check_export(emit: &str, expected: &[u8]) -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source.yaml");
    let out = dir.path().join("output");
    std::fs::write(&source, support::SOURCE)?;
    let result = compile(&source, &out, emit)?;
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(out)?, expected);
    Ok(())
}

#[test]
fn versioned_cli_legacy_exports_remain_exact() -> TestResult {
    let parts = support::legacy()?.to_parts();
    let bytes = postcard::to_allocvec(&parts)?;
    check_export("ir", &bytes)?;
    check_export("postcard", &bytes)?;
    check_export("yaml", serde_saphyr::to_string(&parts)?.as_bytes())
}

#[test]
fn versioned_cli_rejects_unsupported_inner_wire() -> TestResult {
    let dir = tempfile::tempdir()?;
    let ir = dir.path().join("bad.vbir");
    let input = dir.path().join("input.bin");
    let vb_core::workflow::VersionedWorkflowParts::V2(mut body) =
        support::bound(&[])?.to_versioned_parts()
    else {
        return Err("bound fixture lost V2".into());
    };
    body.wire_version = 3;
    let mut bytes = vec![0x01, 0xff, 0x56, 0x42];
    bytes.extend_from_slice(&postcard::to_allocvec(&body)?);
    assert!(matches!(
        vb_storage::decode_compiled_ir(&bytes, None),
        Err(vb_storage::JournalError::ArtifactMalformed)
    ));
    std::fs::write(&ir, bytes)?;
    std::fs::write(&input, [])?;
    assert_eq!(run(&ir, &input)?.status.code(), Some(3));
    Ok(())
}
