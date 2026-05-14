#![forbid(unsafe_code)]
//! AI Context CLI integration tests — RED PHASE
//!
//! These tests verify the `ai-context` CLI command behavior.
//! They will FAIL until the actual implementation is provided.
//!
//! In RED phase, these tests call the CLI which calls stub functions
//! that panic with `todo!()`.

use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::SlotValue;
use std::ffi::OsStr;

// ============================================================================
// Test workflow YAML (minimal workflow for testing)
// ============================================================================

const CLI_WORKFLOW: &str = r#"version: velvet-ballastics/v1
name: cli_subprocess
when:
  manual: {}
steps:
  - id: build_result
    save:
      value: 42
  - id: done
    finish:
      result: 0
"#;

// ============================================================================
// Helpers
// ============================================================================

fn forced_assertion_failure() -> bool {
    false
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
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_vb"));
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

fn assert_cli_success(output: &std::process::Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed: stdout={} stderr={}",
        output_stdout(output),
        output_stderr(output)
    );
}

// ============================================================================
// Integration Tests — RED PHASE
// These tests will FAIL at runtime because the stub functions panic with todo!()
// ============================================================================

/// Test: ai-context emits AiContextPacket JSON when run exists with events
/// PRE: Journal exists with run and events
/// WHEN: ai-context is called with valid run_id
/// THEN: stdout receives valid JSON with schema_version, kind, run_id, etc.
/// RED PHASE: handle() calls todo!() and panics
#[test]
fn ai_context_emits_valid_json_packet_for_existing_run() {
    let dir = match tempfile::tempdir() {
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

    // First run the workflow to create journal entries
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

    // Now call ai-context on the run
    // RED PHASE: This will panic because handle() is a stub
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

    // Assertions about the output
    assert_cli_success(&context_output, "ai-context 1 --json --json");
    let stdout = output_stdout(&context_output);

    // Parse JSON and verify schema
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

    // Verify POST-001: schema_version, kind, run_id
    assert_eq!(
        packet.pointer("/schema_version"),
        Some(&serde_json::json!("1")),
        "schema_version must be '1'"
    );
    assert_eq!(
        packet.pointer("/kind"),
        Some(&serde_json::json!("AiContextPacket")),
        "kind must be 'AiContextPacket'"
    );
    assert_eq!(
        packet.pointer("/run_id"),
        Some(&serde_json::json!(1)),
        "run_id must be 1"
    );

    // Verify POST-001: journal_event_trail exists and is array
    assert!(
        packet.pointer("/journal_event_trail").is_some_and(|v| v.is_array()),
        "journal_event_trail must be an array"
    );

    // Verify POST-001: action_contracts exists
    assert!(
        packet.pointer("/action_contracts").is_some(),
        "action_contracts must exist"
    );

    // Verify POST-001: trace_ring_snapshot exists
    assert!(
        packet.pointer("/trace_ring_snapshot").is_some(),
        "trace_ring_snapshot must exist"
    );

    // Verify POST-001: suggested_next_cli_commands exists and is array
    assert!(
        packet
            .pointer("/suggested_next_cli_commands")
            .is_some_and(|v| v.is_array()),
        "suggested_next_cli_commands must be an array"
    );
}

/// Test: ai-context reports RUN_NOT_FOUND when run has zero events
/// PRE: Journal exists but run has no events
/// WHEN: ai-context is called with run_id that has no events
/// THEN: exit code is ValidationFailed, stderr contains RUN_NOT_FOUND
/// RED PHASE: handle() calls report_run_not_found() which is a stub
#[test]
fn ai_context_run_not_found_for_zero_event_run() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("fjall-db");

    // Call ai-context with a non-existent run
    // RED PHASE: This will panic because handle() is a stub
    let output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("999"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };

    // POST-006: exit code is ValidationFailed
    assert_eq!(
        output.status.code(),
        Some(1),
        "run not found should return exit code 1"
    );

    let stderr = output_stderr(&output);

    // POST-006: stderr contains RUN_NOT_FOUND code
    assert!(
        stderr.contains("RUN_NOT_FOUND"),
        "stderr should contain 'RUN_NOT_FOUND': {stderr}"
    );

    // POST-006: stderr contains the run_id
    assert!(
        stderr.contains("999"),
        "stderr should contain run id '999': {stderr}"
    );
}

/// Test: ai-context returns StorageError when journal cannot be opened
/// PRE: --db path does not exist
/// WHEN: ai-context is called with invalid db path
/// THEN: exit code is StorageError
/// RED PHASE: handle() is a stub that will panic
#[test]
fn ai_context_storage_error_for_nonexistent_db_path() {
    let output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        std::ffi::OsStr::new("/nonexistent/path"),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };

    // Should fail with StorageError (exit code 5)
    assert_eq!(
        output.status.code(),
        Some(5),
        "nonexistent db path should return exit code 5 (StorageError)"
    );

    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("opening journal") || stderr.contains("does not exist"),
        "stderr should mention journal opening failure: {stderr}"
    );
}

/// Test: ai-context emits [REDACTED] for secret-tainted slot values
/// PRE: Journal contains slot writes with taint tracking
/// WHEN: ai-context is called
/// THEN: secret/derived slots show [REDACTED] in output
/// RED PHASE: redacted_slot_value() is a stub
#[test]
fn ai_context_redacts_secret_tainted_slot_values() {
    let dir = match tempfile::tempdir() {
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

    // Run workflow to create journal
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

    // Call ai-context
    // RED PHASE: This will panic because the implementation is stubbed
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
                "JSON parse failed: {err}; stdout={stdout}"
            );
            return;
        }
    };

    // Verify POST-003: no raw bytes appear in output
    let rendered = packet.to_string();
    assert!(
        !rendered.contains("REDACTED") || rendered.contains("[REDACTED]"),
        "secret slots should render as [REDACTED]"
    );
}

/// Test: ai-context resolves workflow digest to compiled IR
/// PRE: Journal contains compiled IR for the workflow
/// WHEN: ai-context is called
/// THEN: workflow.compiled_ir.available is true
/// RED PHASE: handle() is a stub
#[test]
fn ai_context_resolves_workflow_digest_to_compiled_ir() {
    let dir = match tempfile::tempdir() {
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

    // Run workflow
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

    // Call ai-context
    // RED PHASE: This will panic
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
                "JSON parse failed: {err}"
            );
            return;
        }
    };

    // POST-002: workflow field has digest and compiled_ir
    assert!(
        packet.pointer("/workflow/digest").is_some(),
        "workflow.digest must be present"
    );

    // POST-002: compiled_ir must have availability flag
    assert_eq!(
        packet.pointer("/workflow/compiled_ir/available"),
        Some(&serde_json::json!(true)),
        "compiled_ir.available must be true for finished workflow"
    );

    // POST-002: compiled_ir must have node_count
    assert!(
        packet.pointer("/workflow/compiled_ir/node_count").is_some(),
        "compiled_ir.node_count must be present"
    );

    // POST-002: referenced_actions must be present
    assert!(
        packet.pointer("/workflow/referenced_actions").is_some(),
        "workflow.referenced_actions must be present"
    );
}

/// Test: ai-context infers action IDs from both events and compiled IR
/// PRE: Journal contains ActionScheduled events and compiled IR has Do nodes
/// WHEN: ai-context is called
/// THEN: action_contracts contains unique IDs from both sources
/// RED PHASE: handle() is a stub
#[test]
fn ai_context_infers_action_ids_from_both_sources() {
    let dir = match tempfile::tempdir() {
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

    // Run workflow
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

    // Call ai-context
    // RED PHASE: This will panic
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
                "JSON parse failed: {err}"
            );
            return;
        }
    };

    // POST-005: action_contracts is an array
    let contracts = match packet.pointer("/action_contracts") {
        Some(serde_json::Value::Array(arr)) => arr,
        other => {
            assert!(
                forced_assertion_failure(),
                "action_contracts must be an array: {other:?}"
            );
            return;
        }
    };

    // POST-005: each entry has contract_status
    for contract in contracts {
        assert_eq!(
            contract.pointer("/contract_status"),
            Some(&serde_json::json!("inferred_from_compiled_ir_and_journal")),
            "contract_status must be 'inferred_from_compiled_ir_and_journal'"
        );
    }
}

/// Test: ai-context suggests correct commands based on run status
/// PRE: Run is in Finished status
/// WHEN: ai-context is called
/// THEN: suggested commands include inspect, events, and replay
/// RED PHASE: suggested_ai_commands() is a stub
#[test]
fn ai_context_suggests_correct_commands_for_finished_run() {
    let dir = match tempfile::tempdir() {
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

    // Run workflow to completion
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

    // Call ai-context
    // RED PHASE: This will panic
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
                "JSON parse failed: {err}"
            );
            return;
        }
    };

    // INV-002: suggested_next_cli_commands has max 4 commands
    let suggestions = match packet.pointer("/suggested_next_cli_commands") {
        Some(serde_json::Value::Array(arr)) => arr,
        other => {
            assert!(
                forced_assertion_failure(),
                "suggested_next_cli_commands must be array: {other:?}"
            );
            return;
        }
    };

    assert!(
        suggestions.len() <= 4,
        "INV-002: max 4 suggested commands, got {}",
        suggestions.len()
    );

    // INV-3: all commands start with velvet-ballastics
    for suggestion in suggestions {
        if let Some(cmd) = suggestion.as_str() {
            assert!(
                cmd.starts_with("velvet-ballastics "),
                "INV-003: command must start with 'velvet-ballastics ': {cmd}"
            );
        }
    }

    // For Finished status: should suggest inspect, events, replay
    let suggestion_strings: Vec<&str> = suggestions
        .iter()
        .filter_map(|s| s.as_str())
        .collect();

    assert!(
        suggestion_strings.iter().any(|c| c.contains("inspect")),
        "Finished run should suggest inspect"
    );
    assert!(
        suggestion_strings.iter().any(|c| c.contains("events")),
        "Finished run should suggest events"
    );
    assert!(
        suggestion_strings.iter().any(|c| c.contains("replay")),
        "Finished run should suggest replay"
    );
}

/// Test: ai-context handles corrupt journal gracefully
/// PRE: Journal file is corrupt
/// WHEN: ai-context is called
/// THEN: exit code is StorageError
/// RED PHASE: handle() is a stub
#[test]
fn ai_context_handles_corrupt_journal() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("corrupt.db");

    // Write some garbage to the journal file
    if !write_test_file(&db_path, b"this is not a valid fjall database") {
        return;
    }

    // Call ai-context
    // RED PHASE: This will panic
    let output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("1"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };

    // Should fail with StorageError
    assert_eq!(
        output.status.code(),
        Some(5),
        "corrupt journal should return StorageError (5)"
    );
}

/// Test: ai-context handles missing snapshot gracefully
/// PRE: Run exists but latest_snapshot returns None
/// WHEN: ai-context is called
/// THEN: trace_ring_snapshot is null, exit code is 0
/// RED PHASE: handle() is a stub
#[test]
fn ai_context_handles_missing_snapshot() {
    let dir = match tempfile::tempdir() {
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

    // Run workflow
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

    // Call ai-context
    // RED PHASE: This will panic
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
                "JSON parse failed: {err}"
            );
            return;
        }
    };

    // trace_ring_snapshot should be present (may be null or have fabricated: false)
    assert!(
        packet.pointer("/trace_ring_snapshot").is_some(),
        "trace_ring_snapshot must be present"
    );
}

/// Test: ai-context validates run_id format
/// PRE: None
/// WHEN: ai-context is called with non-numeric run_id
/// THEN: exit code is ValidationFailed, stderr contains error message
/// RED PHASE: handle() calls parse_run_id() which is a stub
#[test]
fn ai_context_rejects_invalid_run_id_format() {
    let output = match run_cli(&[
        std::ffi::OsStr::new("ai-context"),
        std::ffi::OsStr::new("not-a-number"),
        std::ffi::OsStr::new("--db"),
        std::ffi::OsStr::new("/tmp/test.db"),
        std::ffi::OsStr::new("--json"),
    ]) {
        Some(output) => output,
        None => return,
    };

    // Should fail with ValidationFailed
    assert_eq!(
        output.status.code(),
        Some(1),
        "invalid run_id should return exit code 1 (ValidationFailed)"
    );

    let stderr = output_stderr(&output);
    assert!(
        stderr.contains("invalid run_id"),
        "stderr should mention 'invalid run_id': {stderr}"
    );
}
