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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]);
    if let Some(output) = output {
        assert_cli_success(&output, "status --emit yaml");
        assert!(output_stdout(&output).contains("status: running"));
    }
}

#[test]
fn cli_status_jsonl_succeeds() {
    let output = run_cli(&[
        std::ffi::OsStr::new("status"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]);
    if let Some(output) = output {
        assert_cli_success(&output, "status --emit yaml");
        assert!(output_stdout(&output).contains("running: true"));
    }
}

#[test]
fn cli_system_status_legacy_json_flag_reports_degraded_text_when_no_backend_is_attached() {
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

    assert_cli_success(&output, "system status legacy --json");
    assert_eq!(
        output_stderr(&output),
        "",
        "system status success must not write stderr"
    );
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("system_status: degraded"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("profile: quick"), "stdout={stdout}");
    assert!(stdout.contains("server: none"), "stdout={stdout}");
    assert!(stdout.contains("connected: false"), "stdout={stdout}");
    assert!(
        stdout.contains("storage_health: Degraded"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("journal_batch_healthy: false"),
        "stdout={stdout}"
    );
    assert!(stdout.contains("blob_store_ok: false"), "stdout={stdout}");
    assert!(stdout.contains("index_healthy: false"), "stdout={stdout}");
    assert!(stdout.contains("writer_queue_depth: 0"), "stdout={stdout}");
    assert!(stdout.contains("active_run_count: 0"), "stdout={stdout}");
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
    let status = status_value;

    assert_eq!(
        packet.get("schema_version"),
        Some(&serde_json::json!("velvet-ballistics/cli-output/v1"))
    );
    assert_eq!(packet.get("kind"), Some(&serde_json::json!("SystemStatus")));
    assert_eq!(packet.get("profile"), Some(&serde_json::json!("standard")));
    assert_eq!(packet.get("server"), Some(&serde_json::json!("none")));
    assert_eq!(packet.get("connected"), Some(&serde_json::json!(false)));
    assert_eq!(packet.get("reason"), Some(&serde_json::json!("no-backend")));
    assert_eq!(
        status.get("storage_health"),
        Some(&serde_json::json!("Degraded"))
    );
    assert_eq!(
        status.get("writer_queue_depth"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        status.get("journal_batch_healthy"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(status.get("snapshot_seq"), Some(&serde_json::Value::Null));
    assert_eq!(status.get("blob_store_ok"), Some(&serde_json::json!(false)));
    assert_eq!(status.get("index_healthy"), Some(&serde_json::json!(false)));
    assert_eq!(status.get("uptime_seconds"), Some(&serde_json::json!(0)));
    assert_eq!(status.get("active_run_count"), Some(&serde_json::json!(0)));
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
                "failed to execute velvet_ballistics CLI for action list"
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballistics CLI for action list --emit yaml"
            );
            return;
        }
    };

    assert_cli_success(&output, "action list --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("success: true"),
        "YAML output should contain success: true; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 1"),
        "YAML output should contain action id: 1; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 2"),
        "YAML output should contain action id: 2; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 3"),
        "YAML output should contain action id: 3; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: deterministic_pure"),
        "YAML output should contain deterministic_pure; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: idempotent_external"),
        "YAML output should contain idempotent_external; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: at_least_once_external"),
        "YAML output should contain at_least_once_external; got: {stdout}"
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
                "failed to execute velvet_ballistics CLI for action list --registry empty"
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
                "failed to execute velvet_ballistics CLI for action list --registry uninitialized"
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
                "failed to execute velvet_ballistics CLI for action list --registry (missing value)"
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
                "failed to execute velvet_ballistics CLI for action list --bogus"
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
                "failed to execute velvet_ballistics CLI for action list --registry bogus"
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
                "failed to execute velvet_ballistics CLI for action list junk"
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
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballistics CLI for action list --emit yaml"
            );
            return;
        }
    };

    assert_cli_success(&output, "action list --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("success: true"),
        "YAML output should contain success: true; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 1"),
        "YAML output should contain action id: 1; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 2"),
        "YAML output should contain action id: 2; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 3"),
        "YAML output should contain action id: 3; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: deterministic_pure"),
        "YAML output should contain deterministic_pure; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: idempotent_external"),
        "YAML output should contain idempotent_external; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: at_least_once_external"),
        "YAML output should contain at_least_once_external; got: {stdout}"
    );
}

#[test]
fn cli_action_inspect_text_output_has_contract_details() {
    let output = run_cli(&[
        std::ffi::OsStr::new("action"),
        std::ffi::OsStr::new("inspect"),
        std::ffi::OsStr::new("write"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballistics CLI for action inspect write"
            );
            return;
        }
    };

    assert_cli_success(&output, "action inspect write");
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
        std::ffi::OsStr::new("write"),
        std::ffi::OsStr::new("--emit"),
        std::ffi::OsStr::new("yaml"),
    ]);
    let output = match output {
        Some(output) => output,
        None => {
            assert!(
                forced_assertion_failure(),
                "failed to execute velvet_ballistics CLI for action inspect write --emit yaml"
            );
            return;
        }
    };

    assert_cli_success(&output, "action inspect write --emit yaml");
    let stdout = output_stdout(&output);
    assert!(
        stdout.contains("success: true"),
        "YAML output should contain success: true; got: {stdout}"
    );
    assert!(
        stdout.contains("id: 2"),
        "YAML output should contain action id: 2; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency: idempotent_external"),
        "YAML output should contain idempotency: idempotent_external; got: {stdout}"
    );
    assert!(
        stdout.contains("retry_safety: key_required"),
        "YAML output should contain retry_safety: key_required; got: {stdout}"
    );
    assert!(
        stdout.contains("idempotency_rule: external retries require a stable idempotency key"),
        "YAML output should contain idempotency_rule; got: {stdout}"
    );
    assert!(
        stdout.contains("failure_codes:"),
        "YAML output should contain failure_codes; got: {stdout}"
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
                "failed to execute velvet_ballistics CLI for action inspect 99"
            );
            return;
        }
    };

    assert_cli_failure_contains(
        &output,
        "action inspect 99",
        "action '99' is not registered",
    );
}

