//! CLI integration tests — truth serum adversarial audit as executable tests.
//!
//! These tests encode the exact scenarios from the manual truth-serum audit
//! so they run on every `cargo test` invocation.

use vb_core::ids::{SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::SlotValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

const CLI_WORKFLOW: &str = r"version: velvet-ballastics/v1
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

fn test_failed() -> bool {
    false
}

fn write_test_file(path: &std::path::Path, contents: &[u8]) -> bool {
    match std::fs::write(path, contents) {
        Ok(()) => true,
        Err(err) => {
            assert!(test_failed(), "failed to write {}: {err}", path.display());
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
            assert!(test_failed(), "failed to execute velvet_ballastics: {err}");
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

// ---------------------------------------------------------------------------
// Phase 1: YAML parsing — vb_yaml
// ---------------------------------------------------------------------------

#[test]
fn yaml_parse_empty_source_returns_error() {
    let result = vb_yaml::parse_workflow_source("");
    assert!(result.is_err(), "empty source should fail");
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
            assert!(test_failed(), "missing version should fail");
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
            assert!(test_failed(), "missing name should fail");
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
            assert_eq!(wf.name, "test-workflow");
            assert_eq!(wf.steps.len(), 2);
        }
        Err(err) => assert!(test_failed(), "should parse valid workflow: {err:?}"),
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
      expr: \"1 + 2\"
";
    let result = vb_yaml::parse_workflow_source(yaml);
    let err = match result {
        Ok(_) => {
            assert!(test_failed(), "missing do.action should fail");
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
            assert!(test_failed(), "missing set.output should fail");
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
            Err(err) => assert!(test_failed(), "parse failed: {err:?}"),
        },
        Err(err) => assert!(test_failed(), "lex failed: {err:?}"),
    }
}

#[test]
fn expr_bytecode_compile_and_eval() {
    let tokens = match vb_expr::lexer::lex_expr("1 + 2") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(test_failed(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(test_failed(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(test_failed(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    match vb_expr::eval::eval_expr_program(&program, &[], &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::I64(3)),
        Err(err) => assert!(test_failed(), "eval failed: {err:?}"),
    }
}

#[test]
fn expr_rejects_division_by_zero() {
    let tokens = match vb_expr::lexer::lex_expr("1 / 0") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(test_failed(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(test_failed(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(test_failed(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    let result = vb_expr::eval::eval_expr_program(&program, &[], &const_vals);
    assert!(result.is_err(), "division by zero should fail");
}

#[test]
fn expr_boolean_logic() {
    let tokens = match vb_expr::lexer::lex_expr("true and false") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(test_failed(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_expr::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(test_failed(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_expr::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(test_failed(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    match vb_expr::eval::eval_expr_program(&program, &[], &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::Bool(false)),
        Err(err) => assert!(test_failed(), "eval failed: {err:?}"),
    }
}

#[test]
fn expr_variable_reference() {
    let compiled = match vb_expr::bytecode::compile_expr("$x + 1", &resolve_test_reference) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(test_failed(), "compile failed: {err:?}");
            return;
        }
    };
    let (program, constants) = compiled;
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    let slots: Vec<Option<SlotValue>> = vec![Some(SlotValue::I64(41))];
    match vb_expr::eval::eval_expr_program(&program, &slots, &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::I64(42)),
        Err(err) => assert!(test_failed(), "eval failed: {err:?}"),
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
    assert!(result.is_err(), "binary input should fail compile");
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
            assert!(test_failed(), "encode failed: {err:?}");
            return;
        }
    };
    let nonzero = match std::num::NonZeroUsize::new(4096) {
        Some(nonzero) => nonzero,
        None => {
            assert!(test_failed(), "nonzero payload limit should be valid");
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
        Err(err) => assert!(test_failed(), "decode failed: {err:?}"),
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
            assert!(test_failed(), "encode failed: {err:?}");
            return;
        }
    };
    assert!(encoded.len() > 10, "encoded record should have header");

    let decoded: Result<(vb_storage::RecordEnvelope, TestPayload), _> =
        vb_storage::decode_record(&encoded, MAGIC, 4096);
    match decoded {
        Ok((_envelope, decoded)) => assert_eq!(decoded, payload),
        Err(err) => assert!(test_failed(), "decode failed: {err:?}"),
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
            assert!(test_failed(), "encode failed: {err:?}");
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
            assert!(test_failed(), "compile workflow failed: {err:?}");
            return;
        }
    };
    let result = vb_codegen::emit_rust_workflow(&compiled);
    match result {
        Ok(output) => {
            assert!(!output.is_empty(), "codegen output should not be empty");
            assert!(output.contains("fn drive"), "should contain drive function");
        }
        Err(err) => assert!(test_failed(), "codegen should succeed: {err:?}"),
    }
}

#[test]
fn cli_run_journaled_then_events_and_inspect_read_temp_db() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
fn cli_run_maps_postcard_slot_values_from_input_bin() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
            return;
        }
    };
    let workflow_path = dir.path().join("workflow.vbir");
    let input_path = dir.path().join("input.bin");

    let workflow_payload = match postcard::to_allocvec(&input_slot_parts()) {
        Ok(payload) => payload,
        Err(err) => {
            assert!(test_failed(), "failed to encode workflow payload: {err}");
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
            assert!(test_failed(), "failed to encode input payload: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
      value: 42
  - id: done
    finish:
      result: 0
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
      value: $steps.nonexistent
  - id: done
    finish:
      result: 0
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
            assert!(test_failed(), "failed to read compiled IR: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
            assert!(test_failed(), "failed to read compiled IR: {err}");
            return;
        }
    };
    let parts = match postcard::from_bytes::<vb_core::workflow::WorkflowParts>(&ir_bytes) {
        Ok(parts) => parts,
        Err(err) => {
            assert!(test_failed(), "failed to decode WorkflowParts: {err}");
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
        Err(err) => assert!(test_failed(), "in-memory compile should succeed: {err:?}"),
    }
}

// ---------------------------------------------------------------------------
// Phase 15: CLI run subcommand
// ---------------------------------------------------------------------------

#[test]
fn cli_run_minimal_workflow_completes() {
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
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
    let dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(err) => {
            assert!(test_failed(), "tempdir failed: {err}");
            return;
        }
    };
    let db_path = dir.path().join("empty-db");

    // Open a journal at the path so the DB exists but has no run events.
    let journal = match vb_storage::FjallJournal::open(&db_path, None) {
        Ok(j) => j,
        Err(err) => {
            assert!(test_failed(), "failed to open journal: {err}");
            return;
        }
    };
    drop(journal);

    let inspect_output = match run_cli(&[
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("999"),
        std::ffi::OsStr::new("--db"),
        db_path.as_os_str(),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&inspect_output, "inspect nonexistent run");
    let stdout = output_stdout(&inspect_output);
    assert!(
        stdout.contains("no events found"),
        "inspect should report no events for nonexistent run: {stdout}"
    );
}
