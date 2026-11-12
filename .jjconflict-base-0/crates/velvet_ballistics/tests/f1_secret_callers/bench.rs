use super::support::*;

#[test]
fn bench_run_uses_declared_host_presence_and_completes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let yaml = directory.path().join("source.yaml");
    std::fs::write(
        &yaml,
        format!(
            "version: velvet-ballistics/v1\nname: benchmark_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY_A}}}\nsteps:\n  - id: done\n    finish: {{result: '$secrets.token'}}\n"
        ),
    )?;
    let output = command()
        .arg("bench-run")
        .arg(&yaml)
        .args(["--emit", "yaml"])
        .env(KEY_A, VALUE)
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stderr, b"");
    no_value(&output.stdout);
    let result: serde_json::Value = serde_saphyr::from_str(std::str::from_utf8(&output.stdout)?)?;
    assert_eq!(
        result.get("success").and_then(serde_json::Value::as_bool),
        Some(true)
    );
    let runtime = result.get("runtime").ok_or("runtime")?;
    for (field, value) in [
        ("submitted", 1),
        ("completed", 1),
        ("failed", 0),
        ("steps", 1),
    ] {
        assert_eq!(
            runtime.get(field).and_then(serde_json::Value::as_u64),
            Some(value),
            "{field}"
        );
    }
    Ok(())
}

#[test]
fn bench_run_rejects_missing_declared_host_presence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let yaml = directory.path().join("source.yaml");
    std::fs::write(
        &yaml,
        format!(
            "version: velvet-ballistics/v1\nname: benchmark_secret\nwhen: {{manual: {{}}}}\nsecrets: {{token: {KEY_A}}}\nsteps:\n  - id: done\n    finish: {{result: '$secrets.token'}}\n"
        ),
    )?;
    let output = command().arg("bench-run").arg(&yaml).output()?;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr)?,
        "runtime submit error: SECRET_UNAVAILABLE: secret requirement unavailable\n"
    );
    Ok(())
}
