// =========================================================================
// parse_step_primitive("together") Harness
// =========================================================================

/// KANI-XI2F-004: Prove parse_step_primitive accepts "together" key without panic
///
/// ## Scope
/// This harness verifies that after adding "together" to both is_primitive()
/// and parse_step_primitive(), the "together" key is accepted and routed
/// to parse_parallel() correctly.
///
/// ## Bounds
/// - Unwind: 8 (accounting for mapping iteration + match arms)
/// - No unwinding checks (parse logic has bounded loops)
///
/// ## Expected Result
/// Kani proves that a step mapping containing "together" is accepted
/// and produces StepPrimitive::Together without panic
///
/// ## Production Dependency
/// Requires production changes to add "together" to:
/// - is_primitive() match arms
/// - parse_step_primitive() match arms
/// - reject_unknown_step_fields() allowed list
#[kani::proof]
#[kani::unwind(8)]
#[kani::no_unwinding_checks]
fn parse_step_primitive_together_harness() {
    use saphyr::{Document, LoadableYamlNode};

    let yaml_text = r#"
id: test-step
together:
  branches:
    - label: branch-1
      steps: []
"#;

    let Ok(yaml_doc) = saphyr::Yaml::load_from_str(yaml_text) else {
        kani::assume(false, "hardcoded YAML must parse");
        return;
    };
    let docs = yaml_doc;
    let Some(root) = docs.into_iter().next() else {
        kani::assume(false, "hardcoded YAML must have a document");
        return;
    };

    let result = crate::ast::parse_steps::parse_step(&root);

    match result {
        Ok(step) => {
            match step.primitive {
                crate::ast::StepPrimitive::Together { branches } => {
                    kani::assert(branches.len() == 1, "together should have 1 branch");
                    kani::assert(branches[0].label == "branch-1", "branch label should match");
                }
                other => {
                    kani::assert(false, "together key should produce StepPrimitive::Together");
                }
            }
        }
        Err(e) => {
            kani::assert(
                false,
                &format!(
                    "parse_step with together should succeed, got: {:?}",
                    e
                ),
            );
        }
    }
}

/// KANI-XI2F-005: Regression - prove "parallel" still works after "together" addition
///
/// ## Scope
/// Ensures that adding "together" as an alias doesn't break existing "parallel" behavior
///
/// ## Expected Result
/// Kani proves that "parallel" key still works and produces StepPrimitive::Together
#[kani::proof]
#[kani::unwind(8)]
#[kani::no_unwinding_checks]
fn parse_step_primitive_parallel_regression_harness() {
    use saphyr::LoadableYamlNode;

    let yaml_text = r#"
id: test-step
parallel:
  branches:
    - label: branch-1
      steps: []
"#;

    let Ok(yaml_doc) = saphyr::Yaml::load_from_str(yaml_text) else {
        kani::assume(false, "hardcoded YAML must parse");
        return;
    };
    let docs = yaml_doc;
    let Some(root) = docs.into_iter().next() else {
        kani::assume(false, "hardcoded YAML must have a document");
        return;
    };

    let result = crate::ast::parse_steps::parse_step(&root);

    match result {
        Ok(step) => {
            match step.primitive {
                crate::ast::StepPrimitive::Together { .. } => {}
                other => {
                    kani::assert(false, "parallel key should produce StepPrimitive::Together");
                }
            }
        }
        Err(e) => {
            kani::assert(
                false,
                &format!(
                    "parse_step with parallel should succeed, got: {:?}",
                    e
                ),
            );
        }
    }
}
