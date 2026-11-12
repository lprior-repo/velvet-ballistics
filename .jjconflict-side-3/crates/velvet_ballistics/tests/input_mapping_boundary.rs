use std::fs;
use std::process::Command;

const YAML_STRING: &str = "version: velvet-ballistics/v1\nname: input_boundary\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n";

#[test]
fn cli_zero_port_input_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let yaml_path = dir.path().join("workflow.yaml");
    let ir_path = dir.path().join("workflow.ir");
    let input_path = dir.path().join("input.bin");

    fs::write(&yaml_path, YAML_STRING)?;

    let compiled = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("compile")
        .arg(&yaml_path)
        .args(["--emit", "postcard", "--out"])
        .arg(&ir_path)
        .output()?;
    assert_eq!(
        compiled.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let encoded_empty = postcard::to_allocvec(&Box::<[vb_core::SlotValue]>::from([]))?;
    let one_value = postcard::to_allocvec(&Box::<[vb_core::SlotValue]>::from([
        vb_core::SlotValue::I64(42),
    ]))?;

    let cases: [(Vec<u8>, Option<&str>); 5] = [
        (Vec::new(), None),
        (encoded_empty.clone(), None),
        (
            vec![255],
            Some("INPUT_MAPPING_FAILED: input-bin decode failed"),
        ),
        (
            one_value,
            Some("INPUT_MAPPING_FAILED: input slot count exceeds workflow slot count"),
        ),
        (
            encoded_empty.clone().into_iter().chain(vec![255]).collect(),
            Some("INPUT_MAPPING_FAILED: input-bin decode failed"),
        ),
    ];

    for (data, expected) in cases {
        fs::write(&input_path, &data)?;
        for (command, path, failure_code) in [("run", &yaml_path, 1), ("run-compiled", &ir_path, 3)]
        {
            let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
                .arg(command)
                .arg(path)
                .arg("--input-bin")
                .arg(&input_path)
                .args(["--durability", "none"])
                .output()?;

            if let Some(expected_msg) = expected {
                assert_eq!(
                    output.status.code(),
                    Some(failure_code),
                    "{command}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), expected_msg);
            } else {
                assert_eq!(output.status.code(), Some(0));
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(
                    stdout
                        .lines()
                        .any(|line| line == "run 1: submitted=1 completed=1 failed=0 steps=2"),
                    "{stdout}"
                );
                assert!(stdout.ends_with("run completed\n"), "{stdout}");
                assert!(output.stderr.is_empty());
            }
        }
    }

    Ok(())
}
