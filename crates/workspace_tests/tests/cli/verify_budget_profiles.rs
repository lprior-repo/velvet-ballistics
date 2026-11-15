use std::fs;
use std::process::Command;

fn run_verify(
    path: &str,
    profile: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(["verify", path, "--profile", profile, "--emit", "yaml"])
        .output()?;
    Ok(output)
}

#[test]
fn valid_workflow_all_profiles() -> Result<(), Box<dyn std::error::Error>> {
    let positive_yaml = r#"version: velvet-ballistics/v1
name: budget_positive
when:
  manual: {}
steps:
  - id: build_result
    save:
      output: saved
      value: '42'
  - id: done
    finish:
      result: saved
"#;

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("positive.yaml");
    fs::write(&path, positive_yaml)?;

    let profiles = ["quick", "standard", "full"];
    for profile in profiles {
        let output = run_verify(path.to_str().ok_or("path")?, profile)?;
        assert_eq!(
            output.status.code(),
            Some(0),
            "profile {} exit code",
            profile
        );

        let doc: serde_json::Value = serde_saphyr::from_str(std::str::from_utf8(&output.stdout)?)?;
        let success = doc.get("success").and_then(serde_json::Value::as_bool);
        assert_eq!(success, Some(true), "profile {} success", profile);

        let actual_profile = doc.get("profile").and_then(serde_json::Value::as_str);
        assert_eq!(actual_profile, Some(profile), "profile {} actual", profile);

        let warnings = doc.get("warnings").ok_or("missing warnings")?;
        assert!(warnings.is_array(), "profile {} warnings array", profile);
        assert_eq!(
            warnings.as_array().ok_or("warnings array")?.len(),
            0,
            "profile {} warnings empty",
            profile
        );

        let checks_value = doc.get("checks").ok_or("missing checks")?;
        let checks: Vec<String> = serde_json::from_value(checks_value.clone())?;

        if profile == "quick" {
            assert_eq!(checks, vec!["yaml_parse", "compilation", "ir_validation"]);
        } else {
            assert_eq!(
                checks,
                vec![
                    "yaml_parse",
                    "compilation",
                    "ir_validation",
                    "budget_computation",
                    "boundedness_policy"
                ]
            );
        }
    }

    Ok(())
}

#[test]
fn overflow_fixture_all_profiles() -> Result<(), Box<dyn std::error::Error>> {
    let overflow_yaml = r#"version: velvet-ballistics/v1
name: budget_overflow
when:
  manual: {}
inputs:
  items:
    is: list
    of: any
    max: 65535
steps:
  - id: seed
    save:
      output: seeded
      value: '1'
  - id: loop
    for_each:
      variable: entry
      input: $input.items
      at_once: 1
      steps:
        - id: inner
          for_each:
            variable: inner_entry
            input: $input.items
            at_once: 1
            steps:
              - id: body
                save:
                  output: seen
                  value: '1'
  - id: done
    finish:
      result: seeded
"#;

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("overflow.yaml");
    fs::write(&path, overflow_yaml)?;

    let profiles = ["quick", "standard", "full"];
    for profile in profiles {
        let output = run_verify(path.to_str().ok_or("path")?, profile)?;
        assert_eq!(
            output.status.code(),
            Some(2),
            "profile {} exit code",
            profile
        );

        assert!(output.stdout.is_empty());
        let doc: serde_json::Value = serde_saphyr::from_str(std::str::from_utf8(&output.stderr)?)?;
        let success = doc.get("success").and_then(serde_json::Value::as_bool);
        assert_eq!(success, Some(false), "profile {} success", profile);

        let message = doc.get("message").and_then(serde_json::Value::as_str);
        assert!(message.is_some(), "profile {} message", profile);
        let msg = message.ok_or("message")?;
        assert!(
            msg.contains("budget step count overflow: 12884901891 cannot be represented"),
            "profile {} message content",
            profile
        );
    }

    Ok(())
}

#[test]
fn verify_ir_digest_matches_emitted_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("digest.yaml");
    let ir_path = dir.path().join("workflow.ir");
    let mut previous = None;
    for value in ["42", "43"] {
        let yaml = format!(
            "version: velvet-ballistics/v1\nname: digest_identity\nwhen:\n  manual: {{}}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '{value}'\n  - id: done\n    finish:\n      result: saved\n"
        );
        fs::write(&path, &yaml)?;
        let compiled = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg("compile")
            .arg(&path)
            .args(["--emit", "ir", "--out"])
            .arg(&ir_path)
            .output()?;
        assert_eq!(compiled.status.code(), Some(0), "{compiled:?}");
        let bytes = fs::read(&ir_path)?;
        let expected_ir = blake3::hash(&bytes).to_hex().to_string();
        let workflow = vb_compile::compile_workflow(yaml.as_bytes())?;
        let expected_workflow: String = workflow
            .digest()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_ne!(expected_ir, expected_workflow);
        for profile in ["quick", "standard", "full"] {
            let output = run_verify(path.to_str().ok_or("path")?, profile)?;
            assert_eq!(output.status.code(), Some(0), "{output:?}");
            let report: serde_json::Value =
                serde_saphyr::from_str(std::str::from_utf8(&output.stdout)?)?;
            assert_eq!(
                report
                    .pointer("/artifact/ir_digest_hex")
                    .and_then(serde_json::Value::as_str),
                Some(expected_ir.as_str())
            );
            assert_eq!(
                report.get("digest").and_then(serde_json::Value::as_str),
                Some(expected_workflow.as_str())
            );
            assert_eq!(
                report
                    .pointer("/artifact/source_digest_hex")
                    .and_then(serde_json::Value::as_str),
                Some(expected_workflow.as_str())
            );
            assert_eq!(
                report.get("profile").and_then(serde_json::Value::as_str),
                Some(profile)
            );
        }
        if let Some(previous) = previous {
            assert_ne!(expected_ir, previous);
        }
        previous = Some(expected_ir);
    }
    Ok(())
}
