use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn run_case(
    steps: usize,
    cmd: &str,
    target: &Path,
    durability: &str,
    db_path: Option<&Path>,
    input_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"));
    command.arg(cmd).arg(target);
    if let Some(db) = db_path {
        command.arg("--db").arg(db);
    }
    command
        .arg("--input-bin")
        .arg(input_path)
        .arg("--durability")
        .arg(durability);

    let output = command.output()?;

    if !output.status.success() {
        return Err(format!(
            "command failed: {:?}\nstdout: {}\nstderr: {}",
            command,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let expected_line = format!("run 1: submitted=1 completed=1 failed=0 steps={steps}");
    let actual_first = stdout.lines().next();
    if actual_first != Some(expected_line.as_str()) {
        return Err(format!(
            "unexpected first line. expected: {}\nactual: {:?}\nfull stdout: {}",
            expected_line, actual_first, stdout
        )
        .into());
    }

    if !stdout.ends_with("run completed\n") {
        return Err(format!("stdout does not end with 'run completed\\n': {}", stdout).into());
    }

    if !stderr.is_empty() {
        return Err(format!("unexpected stderr: {}", stderr).into());
    }

    Ok(())
}

#[test]
fn finish_input_execution() -> Result<(), Box<dyn std::error::Error>> {
    let base = tempdir()?;
    let base_path = base.path();

    let no_save_prefix = "";
    let save_prefix =
        "  - id: idwrite_internal\n    save:\n      output: saved\n      value: '42'\n";

    let cases = vec![(no_save_prefix, 1), (save_prefix, 2)];

    for (prefix, steps) in cases {
        let yaml = format!(
            "version: velvet-ballistics/v1\nname: finish_input_cli\nwhen:\n  manual: {{}}\ninputs:\n  count: number\nsteps:\n{prefix}  - id: done\n    finish:\n      result: $input.count\n"
        );

        let yaml_path = base_path.join(format!("flow_{}.yaml", steps));
        fs::write(&yaml_path, &yaml)?;

        let ir_path = base_path.join(format!("flow_{}.ir", steps));
        let compile_output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg("compile")
            .arg(&yaml_path)
            .args(["--emit", "postcard", "--out"])
            .arg(&ir_path)
            .output()?;

        if !compile_output.status.success() {
            return Err(format!(
                "compile failed: {}\nstderr: {}",
                String::from_utf8_lossy(&compile_output.stdout),
                String::from_utf8_lossy(&compile_output.stderr)
            )
            .into());
        }

        let input_path = base_path.join(format!("input_{}.bin", steps));
        let input: Box<[vb_core::SlotValue]> = [vb_core::SlotValue::I64(99)].into();
        let input_bytes = postcard::to_allocvec(&input)?;
        fs::write(&input_path, input_bytes)?;

        let commands = vec![("run", &yaml_path), ("run-compiled", &ir_path)];

        let durabilities = vec!["none", "journaled", "strict"];

        for (cmd, target) in commands {
            for durability in &durabilities {
                let db_path = if *durability == "none" {
                    None
                } else {
                    Some(base_path.join(format!("db_{}_{}_{}", cmd, steps, durability)))
                };

                run_case(
                    steps,
                    cmd,
                    target,
                    durability,
                    db_path.as_deref(),
                    &input_path,
                )?;
            }
        }
    }

    Ok(())
}
