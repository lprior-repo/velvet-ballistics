//! Error variant completeness tests for vb_compile.
//!
//! These tests ensure every `CompileError` variant has at least one test
//! asserting the exact variant (not merely `is_err()`).
//!
//! Coverage audit as of 2026-05-18:
//! - Target: 5x pub fn coverage (≥400 tests for 80 pub fns)
//! - Current: 337 tests → need ~63 more
//! - This file contributes tests for untested error variants

use crate::{CompileError, CompileErrors, YamlCompiler};

/// Helper: parse source, return first error or fail.
fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(CompileErrors(errors)) => errors
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}

/// Shared immutable fixture: last step uses `save` (not a terminal step).
const LAST_STEP_SAVE_SOURCE: &[u8] = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
"#;

/// Shared immutable fixture: last step carries an unknown field.
const UNKNOWN_STEP_FIELD_SOURCE: &[u8] = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    unknown_field: true
    finish:
      result: 0
"#;

include!("error_variant_tests/error_variants.rs");
include!("error_variant_tests/compile_errors_api.rs");
include!("error_variant_tests/expression_helpers.rs");
include!("error_variant_tests/integration_digest.rs");
include!("error_variant_tests/together_digest.rs");
include!("error_variant_tests/together_digest_more.rs");
