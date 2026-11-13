use super::*;

#[test]
fn control_whitespace_manifest_padding_and_empty_raw_logs() {
    let fixture = setup_test("control-whitespace-manifest-and-log");
    let response = children(&fixture);
    let raw = format!("{}/raw.log", fixture.dir);
    let row = make_row(&fixture, "cargo test", &raw);
    let manifest = write_manifest(&fixture.dir, &[row.clone()]);
    for blank in ["\u{1c}", "\u{1d}", "\u{1e}", "\u{1f}"] {
        write_file(&manifest, &format!("{blank}{row}{blank}\n"));
        write_file(&raw, &log_text(&fixture, "cargo test", "\n"));
        assert_eq!(
            run_checker(&fixture, &response, &manifest),
            (
                0,
                "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children=1\n".into(),
                String::new()
            )
        );
        write_file(&raw, &format!("\t{blank}\n"));
        assert_eq!(
            run_checker(&fixture, &response, &manifest),
            (
                1,
                format!(
                    "VB_JPQ7_CLOSURE_EVIDENCE_FAIL\n- manifest row 1: raw_log_path is empty {raw}\n"
                ),
                String::new()
            )
        );
    }
    cleanup_test(fixture);
}

#[test]
fn control_whitespace_required_fields_are_missing() {
    let fixture = setup_test("control-whitespace-fields");
    let response = children(&fixture);
    let raw = format!("{}/raw.log", fixture.dir);
    write_file(&raw, &log_text(&fixture, "cargo test", "\n"));
    let baseline = make_row(&fixture, "cargo test", &raw);
    let manifest = write_manifest(&fixture.dir, &[baseline.clone()]);
    assert_eq!(
        run_checker(&fixture, &response, &manifest),
        (
            0,
            "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children=1\n".into(),
            String::new()
        )
    );
    for blank in ["\u{1c}", "\u{1d}", "\u{1e}", "\u{1f}"] {
        for field in [
            "bead_id",
            "command",
            "cwd",
            "commit_sha",
            "tool_version",
            "timestamp",
            "raw_log_path",
            "stdout_summary",
            "stderr_summary",
            "status",
            "evidence_kind",
        ] {
            let mut row = baseline.clone();
            row[field] = serde_json::json!(blank);
            let manifest = write_manifest(&fixture.dir, &[row]);
            let (code, stdout, stderr) = run_checker(&fixture, &response, &manifest);
            assert_eq!(code, 1, "field={field} blank={blank:?} stdout={stdout}");
            assert_eq!(stderr, "");
            let expected = format!("- manifest row 1: missing required field {field}");
            assert!(
                stdout.lines().any(|line| line == expected),
                "missing cause for field={field} blank={blank:?}: {stdout}"
            );
        }
    }
    cleanup_test(fixture);
}

#[test]
fn control_whitespace_cannot_link_nonzero_rows() {
    let fixture = setup_test("control-whitespace-resolution");
    let response = children(&fixture);
    let raw = format!("{}/raw.log", fixture.dir);
    write_file(
        &raw,
        &log_text(&fixture, "cargo test", "\n").replace("exit_code: 0", "exit_code: 1"),
    );
    for (kind, field, diagnostic) in [
        (
            "split_followup",
            "resolution_rationale",
            "nonzero row lacks resolution_rationale",
        ),
        (
            "approved_waiver",
            "resolution_rationale",
            "nonzero row lacks resolution_rationale",
        ),
        (
            "split_followup",
            "split_bead_id",
            "split_followup lacks split_bead_id",
        ),
        (
            "approved_waiver",
            "waiver_id",
            "approved_waiver lacks waiver_id",
        ),
    ] {
        for blank in ["\u{1c}", "\u{1d}", "\u{1e}", "\u{1f}"] {
            let mut row = make_row(&fixture, "cargo test", &raw);
            row["exit_code"] = serde_json::json!(1);
            row["status"] = serde_json::json!("FAIL");
            row["resolution_kind"] = serde_json::json!(kind);
            row["resolution_rationale"] = serde_json::json!("explicit followup");
            row["split_bead_id"] = serde_json::json!("vb-followup");
            row["waiver_id"] = serde_json::json!("W-1");
            let manifest = write_manifest(&fixture.dir, &[row.clone()]);
            assert_eq!(
                run_checker(&fixture, &response, &manifest),
                (
                    0,
                    "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children=1\n".into(),
                    String::new()
                )
            );
            row[field] = serde_json::json!(blank);
            let manifest = write_manifest(&fixture.dir, &[row]);
            assert_eq!(
                run_checker(&fixture, &response, &manifest),
                (
                    1,
                    format!(
                        "VB_JPQ7_CLOSURE_EVIDENCE_FAIL\n- manifest row 1: {diagnostic}\n- vb-jpq7.1: no passing closure row or waiver/split-linked failure row\n"
                    ),
                    String::new()
                ),
                "field={field} kind={kind} blank={blank:?}"
            );
        }
    }
    cleanup_test(fixture);
}

#[test]
fn control_whitespace_in_bd_stderr_is_compacted() {
    let fixture = setup_test("control-whitespace-stderr");
    write_file(
        &format!("{}/bd", fixture.bin_dir),
        "#!/bin/sh\nprintf 'a\\034b\\035c\\036d\\037e' >&2\nexit 1\n",
    );
    let output = process::Command::new(env!("CARGO_BIN_EXE_closure-evidence"))
        .arg("--bd-workdir")
        .arg(&fixture.dir)
        .env("PATH", &fixture.bin_dir)
        .output()
        .expect("run checker");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr UTF-8"),
        "CHECK FAILED: bd list failed exit=1 stderr=a b c d e\n"
    );
    cleanup_test(fixture);
}

fn make_row(fixture: &TestFixture, command: &str, raw: &str) -> serde_json::Value {
    serde_json::json!({
        "bead_id": "vb-jpq7.1", "command": command, "cwd": fixture.dir,
        "commit_sha": "abcdef1", "tool_version": "cargo 1.91.0-nightly",
        "timestamp": "2026-01-01T00:00:00Z", "raw_log_path": raw,
        "stdout_summary": "1 passed", "stderr_summary": "empty",
        "exit_code": 0, "status": "PASS", "evidence_kind": "raw-command"
    })
}

fn log_text(fixture: &TestFixture, command: &str, separator: &str) -> String {
    format!(
        "command: {command}{separator}cwd: {}{separator}timestamp: 2026-01-01T00:00:00Z{separator}exit_code: 0",
        fixture.dir
    )
}

fn children(fixture: &TestFixture) -> String {
    write_bd_response(
        &fixture.dir,
        r#"[{"id":"vb-jpq7.1","status":"closed","close_reason":"done"}]"#,
    )
}

#[test]
fn raw_provenance_rejects_prefixes_and_embedded_labels() {
    let fixture = setup_test("provenance-boundaries");
    let response = children(&fixture);
    let raw = format!("{}/raw.log", fixture.dir);
    let row = make_row(&fixture, "cargo test", &raw);
    let manifest = write_manifest(&fixture.dir, &[row]);
    let text = log_text(&fixture, "cargo test", "\n");
    write_file(&raw, &text);
    assert_eq!(
        run_checker(&fixture, &response, &manifest),
        (
            0,
            "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children=1\n".into(),
            String::new()
        )
    );
    for binding in [
        "command: cargo test".to_string(),
        format!("cwd: {}", fixture.dir),
        "timestamp: 2026-01-01T00:00:00Z".to_string(),
        "exit_code: 0".to_string(),
    ] {
        for spoof in [format!("{binding}-suffix"), format!("embedded {binding}")] {
            write_file(&raw, &text.replacen(&binding, &spoof, 1));
            assert_eq!(
                run_checker(&fixture, &response, &manifest),
                (
                    1,
                    format!(
                        "VB_JPQ7_CLOSURE_EVIDENCE_FAIL\n- manifest row 1: raw_log_path lacks provenance binding {binding}\n"
                    ),
                    String::new()
                ),
                "spoof: {spoof}"
            );
        }
    }
    cleanup_test(fixture);
}

#[test]
fn exact_provenance_retains_line_endings_eof_and_multiline_values() {
    let fixture = setup_test("provenance-valid-boundaries");
    let response = children(&fixture);
    let raw = format!("{}/raw.log", fixture.dir);
    for command in ["cargo test", "cargo test\n  --lib"] {
        let row = make_row(&fixture, command, &raw);
        let manifest = write_manifest(&fixture.dir, &[row]);
        for separator in ["\n", "\r\n", "\r"] {
            write_file(&raw, &log_text(&fixture, command, separator));
            assert_eq!(
                run_checker(&fixture, &response, &manifest),
                (
                    0,
                    "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children=1\n".into(),
                    String::new()
                ),
                "command={command:?} separator={separator:?}"
            );
        }
    }
    cleanup_test(fixture);
}

#[test]
fn canonical_commented_provenance_headers_bind_exact_values() {
    let fixture = setup_test("provenance-comment-headers");
    let response = children(&fixture);
    let raw = format!("{}/raw.log", fixture.dir);
    for command in ["cargo test", "cargo test\n  --lib"] {
        let row = make_row(&fixture, command, &raw);
        let manifest = write_manifest(&fixture.dir, &[row]);
        for separator in ["\n", "\r\n", "\r"] {
            let text = format!(
                "# raw evidence{separator}# provenance binding command: {command}{separator}# provenance binding cwd: {}{separator}# provenance binding timestamp: 2026-01-01T00:00:00Z{separator}# provenance binding exit_code: 0",
                fixture.dir
            );
            write_file(&raw, &text);
            assert_eq!(
                run_checker(&fixture, &response, &manifest),
                (
                    0,
                    "VB_JPQ7_CLOSURE_EVIDENCE_PASS closed_children=1\n".into(),
                    String::new()
                ),
                "command={command:?} separator={separator:?}"
            );
            for binding in [
                format!("command: {command}"),
                format!("cwd: {}", fixture.dir),
                "timestamp: 2026-01-01T00:00:00Z".to_string(),
                "exit_code: 0".to_string(),
            ] {
                let header = format!("# provenance binding {binding}");
                for spoof in [format!("embedded {header}"), format!("{header}-suffix")] {
                    write_file(&raw, &text.replacen(&header, &spoof, 1));
                    assert_eq!(
                        run_checker(&fixture, &response, &manifest),
                        (
                            1,
                            format!(
                                "VB_JPQ7_CLOSURE_EVIDENCE_FAIL\n- manifest row 1: raw_log_path lacks provenance binding {binding}\n"
                            ),
                            String::new()
                        ),
                        "spoof={spoof:?} separator={separator:?}"
                    );
                }
            }
        }
    }
    cleanup_test(fixture);
}
