#![forbid(unsafe_code)]

use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn invoke(arguments: &[&str]) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(arguments)
        .output()?)
}

#[test]
fn public_agent_context_delivers_long_names_without_clobber_or_stage_leaks() -> TestResult {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/deliver-filename-cli-tests");
    std::fs::create_dir_all(&root)?;
    let dir = tempfile::Builder::new().tempdir_in(root)?;
    let baseline = invoke(&["agent-context"])?;
    assert_eq!(baseline.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&baseline.stdout)?;
    assert_eq!(value["kind"], "AgentContext");
    let mut expected = serde_json::to_vec(&value)?;
    expected.push(b'\n');
    for length in [250, 251, 255] {
        let path = dir.path().join("x".repeat(length));
        let target = format!("file:{}", path.to_str().ok_or("UTF-8 path")?);
        let output = invoke(&["agent-context", "--deliver", &target])?;
        assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
        assert_eq!(output.stdout, b"");
        assert_eq!(std::fs::read(&path)?, expected);
        let duplicate = invoke(&["agent-context", "--deliver", &target])?;
        assert_eq!(duplicate.status.code(), Some(2));
        assert_eq!(duplicate.stdout, b"");
        assert!(String::from_utf8(duplicate.stderr)?.contains("target already exists"));
        assert_eq!(std::fs::read(&path)?, expected);
    }
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 3);
    Ok(())
}
