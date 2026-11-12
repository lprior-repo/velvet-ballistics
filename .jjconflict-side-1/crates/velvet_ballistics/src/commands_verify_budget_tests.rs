use super::*;

fn real_budget_error() -> Result<vb_core::WorkflowError, String> {
    let nodes: Vec<vb_core::CompiledNode> = vec![];
    let entry = vb_core::StepIdx::new(0);
    let contract = vb_core::ResourceContract::DEFAULT;
    match vb_core::budget::WholeWorkflowBudget::compute(&nodes, entry, &contract) {
        Err(e) => {
            assert_eq!(
                e,
                vb_core::WorkflowError::EntryOutOfBounds {
                    entry: vb_core::StepIdx::new(0)
                }
            );
            Ok(e)
        }
        Ok(_) => Err("expected budget computation to fail".to_string()),
    }
}

#[test]
fn budget_error_fatal_in_full_profile() -> Result<(), String> {
    let error = real_budget_error()?;
    let mut checks = vec!["yaml_parse", "compilation", "ir_validation"];
    let mut warnings: Vec<String> = vec![];
    let result = record_budget_result(VerifyProfile::Full, Err(error), &mut checks, &mut warnings);
    assert_eq!(checks, vec!["yaml_parse", "compilation", "ir_validation"]);
    match result {
        Err(VerifyError::BudgetPolicy(message)) => {
            let expected = format!(
                "budget computation failed: {}",
                vb_core::WorkflowError::EntryOutOfBounds {
                    entry: vb_core::StepIdx::new(0)
                }
            );
            if message != expected {
                return Err(format!("unexpected message: {}", message));
            }
            let exit = exit_code_for_error(&VerifyError::BudgetPolicy(message));
            if u8::from(exit) != 4 {
                return Err(format!("unexpected exit code: {}", u8::from(exit)));
            }
            if !warnings.is_empty() {
                return Err("warnings should be empty".to_string());
            }
            Ok(())
        }
        _ => Err("unexpected result".to_owned()),
    }
}

#[test]
fn budget_error_warning_in_standard_profile() -> Result<(), String> {
    let error = real_budget_error()?;
    let mut checks = vec!["yaml_parse", "compilation", "ir_validation"];
    let mut warnings: Vec<String> = vec![];
    let result = record_budget_result(
        VerifyProfile::Standard,
        Err(error),
        &mut checks,
        &mut warnings,
    );
    assert_eq!(checks, vec!["yaml_parse", "compilation", "ir_validation"]);
    match result {
        Ok(()) => {
            if warnings.len() != 1 {
                return Err(format!("expected 1 warning, got {}", warnings.len()));
            }
            let expected = format!(
                "budget computation note: {}",
                vb_core::WorkflowError::EntryOutOfBounds {
                    entry: vb_core::StepIdx::new(0)
                }
            );
            if let Some(w) = warnings.first() {
                if *w != expected {
                    return Err(format!("unexpected warning: {}", w));
                }
            } else {
                return Err("no warning found".to_string());
            }
            Ok(())
        }
        _ => Err("unexpected result".to_owned()),
    }
}

#[test]
fn full_verification_pipeline_succeeds() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: budget_positive\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n";
    let bytes = yaml.as_bytes();
    for profile in [
        VerifyProfile::Quick,
        VerifyProfile::Standard,
        VerifyProfile::Full,
    ] {
        match run_verification(yaml, bytes, profile) {
            Ok(result) => {
                if profile == VerifyProfile::Quick {
                    let expected = vec!["yaml_parse", "compilation", "ir_validation"];
                    if result.checks != expected {
                        return Err(format!(
                            "Quick profile checks mismatch: {:?}",
                            result.checks
                        ));
                    }
                } else {
                    let expected = vec![
                        "yaml_parse",
                        "compilation",
                        "ir_validation",
                        "budget_computation",
                        "boundedness_policy",
                    ];
                    if result.checks != expected {
                        return Err(format!("profile checks mismatch: {:?}", result.checks));
                    }
                }
                if !result.warnings.is_empty() {
                    return Err(format!("unexpected warnings: {:?}", result.warnings));
                }
            }
            Err(VerifyError::YamlParse(msg)) => return Err(format!("yaml parse failed: {}", msg)),
            Err(VerifyError::Compile(msgs)) => return Err(format!("compile failed: {:?}", msgs)),
            Err(VerifyError::IrValidation(msg)) => {
                return Err(format!("ir validation failed: {}", msg));
            }
            Err(VerifyError::BudgetPolicy(msg)) => {
                return Err(format!("budget policy failed: {}", msg));
            }
            Err(VerifyError::StorageError(msg)) => return Err(format!("storage error: {}", msg)),
            Err(VerifyError::ReplayDivergence(msg)) => {
                return Err(format!("replay divergence: {}", msg));
            }
        }
    }
    Ok(())
}
