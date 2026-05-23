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

const CLI_WORKFLOW: &str = r"version: velvet-ballastics/v1
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
    let finish = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    WorkflowParts {
        name: Box::from("cli-input"),
        digest: WorkflowDigest::from_bytes([7u8; 32]),
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
        name: Box::from("test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
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
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_velvet-ballastics"));
    command.args(args);

    match command.output() {
        Ok(output) => Some(output),
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics: {err}"
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

#[test]
fn cli_status_default_succeeds() {
    let output = run_cli(&[std::ffi::OsStr::new("status")]);
    if let Some(output) = output {
        assert_cli_success(&output, "status");
        assert!(output_stdout(&output).contains("status: running"));
    }
}

#[test]
fn cli_status_json_succeeds() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--json"),
    ]);
    if let Some(output) = output {
        assert_cli_success(&output, "status --json");
        assert!(output_stdout(&output).contains("\"status\": \"running\""));
    }
}

#[test]
fn cli_status_jsonl_succeeds() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--jsonl"),
    ]);
    if let Some(output) = output {
        assert_cli_success(&output, "status --jsonl");
        assert!(output_stdout(&output).contains("\"status\":\"running\""));
    }
}

#[test]
fn cli_system_status_json_reports_degraded_when_no_backend_is_attached() {
    let output = run_cli(&[
        std::ffi::OsStr::new("system"),
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--server"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--json"),
    ]);
    let output = match output {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "system status --json");
    assert_eq!(
        output_stderr(&output),
        "",
        "system status success must not write stderr"
    );
    let stdout = output_stdout(&output);
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "system status JSON did not parse: {error}; stdout={stdout}"
            );
            return;
        }
    };
    let status_value = match packet.get("status") {
        Some(value) => value.clone(),
        None => {
            assert!(forced_assertion_failure(), "missing status field: {stdout}");
            return;
        }
    };
    let status: vb_ui_model::system::SystemStatusView = match serde_json::from_value(status_value) {
        Ok(status) => status,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "status field is not UI-model compatible: {error}; stdout={stdout}"
            );
            return;
        }
    };

    assert_eq!(packet.get("kind"), Some(&serde_json::json!("SystemStatus")));
    assert_eq!(packet.get("profile"), Some(&serde_json::json!("quick")));
    assert_eq!(packet.get("server"), Some(&serde_json::json!("none")));
    assert_eq!(packet.get("connected"), Some(&serde_json::json!(false)));
    assert_eq!(
        status.storage_health,
        vb_ui_model::system::StorageHealth::Degraded
    );
    assert!(!status.journal_batch_healthy);
    assert!(!status.blob_store_ok);
    assert!(!status.index_healthy);
    assert_eq!(status.writer_queue_depth, 0);
    assert_eq!(status.active_run_count, 0);
}

#[test]
fn cli_system_status_yaml_reports_degraded_no_backend() {
    let output = run_cli(&[
        std::ffi::OsStr::new("system"),
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]);
    let output = match output {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "system status --emit yaml");
    let stdout = output_stdout(&output);
    let packet: serde_json::Value = match serde_saphyr::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "system status --emit yaml did not parse as YAML: {error}; stdout={stdout}"
            );
            return;
        }
    };
    let status_value = match packet.get("status") {
        Some(value) => value.clone(),
        None => {
            assert!(forced_assertion_failure(), "missing status field: {stdout}");
            return;
        }
    };
    let status: vb_ui_model::system::SystemStatusView = match serde_json::from_value(status_value) {
        Ok(status) => status,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "status field is not UI-model compatible: {error}; stdout={stdout}"
            );
            return;
        }
    };

    assert_eq!(
        packet.get("schema_version"),
        Some(&serde_json::json!("velvet-ballastics/cli-output/v1"))
    );
    assert_eq!(packet.get("kind"), Some(&serde_json::json!("SystemStatus")));
    assert_eq!(packet.get("profile"), Some(&serde_json::json!("standard")));
    assert_eq!(packet.get("server"), Some(&serde_json::json!("none")));
    assert_eq!(packet.get("connected"), Some(&serde_json::json!(false)));
    assert_eq!(packet.get("reason"), Some(&serde_json::json!("no-backend")));
    assert_eq!(
        status.storage_health,
        vb_ui_model::system::StorageHealth::Degraded
    );
    assert_eq!(status.writer_queue_depth, 0);
    assert!(!status.journal_batch_healthy);
    assert_eq!(status.snapshot_seq, None);
    assert!(!status.blob_store_ok);
    assert!(!status.index_healthy);
    assert_eq!(status.uptime_seconds, 0);
    assert_eq!(status.active_run_count, 0);
}

#[test]
fn cli_system_status_rejects_unknown_server_mode() {
    let output = run_cli(&[
        std::ffi::OsStr::new("system"),
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--server"),
        std::ffi::OsStr::new("remote"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "system status --server remote",
            "unknown server mode: remote",
        );
    }
}

#[test]
fn cli_system_status_rejects_unprobed_server_modes() {
    ["strict", "journaled"].iter().for_each(|mode| {
        let output = run_cli(&[
            std::ffi::OsStr::new("system"),
            std::ffi::OsStr::new("status"),
            std::ffi::OsStr::new("--server"),
            std::ffi::OsStr::new(*mode),
        ]);
        if let Some(output) = output {
            assert_cli_failure_contains(
                &output,
                "system status --server",
                "strict and journaled require a backend probe that is not implemented",
            );
        }
    });
}

#[test]
fn cli_status_rejects_missing_queue_depth_value() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--queue-depth"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "status --queue-depth",
            "missing argument: --queue-depth",
        );
    }
}

#[test]
fn cli_status_rejects_missing_active_runs_value() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--active-runs"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "status --active-runs",
            "missing argument: --active-runs",
        );
    }
}

#[test]
fn cli_status_rejects_missing_trace_dropped_value() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--trace-dropped"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "status --trace-dropped",
            "missing argument: --trace-dropped",
        );
    }
}

#[test]
fn cli_status_rejects_unknown_flag() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--bogus"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "status --bogus",
            "invalid status argument: unknown flag --bogus",
        );
    }
}

#[test]
fn cli_status_rejects_extra_positional_argument() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("extra"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "status extra",
            "invalid status argument: unexpected positional argument extra",
        );
    }
}

#[test]
fn cli_status_rejects_nonnumeric_known_flag() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--queue-depth"),
        std::ffi::OsStr::new("many"),
    ]);
    if let Some(output) = output {
        assert_cli_failure_contains(
            &output,
            "status --queue-depth many",
            "invalid status argument: --queue-depth must be a usize",
        );
    }
}

#[test]
fn cli_action_list_table_output_has_exact_fields() {
    let output = run_cli(&[std::ffi::OsStr::new("action"), std::ffi::OsStr::new("list")]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list"
            );
            return;
        }
    };

    assert_cli_success(&output, "action list");
    assert_eq!(
        output_stdout(&output),
        "id\tidempotency\tretry_safety\tside_effect\tinput_slots\toutput_slots\ttimeout_ms\n1\tdeterministic_pure\tsafe\tnone\t1\t1\t1000\n2\tidempotent_external\tkey_required\twrites\t2\t1\t5000\n3\tat_least_once_external\tunsafe\tsends\t1\t0\t10000\n"
    );
    assert_eq!(output_stderr(&output), "");
}

#[test]
fn cli_action_list_json_output_has_exact_actions() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--json"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --json"
            );
            return;
        }
    };

    assert_cli_success(&output, "action list --json");
    let parsed = match serde_json::from_str::<serde_json::Value>(&output_stdout(&output)) {
        Ok(value) => value,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "action list JSON should parse: {error}; stdout={}",
                output_stdout(&output)
            );
            return;
        }
    };
    assert_eq!(
        parsed,
        serde_json::json!({
            "success": true,
            "actions": [
                {"id": 1, "idempotency": "deterministic_pure", "retry_safety": "safe", "side_effect": "none", "input_slot_count": 1, "output_slot_count": 1, "timeout_ms": 1000},
                {"id": 2, "idempotency": "idempotent_external", "retry_safety": "key_required", "side_effect": "writes", "input_slot_count": 2, "output_slot_count": 1, "timeout_ms": 5000},
                {"id": 3, "idempotency": "at_least_once_external", "retry_safety": "unsafe", "side_effect": "sends", "input_slot_count": 1, "output_slot_count": 0, "timeout_ms": 10000}
            ]
        })
    );
}

#[test]
fn cli_action_list_empty_registry_reports_no_actions() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--registry"),
        std::ffi::OsStr::new("empty"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --registry empty"
            );
            return;
        }
    };

    assert_cli_success(&output, "action list --registry empty");
    assert_eq!(output_stdout(&output), "no registered actions\n");
}

#[test]
fn cli_action_list_uninitialized_registry_fails() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--registry"),
        std::ffi::OsStr::new("uninitialized"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --registry uninitialized"
            );
            return;
        }
    };

    assert!(
        !output.status.success(),
        "uninitialized registry should fail"
    );
    assert_eq!(output_stdout(&output), "");
    assert_eq!(
        output_stderr(&output),
        "action registry is not initialized\n"
    );
}

#[test]
fn cli_action_list_missing_registry_value_fails_with_action_args_diagnostic() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--registry"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --registry (missing value)"
            );
            return;
        }
    };

    assert!(
        !output.status.success(),
        "missing registry value should fail"
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert_eq!(
        first_stderr_line(&output),
        "missing action-args value for --registry (expected: registered, empty, uninitialized)"
    );
}

#[test]
fn cli_action_list_unknown_flag_fails_with_exact_diagnostic() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--bogus"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --bogus"
            );
            return;
        }
    };

    assert!(!output.status.success(), "unknown flag should fail");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert_eq!(
        first_stderr_line(&output),
        "unknown action list flag: --bogus"
    );
}

#[test]
fn cli_action_list_unknown_registry_fails_with_exact_diagnostic() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--registry"),
        std::ffi::OsStr::new("bogus"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --registry bogus"
            );
            return;
        }
    };

    assert!(!output.status.success(), "unknown registry should fail");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert_eq!(
        first_stderr_line(&output),
        "unknown action registry: bogus (expected: registered, empty, uninitialized)"
    );
}

#[test]
fn cli_action_list_trailing_argument_fails() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("junk"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list junk"
            );
            return;
        }
    };

    assert!(!output.status.success(), "trailing arg should fail");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert_eq!(
        first_stderr_line(&output),
        "unexpected action list argument: junk"
    );
}

#[test]
fn cli_action_list_jsonl_output_has_exact_lines() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("list"),
        std::ffi::OsStr::new("--jsonl"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action list --jsonl"
            );
            return;
        }
    };

    assert_cli_success(&output, "action list --jsonl");
    let stdout = output_stdout(&output);
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "JSONL output should be exactly one line");

    let first_line = lines.first().copied().unwrap_or_default();
    let parsed = match serde_json::from_str::<serde_json::Value>(first_line) {
        Ok(value) => value,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "action list JSONL should parse: {error}; line={}",
                first_line
            );
            return;
        }
    };
    assert_eq!(parsed.get("success"), Some(&serde_json::json!(true)));
    let actions = match parsed.get("actions") {
        Some(serde_json::Value::Array(actions)) => actions,
        other => {
            assert!(
                forced_assertion_failure(),
                "actions should be an array: {other:?}"
            );
            return;
        }
    };
    assert_eq!(
        actions.len(),
        3,
        "registered registry should have 3 actions"
    );

    // Verify first action structure
    let first = actions.first().unwrap_or(&serde_json::Value::Null);
    assert_eq!(first.get("id"), Some(&serde_json::json!(1)));
    assert_eq!(
        first.get("idempotency"),
        Some(&serde_json::json!("deterministic_pure"))
    );
    assert_eq!(first.get("retry_safety"), Some(&serde_json::json!("safe")));
    assert_eq!(first.get("side_effect"), Some(&serde_json::json!("none")));
    assert_eq!(first.get("input_slot_count"), Some(&serde_json::json!(1)));
    assert_eq!(first.get("output_slot_count"), Some(&serde_json::json!(1)));
    assert_eq!(first.get("timeout_ms"), Some(&serde_json::json!(1000)));
}

#[test]
fn cli_action_inspect_text_output_has_contract_details() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("2"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action inspect 2"
            );
            return;
        }
    };

    assert_cli_success(&output, "action inspect 2");
    let stdout = output_stdout(&output);
    assert!(stdout.contains("action 2"));
    assert!(stdout.contains("idempotency: idempotent_external"));
    assert!(stdout.contains("retry_safety: key_required"));
    assert!(stdout.contains("failure_codes: rejected,timeout,rate_limited"));
    assert!(stdout.contains("example_input_schema:"));
}

#[test]
fn cli_action_inspect_json_output_has_full_contract() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("2"),
        std::ffi::OsStr::new("--json"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action inspect 2 --json"
            );
            return;
        }
    };

    assert_cli_success(&output, "action inspect 2 --json");
    let parsed = match serde_json::from_str::<serde_json::Value>(&output_stdout(&output)) {
        Ok(value) => value,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "action inspect JSON should parse: {error}; stdout={}",
                output_stdout(&output)
            );
            return;
        }
    };
    assert_eq!(parsed.get("success"), Some(&serde_json::json!(true)));
    let action = parsed.get("action").unwrap_or(&serde_json::Value::Null);
    assert_eq!(action.get("id"), Some(&serde_json::json!(2)));
    assert_eq!(
        action.get("idempotency"),
        Some(&serde_json::json!("idempotent_external"))
    );
    assert_eq!(
        action.get("retry_safety"),
        Some(&serde_json::json!("key_required"))
    );
    assert_eq!(
        action.get("idempotency_rule"),
        Some(&serde_json::json!(
            "external retries require a stable idempotency key"
        ))
    );
    assert!(
        action
            .get("failure_codes")
            .is_some_and(serde_json::Value::is_array)
    );
}

#[test]
fn cli_action_inspect_unregistered_action_fails() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("99"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballastics CLI for action inspect 99"
            );
            return;
        }
    };

    assert_cli_failure_contains(&output, "action inspect 99", "action 99 is not registered");
}

// ---------------------------------------------------------------------------
// Phase 1: YAML parsing — vb_yaml
// ---------------------------------------------------------------------------

#[test]
fn yaml_parse_empty_source_returns_error() {
    let result = vb_yaml::parse_workflow_source("");
    assert_eq!(result, Err(vb_yaml::YamlError::EmptySource));
}

#[test]
fn yaml_parse_binary_bytes_returns_error() {
    let mut binary = [0u8; 5];
    binary[0] = 0xff;
    binary[1] = 0xfe;
    binary[2] = std::hint::black_box(0x00);
    binary[3] = 0x01;
    binary[4] = 0x80;
    let text = std::str::from_utf8(&binary);
    assert!(text.is_err(), "binary is not valid UTF-8");
}

#[test]
fn yaml_parse_missing_version_returns_error() {
    let yaml = "\
name: test
when:
  manual: {}
steps: []
";
    let result = vb_yaml::parse_workflow_source(yaml);
    let err = match result {
        Ok(_) => {
            assert!(forced_assertion_failure(), "missing version should fail");
            return;
        }
        Err(err) => err.to_string(),
    };
    assert!(
        err.contains("version"),
        "error should mention missing version: {err}"
    );
}

#[test]
fn yaml_parse_missing_name_returns_error() {
    let yaml = "\
version: \"velvet-ballastics/v1\"
when:
  manual: {}
steps: []
";
    let result = vb_yaml::parse_workflow_source(yaml);
    let err = match result {
        Ok(_) => {
            assert!(forced_assertion_failure(), "missing name should fail");
            return;
        }
        Err(err) => err.to_string(),
    };
    assert!(
        err.contains("name"),
        "error should mention missing name: {err}"
    );
}

#[test]
fn yaml_parse_valid_minimal_workflow() {
    let yaml = "\
version: \"velvet-ballastics/v1\"
name: test-workflow
when:
  manual: {}
steps:
  - id: start
    set:
      output: greeting
      value: \"hello\"
    then: finish
  - id: finish
    finish:
      result: \"done\"
";
    let result = vb_yaml::parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.name(), "test-workflow");
            assert_eq!(wf.steps().len(), 2);
        }
        Err(err) => assert!(
            forced_assertion_failure(),
            "should parse valid workflow: {err:?}"
        ),
    }
}

#[test]
fn yaml_parse_broken_yaml_returns_error() {
    let yaml = "{{{broken";
    let result = vb_yaml::parse_workflow_source(yaml);
    assert!(matches!(result, Err(vb_yaml::YamlError::ParseError { .. })));
}

#[test]
fn yaml_profile_rejects_anchors() {
    let yaml =
        "version: &velvet \"velvet-ballastics/v1\"\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    let result = vb_yaml::validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(vb_yaml::YamlError::AnchorAliasMerge)),
        "anchors should be rejected"
    );
}

#[test]
fn yaml_parse_step_missing_do_action_returns_error() {
    let yaml = "\
version: \"velvet-ballastics/v1\"
name: test
when:
  manual: {}
steps:
  - id: start
    do:
      input: greeting
";
    let result = vb_yaml::parse_workflow_source(yaml);
    let err = match result {
        Ok(_) => {
            assert!(forced_assertion_failure(), "missing do.action should fail");
            return;
        }
        Err(err) => err.to_string(),
    };
    assert!(
        err.contains("do.action"),
        "error should mention missing do.action: {err}"
    );
}

#[test]
fn yaml_parse_set_missing_output_returns_error() {
    let yaml = "\
version: \"velvet-ballastics/v1\"
name: test
when:
  manual: {}
steps:
  - id: start
    set:
      value: \"hello\"
";
    let result = vb_yaml::parse_workflow_source(yaml);
    let err = match result {
        Ok(_) => {
            assert!(forced_assertion_failure(), "missing set.output should fail");
            return;
        }
        Err(err) => err.to_string(),
    };
    assert!(
        err.contains("set.output"),
        "error should mention missing set.output: {err}"
    );
}

// ---------------------------------------------------------------------------
// Phase 2: Validation — vb_validate
// ---------------------------------------------------------------------------

#[test]
fn validate_schema_rejects_bad_version() {
    use vb_validate::schema::{FieldValue, WorkflowDoc};

    let doc = WorkflowDoc::from_pairs(vec![
        ("version".into(), FieldValue::String("bad-version".into())),
        ("name".into(), FieldValue::String("test".into())),
        (
            "trigger".into(),
            FieldValue::Mapping(vec![("type".into(), FieldValue::String("manual".into()))]),
        ),
        ("steps".into(), FieldValue::Sequence(vec![])),
    ]);
    let result = vb_validate::schema::validate_version(&doc);
    assert!(result.is_err(), "bad version string should fail validation");
}

// ---------------------------------------------------------------------------
// Phase 3: Expression engine — vb_expr
// ---------------------------------------------------------------------------

#[test]
fn expr_lex_and_parse_simple_addition() {
    match vb_expr::lexer::lex_expr("1 + 2") {
        Ok(tokens) => match vb_expr::parser::parse_expr(&tokens) {
            Ok(ast) => assert!(matches!(ast, vb_expr::parser::ExprAst::Binary { .. })),
            Err(err) => assert!(forced_assertion_failure(), "parse failed: {err:?}"),
        },
        Err(err) => assert!(forced_assertion_failure(), "lex failed: {err:?}"),
    }
}

#[test]
fn expr_bytecode_compile_and_eval() {
    let tokens = match vb_expr::lexer::lex_expr("1 + 2") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(forced_assertion_failure(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(forced_assertion_failure(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(forced_assertion_failure(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    match vb_expr::eval::eval_expr_program(&program, &[], &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::I64(3)),
        Err(err) => assert!(forced_assertion_failure(), "eval failed: {err:?}"),
    }
}

#[test]
fn expr_rejects_division_by_zero() {
    let tokens = match vb_expr::lexer::lex_expr("1 / 0") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(forced_assertion_failure(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(forced_assertion_failure(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(forced_assertion_failure(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    let result = vb_expr::eval::eval_expr_program(&program, &[], &const_vals);
    assert_eq!(result, Err(vb_expr::ExprError::DivisionByZero));
}

#[test]
fn expr_boolean_logic() {
    let tokens = match vb_expr::lexer::lex_expr("true and false") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(forced_assertion_failure(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(forced_assertion_failure(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(forced_assertion_failure(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    match vb_expr::eval::eval_expr_program(&program, &[], &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::Bool(false)),
        Err(err) => assert!(forced_assertion_failure(), "eval failed: {err:?}"),
    }
}

#[test]
fn expr_variable_reference() {
    let compiled = match vb_expr::bytecode::compile_expr("$x + 1", &resolve_test_reference) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(forced_assertion_failure(), "compile failed: {err:?}");
            return;
        }
    };
    let (program, constants) = compiled;
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    let slots: Vec<Option<SlotValue>> = vec![Some(SlotValue::I64(41))];
    match vb_expr::eval::eval_expr_program(&program, &slots, &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::I64(42)),
        Err(err) => assert!(forced_assertion_failure(), "eval failed: {err:?}"),
    }
}

// ---------------------------------------------------------------------------
// Phase 4: Core IR validation
// ---------------------------------------------------------------------------

#[test]
fn core_workflow_rejects_out_of_bounds_step() {
    let bad_node = CompiledNode {
        id: StepIdx::new(99),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = minimal_parts(Box::from([bad_node]));
    let result = vb_core::engine::validate_compiled_workflow(&parts);
    assert!(result.is_err(), "out-of-bounds step should fail");
}

#[test]
fn core_workflow_rejects_invalid_jump_target() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Jump {
            target: StepIdx::new(50),
        },
    };
    let parts = minimal_parts(Box::from([node]));
    let result = vb_core::engine::validate_transition_target(&parts);
    assert!(result.is_err(), "invalid jump target should fail");
}

// ---------------------------------------------------------------------------
// Phase 5: Compile pipeline
// ---------------------------------------------------------------------------

#[test]
fn compile_rejects_non_utf8_input() {
    let binary: &[u8] = &[0xff, 0xfe, 0x00];
    let result = vb_compile::compile_workflow(binary);
    let err = match result {
        Ok(compiled) => {
            assert!(
                forced_assertion_failure(),
                "binary input should fail compile: {compiled:?}"
            );
            return;
        }
        Err(err) => err,
    };
    assert_eq!(err.len(), 1);
    assert_eq!(
        err.first().map(std::string::ToString::to_string),
        Some(
            "YAML source must be UTF-8: invalid utf-8 sequence of 1 bytes from index 0".to_string()
        )
    );
}

#[test]
fn compile_rejects_empty_input() {
    let result = vb_compile::compile_workflow(b"");
    assert!(result.is_err(), "empty input should fail compile");
}

#[test]
fn compile_rejects_invalid_yaml() {
    let result = vb_compile::compile_workflow(b"{{{broken");
    assert!(result.is_err(), "broken YAML should fail compile");
}

// ---------------------------------------------------------------------------
// Phase 6: IPC frame encode/decode roundtrip
// ---------------------------------------------------------------------------

#[test]
fn ipc_frame_roundtrip() {
    let header =
        vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Health, 0, 0x1234_5678_9ABC_DEF0u64, 0);
    let encoded = match header.encode() {
        Ok(encoded) => encoded,
        Err(err) => {
            assert!(forced_assertion_failure(), "encode failed: {err:?}");
            return;
        }
    };
    let nonzero = match std::num::NonZeroUsize::new(4096) {
        Some(nonzero) => nonzero,
        None => {
            assert!(
                forced_assertion_failure(),
                "nonzero payload limit should be valid"
            );
            return;
        }
    };
    let max_payload = vb_ipc::MaxPayloadBytes::new(nonzero);
    match vb_ipc::IpcFrameHeader::decode(&encoded, max_payload) {
        Ok(decoded) => {
            assert_eq!(decoded.correlation, header.correlation);
            assert_eq!(decoded.command, vb_ipc::IpcCommand::Health);
            assert_eq!(decoded.payload_len, 0);
        }
        Err(err) => assert!(forced_assertion_failure(), "decode failed: {err:?}"),
    }
}

// ---------------------------------------------------------------------------
// Phase 7: Storage record encode/decode roundtrip
// ---------------------------------------------------------------------------

#[test]
fn storage_encode_decode_roundtrip() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestPayload {
        value: i64,
        label: String,
    }

    let payload = TestPayload {
        value: 42,
        label: "test".into(),
    };
    const MAGIC: u32 = vb_storage::MAGIC_JOURNAL_EVENT;
    let encoded = match vb_storage::encode_record(
        MAGIC,
        vb_storage::RecordKind::StepStarted,
        1,
        &payload,
        4096,
    ) {
        Ok(encoded) => encoded,
        Err(err) => {
            assert!(forced_assertion_failure(), "encode failed: {err:?}");
            return;
        }
    };
    assert!(encoded.len() > 10, "encoded record should have header");

    let decoded: Result<(vb_storage::RecordEnvelope, TestPayload), _> =
        vb_storage::decode_record(&encoded, MAGIC, 4096);
    match decoded {
        Ok((_envelope, decoded)) => assert_eq!(decoded, payload),
        Err(err) => assert!(forced_assertion_failure(), "decode failed: {err:?}"),
    }
}

#[test]
fn storage_corrupt_record_fails_decode() {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestPayload {
        value: i64,
    }

    let payload = TestPayload { value: 42 };
    const MAGIC: u32 = vb_storage::MAGIC_JOURNAL_EVENT;
    let mut encoded = match vb_storage::encode_record(
        MAGIC,
        vb_storage::RecordKind::StepStarted,
        1,
        &payload,
        4096,
    ) {
        Ok(encoded) => encoded,
        Err(err) => {
            assert!(forced_assertion_failure(), "encode failed: {err:?}");
            return;
        }
    };

    // Corrupt last byte
    if let Some(last) = encoded.last_mut() {
        *last = last.wrapping_add(1);
    }
    let result: Result<(vb_storage::RecordEnvelope, TestPayload), _> =
        vb_storage::decode_record(&encoded, MAGIC, 4096);
    assert!(result.is_err(), "corrupted record should fail decode");
}

// ---------------------------------------------------------------------------
// Phase 8: Runtime engine signal types
// ---------------------------------------------------------------------------

#[test]
fn runtime_signal_debug_format() {
    let sig = vb_core::engine::EngineSignal::Continue;
    let debug = format!("{sig:?}");
    assert!(debug.contains("Continue"));
}

#[test]
fn runtime_slot_value_copy_trait() {
    let a = SlotValue::I64(42);
    let b = a;
    assert_eq!(a, b, "SlotValue should be Copy");
}

// ---------------------------------------------------------------------------
// Phase 9: Codegen produces non-empty output
// ---------------------------------------------------------------------------

#[test]
fn codegen_emit_rust_produces_output() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = minimal_parts(Box::from([node]));
    let compiled = match vb_core::workflow::CompiledWorkflow::try_from_parts(parts) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "compile workflow failed: {err:?}"
            );
            return;
        }
    };
    let result = vb_codegen::emit_rust_workflow(&compiled);
    match result {
        Ok(output) => {
            assert!(!output.is_empty(), "codegen output should not be empty");
            assert!(output.contains("fn drive"), "should contain drive function");
        }
        Err(err) => assert!(
            forced_assertion_failure(),
            "codegen should succeed: {err:?}"
        ),
    }
}

#[test]
fn cli_run_journaled_then_events_and_inspect_read_temp_db() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("fjall-db");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run --durability journaled --db");
    let run_stdout = output_stdout(&run_output);
    assert!(
        run_stdout.contains("run completed"),
        "run stdout should report completion: {run_stdout}"
    );

    let events_output = match run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&events_output, "events 1 --db");
    let events_stdout = output_stdout(&events_output);
    assert!(
        events_stdout.contains("RunAccepted"),
        "events stdout should include RunAccepted: {events_stdout}"
    );
    assert!(
        events_stdout.contains("RunFinished"),
        "events stdout should include RunFinished: {events_stdout}"
    );
    assert!(
        events_stdout.contains("event(s) total"),
        "events stdout should include total count: {events_stdout}"
    );

    let inspect_output = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&inspect_output, "inspect 1 --db");
    let inspect_stdout = output_stdout(&inspect_output);
    assert!(
        inspect_stdout.contains("status=finished"),
        "inspect stdout should report finished run: {inspect_stdout}"
    );
}

#[test]
fn cli_ai_context_for_journaled_run_emits_compiled_ir_summary() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("fjall-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run --durability journaled --db");

    let context_output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&context_output, "ai-context 1 --json");
    let stdout = output_stdout(&context_output);
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "ai-context JSON parse failed: {err}; stdout={stdout}"
            );
            return;
        }
    };
    assert_eq!(
        packet.pointer("/kind"),
        Some(&serde_json::json!("AiContextPacket"))
    );
    assert_eq!(
        packet.pointer("/workflow/compiled_ir/available"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        packet.pointer("/workflow/compiled_ir/node_count"),
        Some(&serde_json::json!(2))
    );
    assert_eq!(
        packet.pointer("/workflow/source_included"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        packet.pointer("/workflow/compiled_ir/nodes/0/kind"),
        Some(&serde_json::json!("SetConst"))
    );
    assert_eq!(
        packet.pointer("/workflow/compiled_ir/nodes/1/kind"),
        Some(&serde_json::json!("Finish"))
    );
    assert_eq!(
        packet.pointer("/trace_ring_snapshot/available"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        packet.pointer("/trace_ring_snapshot/fabricated"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        packet.pointer("/trace_ring_snapshot/events"),
        Some(&serde_json::json!([]))
    );
    let suggestions = match packet
        .pointer("/suggested_next_cli_commands")
        .and_then(serde_json::Value::as_array)
    {
        Some(suggestions) => suggestions,
        None => {
            assert!(
                forced_assertion_failure(),
                "suggestions must be a JSON array: {packet}"
            );
            return;
        }
    };
    assert!(
        suggestions.iter().any(|value| value
            .as_str()
            .is_some_and(|command| command.contains("inspect 1 --db"))),
        "finished run should suggest inspect: {packet}"
    );
    assert!(
        suggestions.iter().any(|value| value
            .as_str()
            .is_some_and(|command| command.contains("events 1 --db"))),
        "finished run should suggest events: {packet}"
    );
    assert!(
        suggestions.iter().any(|value| value
            .as_str()
            .is_some_and(|command| command.contains("replay 1 --db"))),
        "finished run should suggest replay: {packet}"
    );
    let rendered = packet.to_string();
    assert!(
        !rendered.contains("version: velvet-ballastics/v1"),
        "source YAML must not be emitted: {rendered}"
    );
    assert!(
        !rendered.contains("static ActionContract records are not embedded"),
        "placeholder contract text must not be emitted: {rendered}"
    );
}

#[test]
fn cli_ai_context_reports_missing_and_invalid_run_ids() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let invalid_output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("not-a-run"),
        std::ffi::OsStr::new("--db"),
        dir.path().as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_eq!(invalid_output.status.code(), Some(2));
    let invalid_stderr = output_stderr(&invalid_output);
    assert!(
        invalid_stderr.contains("invalid run_id 'not-a-run'"),
        "invalid run id error missing: {invalid_stderr}"
    );

    let missing_output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("77"),
        std::ffi::OsStr::new("--db"),
        dir.path().as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_eq!(missing_output.status.code(), Some(2));
    let stderr = output_stderr(&missing_output);
    assert!(stderr.contains("RUN_NOT_FOUND"), "missing code: {stderr}");
    assert!(stderr.contains("77"), "missing run id: {stderr}");
}

#[test]
fn cli_run_maps_postcard_slot_values_from_input_bin() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.vbir");
    let input_path = dir.path().join("input.bin");

    let workflow_payload = match postcard::to_allocvec(&input_slot_parts()) {
        Ok(payload) => payload,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to encode workflow payload: {err}"
            );
            return;
        }
    };
    if !write_test_file(&workflow_path, &workflow_payload) {
        return;
    }
    let values: Box<[SlotValue]> = Box::from([SlotValue::I64(7)]);
    let payload = match postcard::to_allocvec(&values) {
        Ok(payload) => payload,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to encode input payload: {err}"
            );
            return;
        }
    };
    if !write_test_file(&input_path, &payload) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run-compiled"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run-compiled --durability none with input-bin");
    let run_stdout = output_stdout(&run_output);
    assert!(
        run_stdout.contains("run completed"),
        "run stdout should report completion: {run_stdout}"
    );
}

#[test]
fn cli_run_reports_exact_input_mapping_decode_failure() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, b"not-postcard") {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(!run_output.status.success(), "malformed input should fail");
    let stderr = output_stderr(&run_output);
    assert!(
        stderr.contains("INPUT_MAPPING_FAILED: input-bin decode failed"),
        "stderr should contain exact input mapping diagnostic: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Phase 10: Taint propagation via action ABI
// ---------------------------------------------------------------------------

#[test]
fn taint_secret_propagates_through_deterministic_action() {
    use vb_core::action::{Idempotency, propagate_action_taint};
    use vb_core::value::Taint;

    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Secret);
    assert_eq!(result, Taint::Secret, "Secret input should propagate");
}

#[test]
fn taint_clean_stays_clean_for_pure_actions() {
    use vb_core::action::{Idempotency, propagate_action_taint};
    use vb_core::value::Taint;

    let result = propagate_action_taint(Idempotency::DeterministicPure, Taint::Clean);
    assert_eq!(result, Taint::Clean, "Clean input stays clean");
}

#[test]
fn taint_derived_propagates() {
    use vb_core::action::{Idempotency, propagate_action_taint};
    use vb_core::value::Taint;

    let result = propagate_action_taint(Idempotency::IdempotentExternal, Taint::DerivedFromSecret);
    assert_eq!(
        result,
        Taint::DerivedFromSecret,
        "DerivedFromSecret propagates"
    );
}

// ---------------------------------------------------------------------------
// Phase 11: IPC command enum completeness
// ---------------------------------------------------------------------------

#[test]
fn ipc_all_commands_have_distinct_codes() {
    use std::collections::HashSet;
    let commands = [
        vb_ipc::IpcCommand::Health,
        vb_ipc::IpcCommand::Shutdown,
        vb_ipc::IpcCommand::SubmitRun,
        vb_ipc::IpcCommand::SubmitRunInline,
        vb_ipc::IpcCommand::CancelRun,
        vb_ipc::IpcCommand::InspectRun,
        vb_ipc::IpcCommand::ListEvents,
        vb_ipc::IpcCommand::AnswerAsk,
        vb_ipc::IpcCommand::CompleteAction,
        vb_ipc::IpcCommand::FailAction,
        vb_ipc::IpcCommand::DrainTrace,
    ];
    let codes: HashSet<u16> = commands.iter().map(|c| c.as_u16()).collect();
    assert_eq!(
        codes.len(),
        commands.len(),
        "all commands must have unique codes"
    );
}

// ---------------------------------------------------------------------------
// Phase 12: Limits are enforced
// ---------------------------------------------------------------------------

#[test]
fn limits_max_expression_stack_is_bounded() {
    let max = vb_core::limits::MAX_EXPRESSION_STACK;
    assert!(
        max <= 64,
        "expression stack must be bounded to 64: got {max}"
    );
}

#[test]
fn limits_max_steps_per_workflow_is_bounded() {
    let max = vb_core::limits::MAX_STEPS_PER_WORKFLOW;
    assert!(max <= 65535, "max steps must be bounded: got {max}");
}

// ---------------------------------------------------------------------------
// Phase 13: CLI validate subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_validate_valid_minimal_workflow_succeeds() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("valid.yaml");
    let workflow = "version: velvet-ballastics/v1
name: validate_test
when:
  manual: {}
steps:
  - id: greet
    save:
      output: greeting
      value: '42'
  - id: done
    finish:
      result: greeting
";
    if !write_test_file(&workflow_path, workflow.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "validate valid workflow");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("valid"),
        "validate should print 'valid': {stdout}"
    );
}

#[test]
fn cli_validate_invalid_yaml_returns_parse_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("broken.yaml");
    if !write_test_file(&workflow_path, b"{{{not-yaml") {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate should fail on broken YAML"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("YAML parse error") || stderr.contains("YAML parse failed"),
        "validate should report parse error: {stderr}"
    );
}

#[test]
fn cli_validate_undefined_step_reference_returns_validation_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("bad-ref.yaml");
    let workflow = "version: velvet-ballastics/v1
name: bad_ref_test
when:
  manual: {}
steps:
  - id: greet
    save:
      output: greeting
      value: $steps.nonexistent
  - id: done
    finish:
      result: greeting
";
    if !write_test_file(&workflow_path, workflow.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate should fail on undefined step reference"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error"),
        "validate should report compile error for undefined step reference: {stderr}"
    );
}

#[test]
fn cli_validate_type_mismatch_returns_typed_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("type-mismatch.yaml");
    let workflow = "version: velvet-ballastics/v1
name: type_mismatch_test
when:
  manual: {}
steps:
  - id: greet
    save:
      output: message
      value: \"hello\"
    then: done
  - id: done
    finish:
      result: 1 + \"not_a_number\"
";
    if !write_test_file(&workflow_path, workflow.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("validate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate should fail on type mismatch"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error"),
        "validate should report compile error: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Phase 14: CLI compile subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_compile_valid_workflow_produces_ir() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let ir_path = dir.path().join("workflow.vbir");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "compile --emit ir");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("compiled IR written"),
        "compile should report IR written: {stdout}"
    );

    let ir_bytes = match std::fs::read(&ir_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to read compiled IR: {err}"
            );
            return;
        }
    };
    assert!(!ir_bytes.is_empty(), "compiled IR file should not be empty");

    let parts_result = postcard::from_bytes::<vb_core::workflow::WorkflowParts>(&ir_bytes);
    assert!(
        parts_result.is_ok(),
        "compiled IR should be valid postcard-encoded WorkflowParts: {parts_result:?}"
    );
}

#[test]
fn cli_compile_invalid_syntax_fails_with_clear_error() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("bad.yaml");
    let ir_path = dir.path().join("bad.vbir");

    if !write_test_file(&workflow_path, b"version: not-the-right-version\nsteps: []") {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "compile should fail on invalid workflow"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error"),
        "compile should report compile error: {stderr}"
    );
}

#[test]
fn cli_compile_preserves_workflow_digest() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let ir_path = dir.path().join("workflow.vbir");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "compile --emit ir");

    let ir_bytes = match std::fs::read(&ir_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to read compiled IR: {err}"
            );
            return;
        }
    };
    let parts = match postcard::from_bytes::<vb_core::workflow::WorkflowParts>(&ir_bytes) {
        Ok(parts) => parts,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to decode WorkflowParts: {err}"
            );
            return;
        }
    };

    let compile_result = vb_compile::compile_workflow(CLI_WORKFLOW.as_bytes());
    match compile_result {
        Ok(compiled) => {
            assert_eq!(
                parts.digest,
                compiled.digest(),
                "compiled IR digest should match in-memory compile digest"
            );
        }
        Err(err) => assert!(
            forced_assertion_failure(),
            "in-memory compile should succeed: {err:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// Phase 15: CLI run subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_run_minimal_workflow_completes() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "run --durability none");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run completed"),
        "run should report completion: {stdout}"
    );
}

#[test]
fn cli_run_strict_durability_writes_journal_events() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("strict-db");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run --durability strict");
    let stdout = output_stdout(&run_output);
    assert!(
        stdout.contains("run completed"),
        "strict run should complete: {stdout}"
    );

    let events_output = match run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&events_output, "events after strict run");
    let events_stdout = output_stdout(&events_output);
    assert!(
        events_stdout.contains("RunAccepted"),
        "strict run should produce RunAccepted event: {events_stdout}"
    );
    assert!(
        events_stdout.contains("RunFinished"),
        "strict run should produce RunFinished event: {events_stdout}"
    );
}

#[test]
fn cli_run_invalid_workflow_returns_error_exit_code() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("invalid.yaml");
    let input_path = dir.path().join("input.bin");

    if !write_test_file(&workflow_path, b"not-a-workflow-at-all") {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "run should fail on invalid workflow"
    );
    let stderr = output_stderr(&output);
    assert!(
        !stderr.is_empty(),
        "run should produce error output for invalid workflow"
    );
}

// ---------------------------------------------------------------------------
// Phase 16: CLI inspect subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_inspect_compiled_run_shows_status_and_event_count() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("inspect-db");

    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run for inspect setup");

    let inspect_output = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&inspect_output, "inspect 1");
    let stdout = output_stdout(&inspect_output);
    assert!(
        stdout.contains("status=finished"),
        "inspect should show finished status: {stdout}"
    );
    assert!(
        stdout.contains("events="),
        "inspect should show event count: {stdout}"
    );
}

#[test]
fn cli_inspect_nonexistent_run_shows_no_events() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("empty-db");

    let inspect_output = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("999"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_failure_contains(
        &inspect_output,
        "inspect nonexistent run",
        "no events found",
    );
}

// ---------------------------------------------------------------------------
// Doctor trim eligibility integration tests (vb-zo9d)
// ---------------------------------------------------------------------------

#[test]
fn cli_doctor_json_includes_trim_eligibility_check() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("doctor-trim-db");

    // Set up journal with one run that has a snapshot
    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(forced_assertion_failure(), "failed to open journal: {err}");
            return;
        }
    };

    let run = vb_core::RunId::new(100_000);
    let digest = vb_core::WorkflowDigest::from_bytes([0xCC; 32]);
    let events: Vec<vb_storage::JournalEvent> = (0..6u64)
        .map(|i| {
            if i == 0 {
                vb_storage::JournalEvent::RunAccepted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    workflow: digest,
                }
            } else {
                vb_storage::JournalEvent::StepStarted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    step: vb_core::StepIdx::new(
                        u16::try_from(i).unwrap_or_default().saturating_sub(1),
                    ),
                    attempt: 1,
                }
            }
        })
        .collect();
    if let Err(err) = journal.append_strict_batch(&events) {
        assert!(forced_assertion_failure(), "failed to append events: {err}");
        return;
    }

    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 100_000,
    };
    if let Err(err) = journal.put_run_header(&header) {
        assert!(forced_assertion_failure(), "failed to write header: {err}");
        return;
    }

    let snapshot = vb_storage::RunSnapshot {
        run,
        seq: vb_storage::EventSeq::new(3),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    if let Err(err) = journal.put_snapshot(&snapshot) {
        assert!(
            forced_assertion_failure(),
            "failed to write snapshot: {err}"
        );
        return;
    }
    drop(journal);

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "doctor --db <path> --json");

    let stdout = output_stdout(&output);
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "doctor JSON parse failed: {err}; stdout={stdout}"
            );
            return;
        }
    };

    // Find the trim_eligibility check
    let checks = match packet.get("checks").and_then(|c| c.as_array()) {
        Some(checks) => checks,
        None => {
            assert!(
                forced_assertion_failure(),
                "doctor JSON missing checks array: {stdout}"
            );
            return;
        }
    };
    let trim_check = checks
        .iter()
        .find(|c| c.get("check").and_then(|n| n.as_str()) == Some("trim_eligibility"));
    assert!(
        trim_check.is_some(),
        "doctor JSON should include trim_eligibility check: {stdout}"
    );
    let trim_check = trim_check.unwrap_or(&serde_json::Value::Null);
    assert_eq!(trim_check.get("status"), Some(&serde_json::json!("pass")));
    assert_eq!(trim_check.get("total_runs"), Some(&serde_json::json!(1)));
    assert_eq!(trim_check.get("eligible_runs"), Some(&serde_json::json!(1)));
    assert_eq!(trim_check.get("blocked_runs"), Some(&serde_json::json!(0)));
    assert_eq!(
        trim_check.get("total_events_trimmable"),
        Some(&serde_json::json!(3))
    );
}

#[test]
fn cli_doctor_text_reports_trim_eligibility() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("doctor-trim-text-db");

    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(forced_assertion_failure(), "failed to open journal: {err}");
            return;
        }
    };

    let run = vb_core::RunId::new(100_001);
    let digest = vb_core::WorkflowDigest::from_bytes([0xDD; 32]);
    let events = [
        vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow: digest,
        },
        vb_storage::JournalEvent::StepStarted {
            run,
            seq: vb_storage::EventSeq::new(1),
            step: vb_core::StepIdx::new(0),
            attempt: 1,
        },
    ];
    if let Err(err) = journal.append_strict_batch(&events) {
        assert!(forced_assertion_failure(), "failed to append events: {err}");
        return;
    }

    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 100_001,
    };
    if let Err(err) = journal.put_run_header(&header) {
        assert!(forced_assertion_failure(), "failed to write header: {err}");
        return;
    }

    // No snapshot — run should be blocked
    drop(journal);

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "doctor --db <path>");

    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("trim eligibility"),
        "doctor text output should mention trim eligibility: {stdout}"
    );
    assert!(
        stdout.contains("1 total"),
        "doctor text should report 1 total run: {stdout}"
    );
    assert!(
        stdout.contains("0 eligible"),
        "doctor text should report 0 eligible: {stdout}"
    );
    assert!(
        stdout.contains("1 blocked"),
        "doctor text should report 1 blocked: {stdout}"
    );
}

#[test]
fn cli_doctor_returns_success_for_healthy_journal_with_trim_recommended() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("doctor-trim-success-db");

    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(forced_assertion_failure(), "failed to open journal: {err}");
            return;
        }
    };

    let run = vb_core::RunId::new(100_002);
    let digest = vb_core::WorkflowDigest::from_bytes([0xEE; 32]);
    let events: Vec<vb_storage::JournalEvent> = (0..4u64)
        .map(|i| {
            if i == 0 {
                vb_storage::JournalEvent::RunAccepted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    workflow: digest,
                }
            } else {
                vb_storage::JournalEvent::StepStarted {
                    run,
                    seq: vb_storage::EventSeq::new(i),
                    step: vb_core::StepIdx::new(
                        u16::try_from(i).unwrap_or_default().saturating_sub(1),
                    ),
                    attempt: 1,
                }
            }
        })
        .collect();
    if let Err(err) = journal.append_strict_batch(&events) {
        assert!(forced_assertion_failure(), "failed to append events: {err}");
        return;
    }

    let header = vb_storage::RunHeaderRecord {
        run,
        workflow_id: vb_core::WorkflowId::new(0),
        compiled_digest: digest,
        status: 0,
        accepted_at_ms: 100_002,
    };
    if let Err(err) = journal.put_run_header(&header) {
        assert!(forced_assertion_failure(), "failed to write header: {err}");
        return;
    }

    let snapshot = vb_storage::RunSnapshot {
        run,
        seq: vb_storage::EventSeq::new(2),
        workflow: digest,
        slots: vec![0u8],
        taint: vec![],
    };
    if let Err(err) = journal.put_snapshot(&snapshot) {
        assert!(
            forced_assertion_failure(),
            "failed to write snapshot: {err}"
        );
        return;
    }
    drop(journal);

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "doctor --db <path>");
}

#[test]
fn cli_doctor_returns_storage_error_for_unreadable_path() {
    let nonexistent = std::path::PathBuf::from("/nonexistent/path/to/db");

    let output = match run_cli(&[
        std::ffi::OsStr::new("doctor"),
        std::ffi::OsStr::new("--db"),
        nonexistent.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "doctor should fail for unreadable path"
    );
}
// ---------------------------------------------------------------------------
// vb-qi37.13.4: Structured output contract tests
// ---------------------------------------------------------------------------

fn stdout_contains_no_panic_text(stdout: &str) {
    assert!(
        !stdout.contains("thread 'main' panicked"),
        "stdout leaked panic text: {stdout}"
    );
    assert!(
        !stdout.contains("stack backtrace"),
        "stdout leaked backtrace text: {stdout}"
    );
}

fn stderr_contains_no_panic_text(stderr: &str) {
    assert!(
        !stderr.contains("thread 'main' panicked"),
        "stderr leaked panic text: {stderr}"
    );
    assert!(
        !stderr.contains("stack backtrace"),
        "stderr leaked backtrace text: {stderr}"
    );
}

fn parse_yaml_stdout(output: &std::process::Output, command: &str) -> serde_json::Value {
    let stdout = output_stdout(output);
    match serde_saphyr::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            panic!("{command} did not emit parseable YAML: {error}; stdout={stdout}");
        }
    }
}

fn assert_postcard_stdout(
    output: &std::process::Output,
    command: &str,
    expected_payload_kind: Option<&str>,
    required_fields: &[&str],
) {
    assert_cli_success(output, command);
    assert_eq!(output_stderr(output), "", "{command} must not write stderr");
    let (header, packet) = cli_postcard::decode_postcard_json(&output.stdout)
        .unwrap_or_else(|error| panic!("{command} postcard payload must decode: {error}"));
    assert_eq!(header.magic, cli_postcard::CLI_MAGIC);
    assert_eq!(header.schema_version, cli_postcard::CLI_SCHEMA_VERSION);
    assert_eq!(header.kind, cli_postcard::CLI_POSTCARD_KIND);
    assert_eq!(header.header_len, 52);
    assert_eq!(
        packet.get("schema_version"),
        Some(&serde_json::json!("velvet-ballastics/cli-output/v1")),
        "{command} payload schema_version mismatch: {packet}"
    );
    if let Some(expected_kind) = expected_payload_kind {
        assert_eq!(
            packet.get("kind"),
            Some(&serde_json::json!(expected_kind)),
            "{command} payload kind mismatch: {packet}"
        );
    }
    for field in required_fields {
        assert!(
            packet.get(field).is_some(),
            "{command} payload missing field {field}: {packet}"
        );
    }
}

#[test]
fn cli_help_is_bounded_and_non_interactive() {
    let output = match run_cli(&[std::ffi::OsStr::new("--help")]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "--help");
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert_eq!(stderr, "", "help must not write stderr");
    assert!(
        stdout.contains("commands:"),
        "help should list commands: {stdout}"
    );
    assert!(
        stdout.contains(
            "system status [--profile <quick|standard|full>] [--server none] [--emit text|yaml]"
        ),
        "help should list canonical system status command: {stdout}"
    );
    assert!(
        stdout.len() <= 8192,
        "help output must stay bounded: {} bytes",
        stdout.len()
    );
    stdout_contains_no_panic_text(&stdout);
}

#[test]
fn cli_validate_help_short_circuits_missing_workflow_io() {
    let output = run_cli(&[
        std::ffi::OsStr::new("validate"),
        std::ffi::OsStr::new("--help"),
    ])
    .expect("validate --help command must execute");

    assert_cli_success(&output, "validate --help");
    assert_eq!(output_stderr(&output), "");
    assert!(output_stdout(&output).contains("commands:"));
}

#[test]
fn cli_validate_unknown_flag_fails_before_workflow_io() {
    let output = run_cli(&[
        std::ffi::OsStr::new("validate"),
        std::ffi::OsStr::new("missing.yaml"),
        std::ffi::OsStr::new("--bogus"),
    ])
    .expect("validate unknown flag command must execute");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert!(
        output_stderr(&output).starts_with("unknown flag for validate: --bogus"),
        "stderr was {}",
        output_stderr(&output)
    );
}

#[test]
fn cli_events_unknown_flag_fails_without_creating_storage_path() {
    let temp = cli_tempdir().expect("temporary directory must be available");
    let db = temp.path().join("missing-db");
    let output = run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("7"),
        std::ffi::OsStr::new("--db"),
        db.as_os_str(),
        std::ffi::OsStr::new("--garbage"),
    ])
    .expect("events unknown flag command must execute");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output_stdout(&output), "");
    assert!(
        output_stderr(&output).starts_with("unknown flag for events: --garbage"),
        "stderr was {}",
        output_stderr(&output)
    );
    assert!(!db.exists(), "parse failure must not create storage path");
}

#[test]
fn cli_canonical_emit_yaml_covers_required_output_contract_commands() {
    let dir = cli_tempdir().expect("tempdir for yaml output contract");
    let workflow_path = dir.path().join("emit-contract.yaml");
    let input_path = dir.path().join("emit-input.bin");
    let db_path = dir.path().join("emit-db");
    assert!(write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()));
    assert!(write_test_file(&input_path, &[]));

    let validate_output = run_cli(&[
        std::ffi::OsStr::new("validate"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("validate --emit yaml must run");
    assert_cli_success(&validate_output, "validate --emit yaml");
    assert_eq!(output_stderr(&validate_output), "");
    let validate = parse_yaml_stdout(&validate_output, "validate --emit yaml");
    assert_eq!(
        validate.get("kind"),
        Some(&serde_json::json!("validate_report"))
    );
    assert_eq!(validate.get("status"), Some(&serde_json::json!("valid")));

    let verify_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("verify --emit yaml must run");
    assert_cli_success(&verify_output, "verify --emit yaml");
    assert_eq!(output_stderr(&verify_output), "");
    let verify = parse_yaml_stdout(&verify_output, "verify --emit yaml");
    assert_eq!(
        verify.get("kind"),
        Some(&serde_json::json!("verify_report"))
    );
    assert!(
        verify.get("artifact").is_some(),
        "verify artifact missing: {verify}"
    );
    assert!(
        verify.get("replay").is_some(),
        "verify replay missing: {verify}"
    );

    let explain_output = run_cli(&[
        std::ffi::OsStr::new("explain"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("explain --emit yaml must run");
    assert_cli_success(&explain_output, "explain --emit yaml");
    assert_eq!(output_stderr(&explain_output), "");
    let explain = parse_yaml_stdout(&explain_output, "explain --emit yaml");
    assert_eq!(
        explain.get("kind"),
        Some(&serde_json::json!("explain_report"))
    );
    assert_eq!(explain.get("status"), Some(&serde_json::json!("valid")));

    let run_output = run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ])
    .expect("run setup for emit yaml must run");
    assert_cli_success(&run_output, "run setup for emit yaml");

    let events_output = run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("events --emit yaml must run");
    assert_cli_success(&events_output, "events --emit yaml");
    assert_eq!(output_stderr(&events_output), "");
    let events = parse_yaml_stdout(&events_output, "events --emit yaml");
    assert_eq!(
        events.get("kind"),
        Some(&serde_json::json!("events_report"))
    );
    assert!(
        events.get("events").is_some(),
        "events list missing: {events}"
    );
    assert!(
        events.get("total").is_some(),
        "events total missing: {events}"
    );

    let trace_output = run_cli(&[
        std::ffi::OsStr::new("trace"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("trace --emit yaml must run");
    assert_cli_success(&trace_output, "trace --emit yaml");
    assert_eq!(output_stderr(&trace_output), "");
    let trace = parse_yaml_stdout(&trace_output, "trace --emit yaml");
    assert_eq!(trace.get("kind"), Some(&serde_json::json!("trace_report")));
    assert!(trace.get("trace").is_some(), "trace list missing: {trace}");

    let replay_output = run_cli(&[
        std::ffi::OsStr::new("replay"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("replay --emit yaml must run");
    assert_cli_success(&replay_output, "replay --emit yaml");
    assert_eq!(output_stderr(&replay_output), "");
    let replay = parse_yaml_stdout(&replay_output, "replay --emit yaml");
    assert_eq!(
        replay.get("kind"),
        Some(&serde_json::json!("replay_report"))
    );
    assert!(
        replay.get("recovered").is_some(),
        "replay recovered missing: {replay}"
    );

    let diff_output = run_cli(&[
        std::ffi::OsStr::new("diff"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ])
    .expect("diff --emit yaml must run");
    assert_cli_success(&diff_output, "diff --emit yaml");
    assert_eq!(output_stderr(&diff_output), "");
    let diff = parse_yaml_stdout(&diff_output, "diff --emit yaml");
    assert_eq!(diff.get("kind"), Some(&serde_json::json!("diff_report")));
    assert!(
        diff.get("total_differences").is_some(),
        "diff total missing: {diff}"
    );
}

#[test]
fn cli_canonical_emit_postcard_frames_required_output_contract_commands() {
    let dir = cli_tempdir().expect("tempdir for postcard output contract");
    let workflow_path = dir.path().join("emit-postcard.yaml");
    let input_path = dir.path().join("emit-postcard-input.bin");
    let db_path = dir.path().join("emit-postcard-db");
    assert!(write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()));
    assert!(write_test_file(&input_path, &[]));

    let validate_output = run_cli(&[
        std::ffi::OsStr::new("validate"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("validate --emit postcard must run");
    assert_postcard_stdout(
        &validate_output,
        "validate --emit postcard",
        Some("validate_report"),
        &["schema_version", "success", "status", "exit_code"],
    );

    let verify_output = run_cli(&[
        std::ffi::OsStr::new("verify"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("standard"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("verify --emit postcard must run");
    assert_postcard_stdout(
        &verify_output,
        "verify --emit postcard",
        Some("verify_report"),
        &["schema_version", "success", "artifact", "replay"],
    );

    let explain_output = run_cli(&[
        std::ffi::OsStr::new("explain"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("explain --emit postcard must run");
    assert_postcard_stdout(
        &explain_output,
        "explain --emit postcard",
        Some("explain_report"),
        &["schema_version", "success", "status", "artifact"],
    );

    let run_output = run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ])
    .expect("run setup for emit postcard must run");
    assert_cli_success(&run_output, "run setup for emit postcard");

    let events_output = run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("events --emit postcard must run");
    assert_postcard_stdout(
        &events_output,
        "events --emit postcard",
        Some("events_report"),
        &["schema_version", "run_id", "events", "total"],
    );

    let trace_output = run_cli(&[
        std::ffi::OsStr::new("trace"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("trace --emit postcard must run");
    assert_postcard_stdout(
        &trace_output,
        "trace --emit postcard",
        Some("trace_report"),
        &["schema_version", "run_id", "trace", "total"],
    );

    let replay_output = run_cli(&[
        std::ffi::OsStr::new("replay"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("replay --emit postcard must run");
    assert_postcard_stdout(
        &replay_output,
        "replay --emit postcard",
        Some("replay_report"),
        &[
            "schema_version",
            "run_id",
            "recovered",
            "events",
            "terminal",
        ],
    );

    let diff_output = run_cli(&[
        std::ffi::OsStr::new("diff"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("postcard"),
    ])
    .expect("diff --emit postcard must run");
    assert_postcard_stdout(
        &diff_output,
        "diff --emit postcard",
        Some("diff_report"),
        &[
            "schema_version",
            "run_a",
            "run_b",
            "diffs",
            "total_differences",
        ],
    );
}

#[test]
fn cli_status_json_writes_payload_to_stdout_only() {
    let output = match run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "status --json");
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert_eq!(
        stderr, "",
        "status --json must keep diagnostics off stderr on success"
    );
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "status JSON did not parse: {error}; stdout={stdout}"
            );
            return;
        }
    };
    assert_eq!(packet.get("status"), Some(&serde_json::json!("running")));
    stdout_contains_no_panic_text(&stdout);
}

#[test]
fn cli_unknown_command_returns_stderr_diagnostic_without_stack_trace() {
    let output = match run_cli(&[std::ffi::OsStr::new("definitely-not-a-command")]) {
        Some(output) => output,
        None => return,
    };

    assert!(!output.status.success(), "unknown command must fail");
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert_eq!(stdout, "", "unknown command must not write stdout");
    assert!(
        stderr.contains("unknown command: definitely-not-a-command"),
        "stderr should name command: {stderr}"
    );
    stderr_contains_no_panic_text(&stderr);
}

#[test]
fn cli_emit_yaml_contract_is_not_silent_when_master_emit_mode_is_requested() {
    let output = match run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "status --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.starts_with("schema_version: velvet-ballastics/cli-output/v1"),
        "master structured YAML must start with schema_version: {stdout}"
    );
    assert!(
        stdout.contains("\nkind: status\n"),
        "YAML must include kind: {stdout}"
    );
    assert!(
        stdout.contains("\nstatus: running\n"),
        "YAML must include status: {stdout}"
    );
    assert!(
        !stdout.trim_start().starts_with('{'),
        "--emit yaml must not be JSON-shaped: {stdout}"
    );
    let parsed: serde_json::Value = match serde_saphyr::from_str(&stdout) {
        Ok(parsed) => parsed,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "status --emit yaml did not parse as YAML: {error}; stdout={stdout}"
            );
            return;
        }
    };
    assert_eq!(
        parsed.get("schema_version"),
        Some(&serde_json::json!("velvet-ballastics/cli-output/v1"))
    );
}

// ---------------------------------------------------------------------------
// vb-qi37.15.1: simulate command black-box tests
// ---------------------------------------------------------------------------

#[test]
fn cli_simulate_valid_workflow_reports_dry_run_summary() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("simulate.yaml");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("simulate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "simulate workflow.yaml");
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert_eq!(stderr, "", "simulate success must not write stderr");
    assert!(
        stdout.contains("simulation summary"),
        "missing summary: {stdout}"
    );
    assert!(
        stdout.contains("dry-run complete"),
        "missing dry-run completion: {stdout}"
    );
    assert!(
        stdout.contains("steps:    2"),
        "expected two steps: {stdout}"
    );
}

#[test]
fn cli_simulate_json_emits_deterministic_trace() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("simulate-json.yaml");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("simulate"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "simulate workflow.yaml --json");
    let stdout = output_stdout(&output);
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "simulate JSON parse failed: {error}; stdout={stdout}"
            );
            return;
        }
    };
    assert_eq!(packet.get("success"), Some(&serde_json::json!(true)));
    assert_eq!(packet.get("total_steps"), Some(&serde_json::json!(2)));
    assert_eq!(packet.get("total_actions"), Some(&serde_json::json!(0)));
    let trace_len = packet
        .get("trace")
        .and_then(|trace| trace.as_array())
        .map_or(0, std::vec::Vec::len);
    assert_eq!(
        trace_len, 2,
        "simulate trace should contain both steps: {stdout}"
    );
    assert_eq!(
        packet.get("schema_version"),
        Some(&serde_json::json!("velvet-ballastics/v1")),
        "simulate JSON must carry schema_version: {stdout}"
    );
    assert_eq!(
        packet.get("kind"),
        Some(&serde_json::json!("simulate")),
        "simulate JSON must carry kind: {stdout}"
    );
}

#[test]
fn cli_simulate_invalid_workflow_reports_diagnostic() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("invalid-simulate.yaml");
    if !write_test_file(&workflow_path, b"not-a-workflow") {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("simulate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };

    assert!(
        !output.status.success(),
        "simulate invalid workflow must fail"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("compile error") || stderr.contains("YAML"),
        "expected diagnostic: {stderr}"
    );
}

#[test]
fn cli_simulate_does_not_create_db_side_effects() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("simulate-no-db.yaml");
    let db_path = dir.path().join("simulate-db-should-not-exist");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("simulate"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "simulate without db");
    assert!(
        !db_path.exists(),
        "simulate must not create a durable DB path"
    );
}

// ---------------------------------------------------------------------------
// vb-qi37.15.2: submit command and job ledger tests
// ---------------------------------------------------------------------------

fn parse_submit_run_id(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("submitted run ").map(ToOwned::to_owned))
}

#[test]
fn cli_submit_persists_ledger_before_success() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("submit.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("submit-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes())
        || !write_test_file(&input_path, &[])
    {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("submit"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "submit journaled");
    let stdout = output_stdout(&output);
    let Some(run_id) = parse_submit_run_id(&stdout) else {
        assert!(
            forced_assertion_failure(),
            "submit did not print run id: {stdout}"
        );
        return;
    };

    let inspect = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new(&run_id),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&inspect, "inspect submitted run");
    let inspect_stdout = output_stdout(&inspect);
    assert!(
        inspect_stdout.contains(&run_id),
        "inspect should reference run id {run_id}: {inspect_stdout}"
    );
}

#[test]
fn cli_submit_json_returns_structured_identifiers() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("submit-json.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("submit-json-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes())
        || !write_test_file(&input_path, &[])
    {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("submit"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "submit --json");
    assert_eq!(
        output_stderr(&output),
        "",
        "submit --json success must not write stderr"
    );
    let stdout = output_stdout(&output);
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "submit JSON parse failed: {error}; stdout={stdout}"
            );
            return;
        }
    };
    assert!(
        packet
            .get("run_id")
            .and_then(|value| value.as_u64())
            .is_some(),
        "missing numeric run_id: {stdout}"
    );
    assert_eq!(packet.get("status"), Some(&serde_json::json!("submitted")));
    assert_eq!(packet.get("step_count"), Some(&serde_json::json!(2)));
    let digest_len = packet
        .get("digest")
        .and_then(|value| value.as_str())
        .map_or(0, str::len);
    assert_eq!(digest_len, 64, "digest must be 64 hex chars: {stdout}");
}

#[test]
fn cli_submit_rejects_missing_input_bin() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("submit-missing-input.yaml");
    let missing_input = dir.path().join("missing-input.bin");
    let db_path = dir.path().join("submit-missing-input-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("submit"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        missing_input.as_os_str(),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(!output.status.success(), "submit missing input must fail");
    assert_eq!(
        output_stdout(&output),
        "",
        "missing input must not write stdout"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("error reading"),
        "missing input should report read error: {stderr}"
    );
}

#[test]
fn cli_submit_rejects_unknown_durability() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("submit-bad-durability.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("submit-bad-durability-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes())
        || !write_test_file(&input_path, &[])
    {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("submit"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("unsafe-fast"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "submit unknown durability must fail"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("unknown durability mode: unsafe-fast"),
        "stderr should name bad mode: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// Extended CLI integration: end-to-end paths, error handling, and edge cases
// ---------------------------------------------------------------------------

#[test]
fn cli_replay_journaled_run_produces_deterministic_output() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("replay-workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("replay-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run setup for replay test");

    let replay_output = match run_cli(&[
        std::ffi::OsStr::new("replay"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&replay_output, "replay 1 --db");
    let stdout = output_stdout(&replay_output);
    assert!(
        stdout.contains("recovered"),
        "replay should report recovered state: {stdout}"
    );
    assert!(
        stdout.contains("terminal"),
        "replay should report terminal status: {stdout}"
    );
}

#[test]
fn cli_diff_identical_runs_reports_zero_differences() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("diff-workflow.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("diff-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&run_output, "run setup for diff test");

    let diff_output = match run_cli(&[
        std::ffi::OsStr::new("diff"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&diff_output, "diff 1 1 --db");
    let stdout = output_stdout(&diff_output);
    assert!(
        stdout.contains("0 differences")
            || stdout.contains("total_differences")
            || stdout.contains("no differences found"),
        "diff of identical runs should report 0 differences: {stdout}"
    );
}

#[test]
fn cli_run_json_output_reports_structured_run_result() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("run-json.yaml");
    let input_path = dir.path().join("input.bin");
    let db_path = dir.path().join("run-json-db");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("journaled"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "run --json");
    assert_eq!(
        output_stderr(&output),
        "",
        "run --json success must not write stderr"
    );
    let stdout = output_stdout(&output);
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "run JSON parse failed: {error}; stdout={stdout}"
            );
            return;
        }
    };
    assert!(
        packet.get("schema_version").is_some() || packet.get("run_id").is_some(),
        "expected schema_version or run_id: {stdout}"
    );
    assert!(packet.get("run_id").is_some(), "run_id missing: {stdout}");
}

#[test]
fn cli_run_nonexistent_workflow_file_fails_with_diagnostic() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let nonexistent_path = dir.path().join("does-not-exist.yaml");
    let input_path = dir.path().join("input.bin");
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        nonexistent_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "run with nonexistent file must fail"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("error reading")
            || stderr.contains("not found")
            || stderr.contains("No such file"),
        "should report file error: {stderr}"
    );
}

#[test]
fn cli_validate_nonexistent_file_fails_with_diagnostic() {
    let nonexistent_path = std::path::PathBuf::from("/tmp/vb-nonexistent-validate-test.yaml");

    let output = match run_cli(&[
        std::ffi::OsStr::new("validate"),
        nonexistent_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "validate with nonexistent file must fail"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("error reading")
            || stderr.contains("not found")
            || stderr.contains("No such file"),
        "should report file error: {stderr}"
    );
}

#[test]
fn cli_events_nonexistent_run_reports_empty_or_not_found() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("events-empty-db");

    let output = match run_cli(&[
        std::ffi::OsStr::new("events"),
        std::ffi::OsStr::new("42"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert!(
        !output.status.success()
            || stdout.contains("no events")
            || stdout.contains("0 event")
            || stderr.contains("no events")
            || stderr.contains("RUN_NOT_FOUND"),
        "events for nonexistent run must fail or report empty: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn cli_system_status_jsonl_output_is_single_line() {
    let output = match run_cli(&[
        std::ffi::OsStr::new("system"),
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--profile"),
        std::ffi::OsStr::new("quick"),
        std::ffi::OsStr::new("--server"),
        std::ffi::OsStr::new("none"),
        std::ffi::OsStr::new("--jsonl"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "system status --jsonl");
    let stdout = output_stdout(&output);
    let line_count = stdout.lines().count();
    assert_eq!(
        line_count, 1,
        "jsonl output must be exactly one line: {stdout}"
    );
    let packet: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(packet) => packet,
        Err(error) => {
            assert!(
                forced_assertion_failure(),
                "jsonl parse failed: {error}; stdout={stdout}"
            );
            return;
        }
    };
    assert_eq!(packet.get("kind"), Some(&serde_json::json!("SystemStatus")));
}

#[test]
fn cli_run_no_durability_works_without_db_flag() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("nodurability-workflow.yaml");
    let input_path = dir.path().join("input.bin");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "run --durability none without --db");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run completed"),
        "run should complete: {stdout}"
    );
}

#[test]
fn cli_explain_valid_workflow_outputs_valid_status() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("explain.yaml");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("explain"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "explain valid workflow");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("valid"),
        "explain should report valid status: {stdout}"
    );
    assert!(
        stdout.contains("node") || stdout.contains("step"),
        "explain should describe steps: {stdout}"
    );
}

#[test]
fn cli_explain_invalid_workflow_reports_errors() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("explain-invalid.yaml");
    if !write_test_file(&workflow_path, b"not-valid-yaml-at-all") {
        return;
    }

    let output = match run_cli(&[std::ffi::OsStr::new("explain"), workflow_path.as_os_str()]) {
        Some(output) => output,
        None => return,
    };
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert!(
        !output.status.success()
            || stdout.contains("YAML")
            || stdout.contains("parse")
            || stdout.contains("error")
            || stdout.contains("invalid")
            || stderr.contains("YAML")
            || stderr.contains("parse")
            || stderr.contains("error"),
        "should report error or diagnostic: stdout={stdout} stderr={stderr}"
    );
}

#[test]
fn cli_run_with_missing_db_path_for_strict_durability_fails_gracefully() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("strict-nodb.yaml");
    let input_path = dir.path().join("input.bin");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }
    if !write_test_file(&input_path, &[]) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("run"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("strict"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert!(
        !output.status.success(),
        "run --durability strict without --db must fail"
    );
    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("--db is required")
            || stderr.contains("storage")
            || stderr.contains("required")
            || stderr.contains("missing argument: --db"),
        "should report missing db: {stderr}"
    );
}

#[test]
fn cli_compile_without_out_flag_outputs_digest_only() {
    let dir = match cli_tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("compile-no-out.yaml");
    let ir_path = dir.path().join("compile-no-out.vbir");
    if !write_test_file(&workflow_path, CLI_WORKFLOW.as_bytes()) {
        return;
    }

    let output = match run_cli(&[
        std::ffi::OsStr::new("compile"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("ir"),
        std::ffi::OsStr::new("--out"),
        ir_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "compile --emit ir --out");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("compiled IR written"),
        "compile should report IR written: {stdout}"
    );
    let ir_bytes = match std::fs::read(&ir_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to read compiled IR: {err}"
            );
            return;
        }
    };
    assert!(!ir_bytes.is_empty(), "compiled IR file should not be empty");
}
