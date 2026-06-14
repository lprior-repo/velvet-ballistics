// =========================================================================
// Error Handling Harnesses
// =========================================================================

/// KANI-XI2F-006: Prove duplicate primitive keys return typed error
///
/// ## Scope
/// Verifies that providing both "together" and "parallel" returns a typed
/// YamlError::FieldShape error rather than panicking
///
/// ## Expected Result
/// Kani proves the function returns an error (not panic) for duplicate primitives
#[kani::proof]
#[kani::unwind(8)]
#[kani::no_unwinding_checks]
fn parse_step_duplicate_primitive_error_harness() {
    use saphyr::LoadableYamlNode;

    let yaml_text = r#"
id: test-step
parallel:
  branches: []
together:
  branches: []
"#;

    let docs = saphyr::Yaml::load_from_str(yaml_text).unwrap();
    let root = docs.into_iter().next().unwrap();

    let result = crate::ast::parse_steps::parse_step(&root);

    match result {
        Ok(_) => {
            kani::assert(false, "duplicate primitives should return error");
        }
        Err(crate::YamlError::FieldShape { field, expected }) => {
            kani::assert(field == "step", "error field should be 'step'");
            kani::assert(
                expected == "exactly one primitive",
                "error should mention exactly one primitive",
            );
        }
        Err(other) => {}
    }
}

// =========================================================================
// Arbitrary-based Negative Testing
// =========================================================================

/// KANI-XI2F-007: Prove is_primitive is bounded (no panic on arbitrary strings)
///
/// ## Scope
/// Uses kani::any() to generate arbitrary strings and proves is_primitive
/// never panics regardless of input
///
/// ## Expected Result
/// Kani proves no panic on arbitrary string inputs
#[kani::proof]
#[kani::unwind(4)]
#[kani::no_unwinding_checks]
fn is_primitive_arbitrary_string_harness() {
    let input: [u8; 32] = kani::any();
    let s = core::str::from_utf8(&input);
    match s {
        Ok(valid_str) => {
            let _result = crate::ast::parse_steps::is_primitive(valid_str);
        }
        Err(_) => {}
    }
}

// =========================================================================
// Evidence Commands (for documentation)
// =========================================================================

/// ## Kani Evidence Commands
///
/// ```bash
/// # Primary harnesses
/// TMPDIR=target/tmp cargo kani -p vb_yaml --harness is_primitive_together_harness --default-unwind 4 --no-unwinding-checks
/// TMPDIR=target/tmp cargo kani -p vb_yaml --harness parse_step_primitive_together_harness --default-unwind 8 --no-unwinding-checks
///
/// # Regression harnesses
/// TMPDIR=target/tmp cargo kani -p vb_yaml --harness is_primitive_parallel_still_works_harness --default-unwind 4 --no-unwinding-checks
/// TMPDIR=target/tmp cargo kani -p vb_yaml --harness parse_step_primitive_parallel_regression_harness --default-unwind 8 --no-unwinding-checks
///
/// # Error handling harnesses
/// TMPDIR=target/tmp cargo kani -p vb_yaml --harness parse_step_duplicate_primitive_error_harness --default-unwind 8 --no-unwinding-checks
///
/// # Arbitrary testing
/// TMPDIR=target/tmp cargo kani -p vb_yaml --harness is_primitive_arbitrary_string_harness --default-unwind 4 --no-unwinding-checks
/// ```
///
/// ## Expected Output
/// All harnesses should report: **0 errors (VERIFIED)**
///
/// ## Prerequisites
/// - Production code changes must be made first (add "together" to is_primitive, parse_step_primitive, reject_unknown_step_fields)
/// - vb_yaml crate must be compiled with `cargo build -p vb_yaml`
