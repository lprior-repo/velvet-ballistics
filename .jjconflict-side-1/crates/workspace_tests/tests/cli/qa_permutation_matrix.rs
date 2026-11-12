//! vb-9y985 permutation matrix: every YAML command × malformed classes,
//! flag permutations, path/encoding permutations, and QA regression cases.
//! Loop-generated cases; each iteration is an asserted case.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

const VALID: &str = "version: velvet-ballistics/v1\nname: minimal_workflow\nwhen:\n  manual: {}\nsteps:\n  - id: step_hello\n    set:\n      output: greeting\n      value: \"1\"\n  - id: done\n    finish:\n      result: greeting\n";

fn cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(args)
        .output()
        .expect("failed to spawn velvet-ballistics");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn write(dir: &tempfile::TempDir, name: &str, content: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("write fixture");
    path
}

fn malformed_cases() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("empty", b"".to_vec()),
        ("whitespace", b"  \n\n".to_vec()),
        ("lone_marker", b"---\n".to_vec()),
        ("truncated", b"version: velvet-ballistics/v1\nname: x\nwhen:\n  manu".to_vec()),
        ("unclosed_quote", b"version: \"velvet-ballistics/v1\nname: x\n".to_vec()),
        ("unclosed_bracket", b"version: velvet-ballistics/v1\nsteps: [a, b\n".to_vec()),
        ("bad_indent", b"version: velvet-ballistics/v1\n name: x\n".to_vec()),
        ("tabs", b"version: velvet-ballistics/v1\n\tname: x\n".to_vec()),
        ("invalid_escape", b"version: \"a\\q\"\n".to_vec()),
        ("binary", (0u8..=255).collect()),
        ("nul", b"version: a\x00b\n".to_vec()),
        ("multidoc", b"a: 1\n---\nb: 2\n".to_vec()),
        ("custom_tag", b"version: !mytag x\n".to_vec()),
        ("dup_keys", b"version: a\nversion: b\n".to_vec()),
        ("directive", b"%YAML 9.9\n---\nversion: velvet-ballistics/v1\nname: x\nwhen:\n  manual: {}\nsteps:\n  - id: d\n    finish: {result: 0}\n".to_vec()),
        ("merge_key", b"a:\n  <<: {b: 1}\n".to_vec()),
        ("anchor", b"a: &x 1\nb: *x\n".to_vec()),
        ("ambiguous_yes", b"version: velvet-ballistics/v1\nname: yes\n".to_vec()),
        ("deep_flow", format!("{}: 1\n", "[".repeat(500)).into_bytes()),
        ("huge_scalar", format!("a: {}\n", "x".repeat(200_000)).into_bytes()),
        ("unknown_action", "version: velvet-ballistics/v1\nname: t\nwhen:\n  manual: {}\nsteps:\n  - id: a\n    set: {output: x, value: '1'}\n  - id: act\n    do: {action: '9999', input: '$a'}\n  - id: done\n    finish: {result: x}\n".as_bytes().to_vec()),
        ("unknown_field", "version: velvet-ballistics/v1\nname: t\nbogus: 1\nwhen:\n  manual: {}\nsteps:\n  - id: d\n    finish: {result: 0}\n".as_bytes().to_vec()),
        ("wrong_type", "version: 123\nname: t\nwhen:\n  manual: {}\nsteps:\n  - id: d\n    finish: {result: 0}\n".as_bytes().to_vec()),
        ("missing_steps", b"version: velvet-ballistics/v1\nname: t\nwhen:\n  manual: {}\n".to_vec()),
    ]
}

#[test]
fn qa_matrix_malformed_times_commands_never_panic_or_pass() {
    let dir = tempdir().expect("tempdir");
    let mut cases = 0_usize;
    for (name, content) in malformed_cases() {
        let file = write(&dir, &format!("{name}.yaml"), &content);
        let f = file.to_str().expect("utf8 path");
        for command in [
            "validate",
            "verify",
            "explain",
            "graph",
            "simulate",
            "bench-run",
        ] {
            let (code, _out, err) = cli(&[command, f]);
            assert_ne!(code, 0, "{command} accepted malformed {name}: {err}");
            assert_ne!(code, 101, "{command} panicked on {name}: {err}");
            assert!(
                !err.contains("panicked"),
                "{command} panic text on {name}: {err}"
            );
            assert_ne!(err.len(), 0, "{command} silent on {name}");
            cases += 1;
        }
        let out = dir.path().join("out.vbir");
        let (code, _o, err) = cli(&[
            "compile",
            f,
            "--emit",
            "ir",
            "--out",
            out.to_str().expect("utf8"),
        ]);
        assert_ne!(code, 0, "compile accepted malformed {name}: {err}");
        assert_ne!(code, 101, "compile panicked on {name}: {err}");
        cases += 1;
    }
    assert!(cases >= 150, "matrix too small: {cases}");
}

#[test]
fn qa_matrix_valid_times_emit_modes_pass() {
    let dir = tempdir().expect("tempdir");
    let file = write(&dir, "valid.yaml", VALID.as_bytes());
    let f = file.to_str().expect("utf8");
    let mut cases = 0_usize;
    for command in [
        "validate",
        "verify",
        "explain",
        "graph",
        "simulate",
        "bench-run",
    ] {
        for emit in ["text", "yaml", "postcard"] {
            let (code, _out, err) = cli(&[command, f, "--emit", emit]);
            assert_eq!(code, 0, "{command} --emit {emit} failed: {err}");
            cases += 1;
        }
    }
    for profile in ["quick", "standard", "full"] {
        let (code, _o, err) = cli(&["verify", f, "--profile", profile]);
        assert_eq!(code, 0, "verify {profile}: {err}");
        cases += 1;
    }
    assert!(cases >= 20, "valid matrix too small: {cases}");
}

#[test]
fn qa_flag_permutations() {
    let dir = tempdir().expect("tempdir");
    let file = write(&dir, "valid.yaml", VALID.as_bytes());
    let f = file.to_str().expect("utf8");
    // Surplus positional rejected on every single-file command.
    for command in [
        "validate",
        "verify",
        "explain",
        "graph",
        "simulate",
        "bench-run",
    ] {
        let (code, _o, err) = cli(&[command, f, "extra.yaml"]);
        assert_eq!(code, 2, "{command} surplus positional: {err}");
        assert!(err.contains("unexpected argument"), "{command}: {err}");
    }
    // Duplicate value flags rejected.
    for args in [
        vec!["validate", f, "--emit", "text", "--emit", "yaml"],
        vec!["verify", f, "--profile", "quick", "--profile", "full"],
    ] {
        let (code, _o, err) = cli(&args);
        assert_eq!(code, 2, "duplicate flag accepted: {args:?} {err}");
        assert!(err.contains("duplicate flag"), "{err}");
    }
    // Equals form works.
    let (code, _o, err) = cli(&["validate", f, "--emit=yaml"]);
    assert_eq!(code, 0, "equals form rejected: {err}");
    // Empty path values rejected with exit 2, never panic 101.
    for args in [
        vec!["doctor", "--db", ""],
        vec!["ipc-serve", "--socket", "/tmp/x.sock", "--db", ""],
        vec!["ipc-serve", "--socket", "", "--db", "/tmp/xdb"],
    ] {
        let (code, _o, err) = cli(&args);
        assert_eq!(code, 2, "empty path accepted: {args:?} code={code} {err}");
    }
}

#[test]
fn qa_io_error_kinds_stable() {
    let dir = tempdir().expect("tempdir");
    let (code, _o, err) = cli(&["validate", "/nonexistent-qa/nope.yaml"]);
    assert_eq!(code, 2);
    assert!(err.contains("file_not_found"), "{err}");
    let adir = dir.path().join("adir");
    fs::create_dir(&adir).expect("mkdir");
    let (code, _o, err) = cli(&["validate", adir.to_str().expect("utf8")]);
    assert_eq!(code, 2);
    assert!(err.contains("is_directory"), "{err}");
}

#[test]
fn qa_bom_and_numeric_action_inspect() {
    let dir = tempdir().expect("tempdir");
    let mut bom = b"\xef\xbb\xbf".to_vec();
    bom.extend_from_slice(VALID.as_bytes());
    let file = write(&dir, "bom.yaml", &bom);
    let (code, _o, err) = cli(&["validate", file.to_str().expect("utf8")]);
    assert_eq!(code, 0, "BOM workflow rejected: {err}");
    for id in ["1", "2", "3"] {
        let (code, out, err) = cli(&["action", "inspect", id]);
        assert_eq!(code, 0, "action inspect {id}: {err}");
        assert_ne!(out.len(), 0);
    }
    let (code, _o, _e) = cli(&["action", "inspect", "9999"]);
    assert_eq!(code, 2);
}

#[test]
fn qa_unknown_action_rejected_before_runtime() {
    let dir = tempdir().expect("tempdir");
    let body = "version: velvet-ballistics/v1\nname: t\nwhen:\n  manual: {}\nsteps:\n  - id: a\n    set: {output: x, value: '1'}\n  - id: act\n    do: {action: '9999', input: '$a'}\n  - id: done\n    finish: {result: x}\n";
    let file = write(&dir, "unknown_action.yaml", body.as_bytes());
    let f = file.to_str().expect("utf8");
    for command in [
        "validate",
        "verify",
        "explain",
        "graph",
        "simulate",
        "bench-run",
    ] {
        let (code, _o, err) = cli(&[command, f]);
        assert_ne!(code, 0, "{command} accepted unknown action: {err}");
        assert!(err.contains("unknown action"), "{command}: {err}");
    }
    for id in ["1", "2", "3"] {
        let known = body.replace("'9999'", &format!("'{id}'"));
        let known_file = write(&dir, &format!("known_{id}.yaml"), known.as_bytes());
        let (code, _o, err) = cli(&["validate", known_file.to_str().expect("utf8")]);
        assert_eq!(code, 0, "registered action {id} rejected: {err}");
    }
}

#[test]
fn qa_repeat_nonterminal_run_fails() {
    let dir = tempdir().expect("tempdir");
    let body = "version: velvet-ballistics/v1\nwhen:\n  manual: {}\nname: t\nsteps:\n  - id: a\n    set: {output: x, value: \"1\"}\n  - id: r\n    repeat: {max_attempts: 65535, steps: [{id: b, set: {output: \"y1\", value: \"2\"}}]}\n  - id: done\n    finish: {result: x}\n";
    let file = write(&dir, "repeat.yaml", body.as_bytes());
    let input = write(&dir, "input.bin", b"");
    let (code, out, _err) = cli(&[
        "run",
        file.to_str().expect("utf8"),
        "--input-bin",
        input.to_str().expect("utf8"),
        "--durability",
        "none",
    ]);
    assert_ne!(code, 0, "non-terminal run reported success: {out}");
    assert!(out.contains("not terminal"), "{out}");
}

#[test]
fn qa_error_echo_truncated_and_utf16_named() {
    let dir = tempdir().expect("tempdir");
    let huge = format!(
        "version: velvet-ballistics/v1\nname: t\nwhen:\n  manual: {{}}\nsteps:\n  - id: first\n    set: {{output: x, value: '{} + 1'}}\n  - id: done\n    finish: {{result: x}}\n",
        "9".repeat(5000)
    );
    let file = write(&dir, "huge.yaml", huge.as_bytes());
    let (code, _o, err) = cli(&["validate", file.to_str().expect("utf8")]);
    assert_ne!(code, 0);
    assert!(
        err.len() < 2000,
        "error echo unbounded: {} bytes",
        err.len()
    );
    // UTF-16LE with BOM is named, not a raw UTF-8 failure.
    let mut utf16: Vec<u8> = vec![0xff, 0xfe];
    for b in VALID.as_bytes() {
        utf16.push(*b);
        utf16.push(0);
    }
    let file16 = write(&dir, "utf16.yaml", &utf16);
    let (code, _o, err) = cli(&["validate", file16.to_str().expect("utf8")]);
    assert_eq!(code, 2);
    assert!(err.contains("UTF-16"), "{err}");
}

#[test]
fn qa_failed_run_names_step() {
    let dir = tempdir().expect("tempdir");
    let body = "version: velvet-ballistics/v1\nwhen:\n  manual: {}\nname: t\nsteps:\n  - id: a\n    set: {output: x, value: \"1\"}\n  - id: r\n    choose:\n      branches: [{when: \"false\", steps: [{id: b, set: {output: \"y1\", value: \"2\"}}]}]\n  - id: done\n    finish: {result: x}\n";
    let file = write(&dir, "choose.yaml", body.as_bytes());
    let input = write(&dir, "input.bin", b"");
    let (code, _out, err) = cli(&[
        "run",
        file.to_str().expect("utf8"),
        "--input-bin",
        input.to_str().expect("utf8"),
        "--durability",
        "none",
    ]);
    assert_eq!(code, 1, "choose fallthrough run: {err}");
    assert!(err.contains("run failed at step"), "{err}");
}
