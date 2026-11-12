#![forbid(unsafe_code)]
#![cfg(not(miri))]
//! CLI integration tests — truth serum adversarial audit as executable tests.
//!
//! These tests encode the exact scenarios from the manual truth-serum audit
//! so they run on every `cargo test` invocation.

use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::SlotValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

#[path = "../src/cli_postcard.rs"]
mod cli_postcard;

const CLI_WORKFLOW: &str = r"version: velvet-ballistics/v1
name: cli_subprocess
when:
  manual: {}
steps:
  - id: build_result
    save:
      output: saved
      value: '42'
  - id: done
    finish:
      result: saved
";

fn input_slot_parts() -> WorkflowParts {
    let finish = vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    );
    WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("cli-input"),
        digest: WorkflowDigest::from_bytes([7u8; 32]),
        input_slots: Box::from([vb_core::workflow::CompiledInputSlot {
            slot: SlotIdx::ZERO,
            kind: vb_core::workflow::InputSlotKind::I64,
            constraint: vb_core::workflow::CompiledInputConstraint::unconstrained(),
        }]),
        nodes: Box::from([finish]),
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn minimal_parts(nodes: Box<[CompiledNode]>) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        input_slots: Box::from([]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    }
}

fn resolve_test_reference(reference: &str) -> Option<vb_core::ids::SlotIdx> {
    match reference {
        "$x" => Some(vb_core::ids::SlotIdx::new(0)),
        _ => None,
    }
}

fn forced_assertion_failure() -> bool {
    false
}

fn cli_tempdir() -> std::io::Result<tempfile::TempDir> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/cli-integration-tmp");
    std::fs::create_dir_all(&root)?;
    tempfile::Builder::new().prefix("vb-cli-").tempdir_in(root)
}

fn write_test_file(path: &std::path::Path, contents: &[u8]) -> bool {
    match std::fs::write(path, contents) {
        Ok(()) => true,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to write {}: {err}",
                path.display()
            );
            false
        }
    }
}

fn run_cli(args: &[&std::ffi::OsStr]) -> Option<std::process::Output> {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.args(args);

    match command.output() {
        Ok(output) => Some(output),
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballistics: {err}"
            );
            None
        }
    }
}

fn output_stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn output_stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn first_stderr_line(output: &std::process::Output) -> String {
    let stderr = output_stderr(output);
    match stderr.lines().next() {
        Some(line) => line.into(),
        None => String::new(),
    }
}

fn assert_cli_success(output: &std::process::Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed: stdout={} stderr={}",
        output_stdout(output),
        output_stderr(output)
    );
}

fn assert_cli_failure_contains(output: &std::process::Output, command: &str, diagnostic: &str) {
    assert!(
        !output.status.success(),
        "{command} should fail: stdout={} stderr={}",
        output_stdout(output),
        output_stderr(output)
    );
    let stderr = output_stderr(output);
    assert!(
        stderr.contains(diagnostic),
        "{command} stderr should contain {diagnostic:?}: {stderr}"
    );
}

include!("cli_integration/part_a.rs");
include!("cli_integration/part_b.rs");
include!("cli_integration/part_c.rs");
include!("cli_integration/part_d.rs");
