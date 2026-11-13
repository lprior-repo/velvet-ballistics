use super::*;

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
