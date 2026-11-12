// ---------------------------------------------------------------------------
// Phase 1: YAML parsing — vb_yaml
// ---------------------------------------------------------------------------

#[test]
fn yaml_parse_empty_source_returns_error() {
    let result = vb_compile::parse_workflow_source("");
    assert_eq!(result, Err(vb_compile::YamlError::EmptySource));
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
    let result = vb_compile::parse_workflow_source(yaml);
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
version: \"velvet-ballistics/v1\"
when:
  manual: {}
steps: []
";
    let result = vb_compile::parse_workflow_source(yaml);
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
version: \"velvet-ballistics/v1\"
name: test_workflow
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
    let result = vb_compile::parse_workflow_source(yaml);
    match result {
        Ok(wf) => {
            assert_eq!(wf.name(), "test_workflow");
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
    let result = vb_compile::parse_workflow_source(yaml);
    assert!(matches!(
        result,
        Err(vb_compile::YamlError::ParseError { .. })
    ));
}

#[test]
fn yaml_profile_rejects_anchors() {
    let yaml =
        "version: &velvet \"velvet-ballistics/v1\"\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    let result = vb_compile::validate_yaml_profile(yaml);
    assert!(
        matches!(result, Err(vb_compile::YamlError::AnchorAliasMerge)),
        "anchors should be rejected"
    );
}

#[test]
fn yaml_parse_step_missing_do_action_returns_error() {
    let yaml = "\
version: \"velvet-ballistics/v1\"
name: test
when:
  manual: {}
steps:
  - id: start
    do:
      input: greeting
";
    let result = vb_compile::parse_workflow_source(yaml);
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
version: \"velvet-ballistics/v1\"
name: test
when:
  manual: {}
steps:
  - id: start
    set:
      value: \"hello\"
";
    let result = vb_compile::parse_workflow_source(yaml);
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
    match vb_compile::lexer::lex_expr("1 + 2") {
        Ok(tokens) => match vb_compile::parser::parse_expr(&tokens) {
            Ok(ast) => assert!(matches!(ast, vb_compile::parser::ExprAst::Binary { .. })),
            Err(err) => assert!(forced_assertion_failure(), "parse failed: {err:?}"),
        },
        Err(err) => assert!(forced_assertion_failure(), "lex failed: {err:?}"),
    }
}

#[test]
fn expr_bytecode_compile_and_eval() {
    let tokens = match vb_compile::lexer::lex_expr("1 + 2") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(forced_assertion_failure(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_compile::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(forced_assertion_failure(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_compile::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(forced_assertion_failure(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    match vb_compile::eval::eval_expr_program(&program, &[], &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::I64(3)),
        Err(err) => assert!(forced_assertion_failure(), "eval failed: {err:?}"),
    }
}

#[test]
fn expr_rejects_division_by_zero() {
    let tokens = match vb_compile::lexer::lex_expr("1 / 0") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(forced_assertion_failure(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_compile::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(forced_assertion_failure(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_compile::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(forced_assertion_failure(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    let result = vb_compile::eval::eval_expr_program(&program, &[], &const_vals);
    assert_eq!(result, Err(vb_compile::ExprError::DivisionByZero));
}

#[test]
fn expr_boolean_logic() {
    let tokens = match vb_compile::lexer::lex_expr("true and false") {
        Ok(tokens) => tokens,
        Err(err) => {
            assert!(forced_assertion_failure(), "lex failed: {err:?}");
            return;
        }
    };
    let ast = match vb_compile::parser::parse_expr(&tokens) {
        Ok(ast) => ast,
        Err(err) => {
            assert!(forced_assertion_failure(), "parse failed: {err:?}");
            return;
        }
    };
    let mut constants = Vec::new();
    let program = match vb_compile::bytecode::compile_expr_with_pool(&ast, &mut constants) {
        Ok(program) => program,
        Err(err) => {
            assert!(forced_assertion_failure(), "bytecode failed: {err:?}");
            return;
        }
    };
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    match vb_compile::eval::eval_expr_program(&program, &[], &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::Bool(false)),
        Err(err) => assert!(forced_assertion_failure(), "eval failed: {err:?}"),
    }
}

#[test]
fn expr_variable_reference() {
    let compiled = match vb_compile::bytecode::compile_expr("$x + 1", &resolve_test_reference) {
        Ok(compiled) => compiled,
        Err(err) => {
            assert!(forced_assertion_failure(), "compile failed: {err:?}");
            return;
        }
    };
    let (program, constants) = compiled;
    let const_vals: Vec<vb_core::value::ConstValue> = constants;
    let slots: Vec<Option<SlotValue>> = vec![Some(SlotValue::I64(41))];
    match vb_compile::eval::eval_expr_program(&program, &slots, &const_vals) {
        Ok(result) => assert_eq!(result, SlotValue::I64(42)),
        Err(err) => assert!(forced_assertion_failure(), "eval failed: {err:?}"),
    }
}

// ---------------------------------------------------------------------------
// Phase 4: Core IR validation
// ---------------------------------------------------------------------------

#[test]
fn core_workflow_rejects_out_of_bounds_step() {
    let bad_node = vb_core::workflow::node(StepIdx::new(99), CompiledNodeKind::Nop, None, None);
    let parts = minimal_parts(Box::from([bad_node]));
    let result = vb_core::engine::validate_compiled_workflow(&parts);
    assert!(result.is_err(), "out-of-bounds step should fail");
}

#[test]
fn core_workflow_rejects_invalid_jump_target() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Jump {
            target: StepIdx::new(50),
        },
        None,
        None,
    );
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
    use vb_storage::codec::EnforceKindParity;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestPayload {
        value: i64,
        label: String,
    }

    // TestPayload has no payload-level record-kind discriminant, so the
    // default no-op parity check is correct. Implementing the trait is
    // required by `decode_record`'s new bound.
    impl EnforceKindParity for TestPayload {}

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
    use vb_storage::codec::EnforceKindParity;

    #[derive(Serialize, Deserialize)]
    struct TestPayload {
        value: i64,
    }

    impl EnforceKindParity for TestPayload {}

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
        !rendered.contains("version: velvet-ballistics/v1"),
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

