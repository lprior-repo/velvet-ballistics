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
    max: 4294967295
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
            msg.contains("budget step count overflow: 8589934593 cannot be represented"),
            "profile {} message content",
            profile
        );
    }

    Ok(())
}
