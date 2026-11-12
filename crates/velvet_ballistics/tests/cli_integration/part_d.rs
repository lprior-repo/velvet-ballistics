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
        Some(&serde_json::json!("velvet-ballistics/cli-output/v1")),
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
    // Scoped per-command help (vb-c5wp6): usage for validate only, not the
    // full command list. Full help remains available via bare `help`.
    assert!(
        output_stdout(&output).contains("validate   <workflow.yaml>"),
        "stdout={}",
        output_stdout(&output)
    );
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "status --emit yaml");
    let stdout = output_stdout(&output);
    let stderr = output_stderr(&output);
    assert_eq!(
        stderr, "",
        "status --emit yaml must keep diagnostics off stderr on success"
    );
    assert!(
        stdout.contains("status: running"),
        "YAML output should contain status: running; got: {stdout}"
    );
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
        stdout.starts_with("schema_version: velvet-ballistics/cli-output/v1"),
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
        Some(&serde_json::json!("velvet-ballistics/cli-output/v1"))
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => return,
    };

    assert_cli_success(&output, "simulate workflow.yaml --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("success: true"),
        "YAML output should contain success: true; got: {stdout}"
    );
    assert!(
        stdout.contains("total_steps: 2"),
        "YAML output should contain total_steps: 2; got: {stdout}"
    );
    assert!(
        stdout.contains("total_actions: 0"),
        "YAML output should contain total_actions: 0; got: {stdout}"
    );
    assert!(
        stdout.contains("trace:"),
        "YAML output should contain trace: ; got: {stdout}"
    );
    assert!(
        stdout.contains("schema_version: velvet-ballistics/v1"),
        "YAML output should contain schema_version: velvet-ballistics/v1; got: {stdout}"
    );
    assert!(
        stdout.contains("kind: simulate"),
        "YAML output should contain kind: simulate; got: {stdout}"
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "submit --emit yaml");
    assert_eq!(
        output_stderr(&output),
        "",
        "submit --emit yaml success must not write stderr"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run_id:"),
        "YAML output should contain run_id: ; got: {stdout}"
    );
    assert!(
        stdout.contains("status: submitted"),
        "YAML output should contain status: submitted; got: {stdout}"
    );
    assert!(
        stdout.contains("step_count: 2"),
        "YAML output should contain step_count: 2; got: {stdout}"
    );
    assert!(
        stdout.contains("digest:"),
        "YAML output should contain digest: ; got: {stdout}"
    );
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]) {
        Some(output) => output,
        None => return,
    };
    assert_cli_success(&output, "run --emit yaml");
    assert_eq!(
        output_stderr(&output),
        "",
        "run --emit yaml success must not write stderr"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("run_id:") || stdout.contains("schema_version:"),
        "YAML output should contain run_id or schema_version; got: {stdout}"
    );
    assert!(stdout.contains("run_id:"), "run_id missing: {stdout}");
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
fn cli_system_status_legacy_jsonl_flag_emits_status_text() {
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
    assert_cli_success(&output, "system status legacy --jsonl");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("system_status: degraded"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("profile: quick"), "stdout={stdout}");
    assert!(stdout.contains("server: none"), "stdout={stdout}");
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
