//! Cold-path YAML compiler boundary.
//!
//! YAML enters the system only through this crate. The hot engine consumes only
//! `vb_core::CompiledWorkflow` values built from native Rust `saphyr` parsing.

#![forbid(unsafe_code)]
// Pedantic allows: documentation-only lints that would require pervasive changes
// with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::return_self_not_must_use)]
//! ```

// NOTE: Validation deduplication with `vb_validate` (DRIFT-5)
// -----------------------------------------------
// Reference validation is shared: this crate builds a `RefTables` from its AST
// and calls `vb_validate::references::validate_single_reference` for each
// reference, avoiding duplicate validation logic.
//
// Control-flow and type/taint validation remain compile-local because they
// need structured step/target indices and AST-specific type inference that the
// standalone validator's string-based error model cannot represent. These
// modules perform the same *logical* checks as `vb_validate` but on different
// input types.

// ============================================================================
// MODULES
// ============================================================================

pub mod ast;
mod control_flow;
pub mod expression;
mod expression_bytecode;
pub mod references;
mod schema;
pub mod strict_yaml;
mod type_taint;

// New module split for architectural drift enforcement (Round 6)
mod errors;    // CompileError, CompileErrors, error codes
mod ir_emitter; // Lowering functions, SlotCompiler
mod phases;     // YAML strict profile validation
mod workflow_compile; // WorkflowBuilder, build_workflow_parts

// ============================================================================
// PUBLIC RE-EXPORTS
// ============================================================================

pub use expression_bytecode::{compile_expr_to_bytecode, compile_expr_to_bytecode_with_accessors};

// Re-export the shared validation error types from `vb_validate` so that
// downstream consumers of this crate can optionally use the standalone
// validator's error domain without depending on `vb_validate` directly.
pub use vb_validate::{ValidationError, ValidationResult};

// Re-export error types
pub use errors::{
    check_idempotency_gates, validate_public_name, CompileError, CompileErrors,
};

// Re-export IR emitter types and functions
pub use ir_emitter::{
    compile_to_generated_rust, compute_compiled_digest,
    emit_compiled_artifact, lower_ask, lower_do, lower_finish, lower_for_each, lower_reduce,
    lower_repeat, lower_set, lower_together, lower_wait, SlotCompiler, validate_ir,
};

// Re-export phases functions
pub use phases::{checked_utf8, reject_duplicate_mapping_keys, single_document, validate_strict_profile};

// Re-export workflow compile functions
pub use workflow_compile::{
    build_accessor_table, build_constant_pool, build_slot_layout, build_workflow_parts,
    compile_workflow, compile_workflow_with_contracts, WORKFLOW_VERSION,
};

// Re-export error helpers for use by other modules
pub use errors::non_string_key_error;

// ============================================================================
// IMPORTS
// ============================================================================

use saphyr::{LoadableYamlNode, Yaml};
use vb_core::{
    AccessorProgram, ActionContract, CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue,
    ResourceContract, WorkflowDigest, WorkflowParts,
};

// ============================================================================
// CONSTANTS
// ============================================================================

const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_DEPTH: u16 = 64;
const DEFAULT_MAX_NODES: u32 = 100_000;
const DEFAULT_MAX_SEQUENCE_LEN: usize = 10_000;
const DEFAULT_MAX_MAPPING_ENTRIES: usize = 1_024;
const DEFAULT_MAX_SCALAR_BYTES: usize = 65_536;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Strict YAML resource limits for cold compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    /// Maximum workflow source size in bytes.
    pub max_source_bytes: usize,
    /// Maximum YAML nesting depth.
    pub max_depth: u16,
    /// Maximum total YAML nodes visited by validation.
    pub max_nodes: u32,
    /// Maximum sequence length.
    pub max_sequence_len: usize,
    /// Maximum mapping entry count.
    pub max_mapping_entries: usize,
    /// Maximum UTF-8 scalar length in bytes.
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_depth: DEFAULT_MAX_DEPTH,
            max_nodes: DEFAULT_MAX_NODES,
            max_sequence_len: DEFAULT_MAX_SEQUENCE_LEN,
            max_mapping_entries: DEFAULT_MAX_MAPPING_ENTRIES,
            max_scalar_bytes: DEFAULT_MAX_SCALAR_BYTES,
        }
    }
}

/// Cold compiler facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlCompiler {
    limits: YamlLimits,
}

/// Source location exposed by `saphyr-parser`.
///
/// `index` is the parser-provided byte offset into the UTF-8 source. `line` and
/// `column` are one-indexed parser marks. Tree-only validation paths use an
/// unavailable mark because `saphyr::Yaml` nodes do not retain marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMark {
    /// Parser-provided byte offset.
    pub index: usize,
    /// Parser-provided exclusive byte offset where the event span ends.
    pub end_index: usize,
    /// One-indexed source line.
    pub line: usize,
    /// One-indexed source column.
    pub column: usize,
    /// Whether this mark came from `saphyr-parser` event data.
    pub available: bool,
}

impl SourceMark {
    #[must_use]
    pub(crate) fn from_parser_span(span: saphyr_parser::Span) -> Self {
        Self {
            index: span.start.index(),
            end_index: span.end.index(),
            line: span.start.line(),
            column: span.start.col(),
            available: true,
        }
    }

    #[must_use]
    pub(crate) const fn unavailable() -> Self {
        Self {
            index: 0,
            end_index: 0,
            line: 0,
            column: 0,
            available: false,
        }
    }
}

impl YamlCompiler {
    /// Creates a compiler with explicit strict-profile limits.
    #[must_use]
    pub const fn new(limits: YamlLimits) -> Self {
        Self { limits }
    }

    /// Parses and validates YAML, then emits compiled workflow IR.
    pub fn compile(&self, source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
        let text = checked_utf8(source, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        strict_yaml::reject_unsupported_profile_events(text)
            .map_err(|e| CompileErrors(vec![e]))?;
        reject_duplicate_mapping_keys(text).map_err(|e| CompileErrors(vec![e]))?;
        let docs =
            Yaml::load_from_str(text).map_err(|e| CompileErrors(vec![CompileError::Parse(e)]))?;
        let doc = single_document(&docs).map_err(|e| CompileErrors(vec![e]))?;
        validate_strict_profile(doc, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        validate_workflow_document_shape(doc).map_err(|e| CompileErrors(vec![e]))?;
        schema::validate_input_schemas(doc)?;
        let ast = ast::parse_workflow_ast(text, doc).map_err(|e| CompileErrors(vec![e]))?;
        references::validate_workflow_ast(&ast)?;
        type_taint::validate_workflow_ast(&ast)?;
        control_flow::validate_workflow_ast(&ast)?;
        let parts =
            build_workflow_parts(text, doc).map_err(|e| CompileErrors(vec![e]))?;
        vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;
        let workflow = CompiledWorkflow::try_from_parts(parts)
            .map_err(|e| CompileErrors(vec![e.into()]))?;
        Ok(workflow)
    }

    /// Parses strict YAML into the cold typed AST without emitting runtime IR.
    pub fn parse_ast(&self, source: &[u8]) -> Result<ast::WorkflowAst, CompileErrors> {
        let text = checked_utf8(source, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        strict_yaml::reject_unsupported_profile_events(text)
            .map_err(|e| CompileErrors(vec![e]))?;
        reject_duplicate_mapping_keys(text).map_err(|e| CompileErrors(vec![e]))?;
        let docs =
            Yaml::load_from_str(text).map_err(|e| CompileErrors(vec![CompileError::Parse(e)]))?;
        let doc = single_document(&docs).map_err(|e| CompileErrors(vec![e]))?;
        validate_strict_profile(doc, self.limits).map_err(|e| CompileErrors(vec![e]))?;
        validate_workflow_document_shape(doc).map_err(|e| CompileErrors(vec![e]))?;
        schema::validate_input_schemas(doc)?;
        let ast = ast::parse_workflow_ast(text, doc).map_err(|e| CompileErrors(vec![e]))?;
        references::validate_workflow_ast(&ast)?;
        type_taint::validate_workflow_ast(&ast)?;
        control_flow::validate_workflow_ast(&ast)?;
        Ok(ast)
    }
}

impl Default for YamlCompiler {
    fn default() -> Self {
        Self::new(YamlLimits::default())
    }
}

// Helper functions that delegate to workflow_compile module
fn validate_workflow_document_shape(doc: &Yaml<'_>) -> Result<(), CompileError> {
    workflow_compile::validate_workflow_document_shape(doc)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
#[allow(clippy::panic_in_result_fn)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ExprProgram, WorkflowParts};
    use vb_core::ConstValue;
    use vb_core::{CompiledWorkflow, ResourceContract};

    macro_rules! compile_test_fail {
        ($($arg:tt)*) => {{
            let failed = false;
            assert!(failed, $($arg)*);
            return;
        }};
    }

    const NESTED_SAVE_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: nested_save
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
      tags:
        - demo
        - fast
      metadata:
        attempts: 1
        active: true
        note: null
  - id: done
    finish:
      result: 0
"#;

    const OPTIONAL_TOP_LEVEL_FIELDS_SOURCE: &[u8] = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
inputs:
  value: text
vars:
  label: 1
secrets:
  api_key: API_KEY
result: {}
examples:
  - name: fixture
    input:
      value: 1
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    fn compile_with_inputs(inputs: &str) -> Result<CompiledWorkflow, CompileErrors> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        YamlCompiler::default().compile(source.as_bytes())
    }

    fn compile_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().compile(source) {
            Ok(_) => "compile unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn parse_ast_error_text(source: &[u8]) -> String {
        match YamlCompiler::default().parse_ast(source) {
            Ok(_) => "parse_ast unexpectedly succeeded".to_owned(),
            Err(errors) => match errors.first() {
                Some(error) => error.to_string(),
                None => "CompileErrors was empty".to_owned(),
            },
        }
    }

    fn assert_compile_parse_first_error(source: &[u8]) {
        assert_eq!(compile_error_text(source), parse_ast_error_text(source));
    }

    fn compile_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().compile(source) {
            Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn parse_first_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().parse_ast(source) {
            Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
    where
        T: core::fmt::Debug + PartialEq,
    {
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, found {actual:?}"))
        }
    }

    fn assert_compile_code(source: &[u8], expected: &'static str) -> Result<(), String> {
        let error = compile_first_error(source)?;
        ensure_equal(error.code(), expected)?;
        ensure_equal(error.diagnostic_code(), expected)
    }

    #[test]
    fn compile_error_exposes_stable_validation_codes() -> Result<(), String> {
        for (source, code) in [
            (
                b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
                "DUPLICATE_KEY",
            ),
            (
                b"version: velvet-ballastics/v1\nname: &n fast_path\ncopy: *n\n",
                "FORBIDDEN_YAML_FEATURE",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "UNKNOWN_TOP_LEVEL_FIELD",
            ),
            (
                b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "MISSING_REQUIRED_FIELD",
            ),
            (
                b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_VERSION",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: BuildResult\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: finish\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
                "RESERVED_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
                "DUPLICATE_ID",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
                "MISSING_STEP_PRIMITIVE",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
                "MULTIPLE_STEP_PRIMITIVES",
            ),
            (
                b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
                "INVALID_CHOOSE",
            ),
        ] {
            assert_compile_code(source, code)?;
        }
        Ok(())
    }

    #[test]
    fn reference_diagnostic_codes_cover_public_reference_contract() -> Result<(), String> {
        assert_compile_code(
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.missing == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            "UNKNOWN_REFERENCE",
        )?;
        assert_compile_code(
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$secrets.api_key == \"x\""
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            "SECRET_NOT_DECLARED",
        )
    }

    #[test]
    fn compile_errors_exposes_ordered_error_and_code_accessors() {
        let errors = CompileErrors(vec![
            CompileError::SourceTooLarge {
                actual: 8,
                limit: 4,
            },
            CompileError::InvalidVersion {
                actual: Box::<str>::from("velvet/v1"),
            },
        ]);
        let codes: Vec<&'static str> = errors.diagnostic_codes().collect();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors.as_slice().len(), 2);
        assert_eq!(errors.iter().count(), 2);
        assert_eq!(codes, vec!["PAYLOAD_TOO_LARGE", "INVALID_VERSION"]);
    }

    #[test]
    fn parse_ast_and_compile_expose_same_diagnostic_codes() -> Result<(), String> {
        for source in [
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n".as_slice(),
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.flag =="
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
            br#"version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$secrets.api_key == \"x\""
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#,
        ] {
            let compile = compile_first_error(source)?;
            let parse = parse_first_error(source)?;
            ensure_equal(compile.code(), parse.code())?;
        }
        Ok(())
    }

    #[test]
    fn compiler_rejects_save_object_until_handle_arenas_exist() {
        let source = br#"
version: velvet-ballastics/v1
name: fast_path
when:
  manual: {}
steps:
  - id: build_result
    save:
      text: done
  - id: done
    finish:
      result: 0
"#;
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { step: 0 }))
        ));
    }

    #[test]
    fn compiler_rejects_nested_save_values_until_handle_arenas_exist() {
        let result = YamlCompiler::default().compile(NESTED_SAVE_SOURCE);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { step: 0 }))
        ));
    }

    #[test]
    fn compiler_rejects_scalar_save_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save: done\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape { field: "save", .. }))
        ));
    }

    #[test]
    fn compiler_rejects_save_references_until_expressions_exist() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value: text\nsteps:\n  - id: build_result\n    save:\n      text: $input.value\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_empty_steps() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps: []\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::EmptySteps)))
        );
    }

    #[test]
    fn compiler_rejects_unsupported_top_level_fields() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nunexpected: true\nsteps:\n  - finish:\n      result: 0\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTopLevelField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_missing_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"name: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingField { .. })))
        );
    }

    #[test]
    fn compiler_rejects_non_canonical_workflow_version() {
        let result = YamlCompiler::default().compile(
            b"version: velvet/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidVersion { .. })))
        );
    }

    #[test]
    fn compiler_accepts_optional_top_level_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_accepts_allowed_input_schema_shorthand() {
        for shorthand in [
            "text",
            "number",
            "boolean",
            "object",
            "any",
            "list<any>",
            "list<text>",
            "list<number>",
            "list<boolean>",
        ] {
            let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
                "schema shorthand {shorthand} should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_unknown_input_schema_shorthand() {
        for shorthand in ["integer", "string", "list", "list<object>"] {
            let result = compile_with_inputs(&format!("  value: {shorthand}\n"));

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "schema shorthand {shorthand} should be rejected"
            );
        }
    }

    #[test]
    fn compiler_and_ast_report_same_schema_diagnostics() {
        for inputs in [
            "  value: integer\n",
            "  value:\n    is: text\n    kind: text\n",
            "  value:\n    is: text\n    default: 1\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {{}}\ninputs:\n{inputs}steps:\n  - id: done\n    finish:\n      result: 0\n"
            );

            assert_compile_parse_first_error(source.as_bytes());
        }
    }

    #[test]
    fn schema_validation_does_not_preempt_yaml_profile_errors() {
        assert_compile_parse_first_error(
            b"version: velvet-ballastics/v1\nname: &n schema_case\ninputs:\n  value: integer\ncopy: *n\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_duplicate_key_errors() {
        assert_compile_parse_first_error(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: schema_case\ninputs:\n  value: integer\n",
        );
    }

    #[test]
    fn schema_validation_does_not_preempt_lowering_errors() {
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: route\n    choose: true\n";

        assert_eq!(
            compile_error_text(source),
            CompileError::LastStepMustFinish.to_string()
        );
        assert_compile_parse_first_error(source);
    }

    #[test]
    fn schema_validation_does_not_preempt_finish_position_errors() {
        let source = b"version: velvet-ballastics/v1\nname: schema_case\nwhen:\n  manual: {}\ninputs:\n  value: integer\nsteps:\n  - id: early\n    finish:\n      result: 0\n      status: success\n  - id: done\n    finish:\n      result: 0\n";

        assert!(compile_error_text(source).contains("field finish"));
        assert_compile_parse_first_error(source);
    }

    #[test]
    fn compiler_accepts_input_long_form_scalar_constraints() {
        let result = compile_with_inputs(
            "  title:\n    from: request.body.title\n    is: text\n    default: hello\n    min_length: 1\n    max_length: 20\n    optional: true\n    nullable: false\n    secret: false\n  score:\n    is: number\n    default: 10\n    min: 0\n    max: 100\n  approved:\n    is: boolean\n    default: true\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_accepts_input_long_form_object_fields() {
        let result = compile_with_inputs(
            "  customer:\n    from: request.body.customer\n    is: object\n    fields:\n      id: text\n      email: text\n      address:\n        is: object\n        optional: true\n        nullable: true\n        fields:\n          city: text\n          country: text\n    extra: reject\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_accepts_input_long_form_list_elements() {
        for element in ["any", "text", "number", "boolean", "object"] {
            let result = compile_with_inputs(&format!(
                "  values:\n    is: list\n    of: {element}\n    default: []\n    min: 0\n    max: 10\n"
            ));

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "schema_case"),
                "list element schema {element} should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_input_schema_unknown_fields() {
        for inputs in [
            "  value:\n    is: text\n    kind: text\n",
            "  customer:\n    is: object\n    fields:\n      value:\n        is: text\n        from: request.body.value\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownInputSchemaField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_pattern_until_bounded_regex_exists() {
        let result = compile_with_inputs("  value:\n    is: text\n    pattern: ^[a-z]+$\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema {
                field: "inputs.pattern",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_child_fields() {
        for inputs in [
            "  values:\n    is: list\n",
            "  value:\n    is: text\n    of: text\n",
            "  value:\n    is: text\n    fields:\n      nested: text\n",
            "  value:\n    is: text\n    extra: reject\n",
            "  customer:\n    is: object\n    extra: ignore\n",
            "  customer:\n    is: object\n    fields: true\n",
            "  values:\n    is: list\n    of: integer\n",
            "  value:\n    is: integer\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "invalid schema should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_boolean_input_schema_flags() {
        for flag in ["optional", "nullable", "secret"] {
            let result = compile_with_inputs(&format!("  value:\n    is: text\n    {flag}: yes\n"));

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_default_that_does_not_match_input_schema() {
        for inputs in [
            "  value:\n    is: text\n    default: 1\n",
            "  value:\n    is: number\n    default: nope\n",
            "  value:\n    is: boolean\n    default: nope\n",
            "  value:\n    is: object\n    default: []\n",
            "  value:\n    is: list\n    of: text\n    default: {}\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
            ));
        }
    }

    #[test]
    fn compiler_validates_null_input_schema_defaults() {
        let rejected = compile_with_inputs("  value:\n    is: text\n    default: null\n");
        let accepted =
            compile_with_inputs("  value:\n    is: text\n    nullable: true\n    default: null\n");

        assert!(matches!(
            rejected,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))
        ));
        assert!(matches!(accepted, Ok(ref workflow) if workflow.name() == "schema_case"));
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_bounds() {
        for inputs in [
            "  value:\n    is: number\n    min: 10\n    max: 1\n",
            "  values:\n    is: list\n    of: text\n    min: -1\n",
            "  value:\n    is: text\n    min: 1\n",
            "  value:\n    is: text\n    min_length: -1\n",
            "  value:\n    is: text\n    min_length: 10\n    max_length: 1\n",
            "  value:\n    is: number\n    min_length: 1\n",
        ] {
            let result = compile_with_inputs(inputs);

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidInputSchema { .. }))),
                "invalid bounds should be rejected: {inputs}"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_mapping_optional_top_level_fields() {
        for field in ["inputs", "vars", "secrets"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}: true\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
                "{field} must be mapping-shaped"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_optional_top_level_names() {
        for (field, key) in [
            ("inputs", "InputValue"),
            ("vars", "run"),
            ("secrets", "api-key"),
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\n{field}:\n  {key}: value\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { .. }))),
                "{field}.{key} must use Velvet v1 public naming"
            );
        }
    }

    #[test]
    fn compiler_rejects_invalid_input_schema_shapes() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\ninputs:\n  value:\n    - text\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. })))
        );
    }

    #[test]
    fn compiler_rejects_runtime_references_in_vars() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nvars:\n  label: $input.value\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedConstantValue { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_non_string_secret_bindings() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsecrets:\n  api_key: 42\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_examples_shape() {
        for examples in ["true", "\n  - fixture"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. }))),
                "examples must be a sequence of mappings"
            );
        }
    }

    #[test]
    fn compiler_rejects_examples_without_valid_names() {
        for examples in ["\n  - input: {}", "\n  - name: 42", "\n  - name: run"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nexamples: {examples}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(
                        errors.first(),
                        Some(
                            CompileError::MissingField { .. }
                                | CompileError::FieldShape { .. }
                                | CompileError::InvalidName { .. }
                        )
                    )
                ),
                "examples must declare valid fixture names"
            );
        }
    }

    #[test]
    fn compiler_rejects_non_empty_top_level_result_until_result_ir_exists() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nresult:\n  value: $build_result.value\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedTopLevelResult))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_top_level_result() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nresult: done\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape {
                field: "result",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_names() {
        for name in ["", "FastPath", "fast-path", "run"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: \"{name}\"\nwhen:\n  manual: {{}}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidName { field: "name", .. }))),
                "workflow name {name:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingStepId { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_step_ids() {
        for id in ["", "BuildResult", "build-result", "finish"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: \"{id}\"\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(
                        errors.first(),
                        Some(CompileError::InvalidName {
                            field: "step id",
                            ..
                        })
                    )
                ),
                "step id {id:?} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_duplicate_step_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: duplicate\n    save:\n      value: 1\n  - id: duplicate\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateStepId { .. })))
        );
    }

    #[test]
    fn compiler_accepts_step_display_name_metadata() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: Build Result\n    save:\n      value: 1\n  - id: done\n    name: Done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"));
    }

    #[test]
    fn compiler_rejects_non_string_step_display_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    name: 42\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape { field: "name", .. }))
        ));
    }

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields() {
        for control in ["if", "with", "try_again", "on_error", "then"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: build_result\n    {control}: true\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedStepControlField { .. }))
                ),
                "control field {control} must be rejected until Phase 0 compiles it"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_workflow_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingField { .. })))
        );
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_shapes() {
        for source in [
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen: manual\nsteps:\n  - finish:\n      result: 0\n".as_slice(),
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen: {}\nsteps:\n  - finish:\n      result: 0\n",
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\n  event: {}\nsteps:\n  - finish:\n      result: 0\n",
        ] {
            let result = YamlCompiler::default().compile(source);

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_kind() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  file: {}\nsteps:\n  - finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerKind { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config() {
        for trigger in ["manual", "webhook", "schedule", "event"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
                "trigger {trigger} config must be mapping-shaped"
            );
        }
    }

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs() {
        for when_body in [
            "  manual: {}\n",
            "  webhook:\n    path: /github\n    method: POST\n    unique: request.header.X-GitHub-Delivery\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: UTC\n",
            "  event:\n    name: customer.created\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
                "valid trigger should compile"
            );
        }
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields() {
        for when_body in [
            "  manual:\n    extra: true\n",
            "  webhook:\n    path: /github\n    method: POST\n    extra: true\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    extra: true\n",
            "  event:\n    name: customer.created\n    extra: true\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields() {
        for when_body in [
            "  webhook:\n    method: POST\n",
            "  webhook:\n    path: /github\n",
            "  schedule:\n    timezone: UTC\n",
            "  event: {}\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values() {
        for when_body in [
            "  webhook:\n    path: github\n    method: POST\n",
            "  webhook:\n    path: /github\n    method: TRACE\n",
            "  webhook:\n    path: 42\n    method: POST\n",
            "  webhook:\n    path: /github\n    method: POST\n    unique: 42\n",
            "  schedule:\n    cron: \"0 0 0 0 0 0\"\n",
            "  schedule:\n    cron: 42\n",
            "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: 42\n",
            "  event:\n    name: 42\n",
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(matches!(
                result,
                Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
            ));
        }
    }

    #[test]
    fn compiler_rejects_backward_branch_targets() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: true\n      on_true: 0\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::BackwardBranchTarget { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_choose_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose:\n      condition: 0\n      on_true: 1\n      on_false: 1\n      otherwise: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField {
                primitive: "choose",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_choose_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: route\n    choose: true\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape {
                field: "choose",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_extra_phase_zero_finish_fields() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n      status: success\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField {
                primitive: "finish",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_non_mapping_phase_zero_finish_body() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish: success\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::StepFieldShape {
                field: "finish",
                ..
            }))
        ));
    }

    #[test]
    fn compiler_rejects_aliases() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: &n fast\ncopy: *n\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::AnchorForbidden { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_custom_tags_with_mark() {
        let result = YamlCompiler::default().compile(b"version: !custom velvet-ballastics/v1\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::TagForbidden { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_non_string_object_keys_with_mark() {
        let result = YamlCompiler::default().compile(b"? [bad]\n: value\n");

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::NonStringKey { mark }) if mark.available)
        ));
    }

    #[test]
    fn compiler_rejects_duplicate_top_level_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nversion: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateKey { .. })))
        );
    }

    #[test]
    fn compiler_rejects_duplicate_nested_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      text: first\n      text: second\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::DuplicateKey { .. })))
        );
    }

    #[test]
    fn compiler_rejects_legacy_step_aliases() {
        for alias in ["gather", "summarize", "copy"] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: legacy\n    {alias}:\n      slot: 0\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepField { .. }))),
                "legacy alias {alias} must be rejected"
            );
        }
    }

    #[test]
    fn compiler_rejects_missing_step_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: only_metadata\n    name: Only Metadata\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingStepPrimitive { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_multiple_step_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      slot: 0\n      value: 1\n    finish:\n      result: 0\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MultipleStepPrimitives { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_malformed_master_primitives_with_exact_diagnostic() {
        for (primitive, code) in [
            ("for_each", "INVALID_FOR_EACH"),
            ("together", "INVALID_TOGETHER"),
            ("collect", "INVALID_COLLECT"),
            ("reduce", "INVALID_REDUCE"),
            ("repeat", "INVALID_REPEAT"),
        ] {
            let source = format!(
                "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {{}}\nsteps:\n  - id: unsupported\n    {primitive}: noop\n  - id: done\n    finish:\n      result: 0\n"
            );
            let result = YamlCompiler::default().compile(source.as_bytes());

            assert!(
                matches!(
                    result,
                    Err(ref errors)
                        if errors.first().map(CompileError::code) == Some(code)
                ),
                "primitive {primitive} should be rejected with exact invalid diagnostic"
            );
        }
    }

    #[test]
    fn compiler_lowers_yaml_for_each_to_loop_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: for_each_case\nwhen:\n  manual: {}\nsteps:\n  - id: list\n    save:\n      value: 1\n  - id: each\n    for_each:\n      input: 0\n      item: 1\n      limit: 10\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let start = workflow
            .node(StepIdx::new(1))
            .ok_or("missing for_each start")?;
        let next = workflow
            .node(StepIdx::new(2))
            .ok_or("missing for_each next")?;

        assert!(
            matches!(start.kind, CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } if input == SlotIdx::ZERO && item_slot == SlotIdx::new(1) && limit == 10 && body == StepIdx::new(2) && done == StepIdx::new(3))
        );
        assert!(
            matches!(next.kind, CompiledNodeKind::ForEachNext { iterator_slot, body, done } if iterator_slot == SlotIdx::new(1) && body == StepIdx::new(2) && done == StepIdx::new(3))
        );
        Ok(())
    }

    #[test]
    fn compiler_rejects_for_each_with_unsupported_at_once_field() {
        let source = "version: velvet-ballastics/v1\nname: for_each_unsupported\nwhen:\n  manual: {}\nsteps:\n  - id: list\n    save:\n      value: [1, 2, 3]\n  - id: each\n    for_each:\n      input: 0\n      item: 1\n      limit: 10\n      at_once: 5\n  - id: done\n    finish:\n      result: 0\n";
        let result = YamlCompiler::default().compile(source.as_bytes());
        assert!(
            matches!(
                result,
                Err(ref errors)
                    if errors.first().map(CompileError::code) == Some("INVALID_FOR_EACH")
            ),
            "for_each with at_once must be rejected with INVALID_FOR_EACH, got: {result:?}"
        );
        assert!(
            matches!(
                result,
                Err(ref errors)
                    if matches!(errors.first(), Some(CompileError::UnsupportedStepPrimitive { step: 1, primitive: "for_each" }))
            ),
            "for_each with at_once must produce UnsupportedStepPrimitive error, got: {result:?}"
        );
    }

    #[test]
    fn compiler_lowers_yaml_together_to_start_and_join_nodes() -> Result<(), String> {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: together_case\nwhen:\n  manual: {}\nsteps:\n  - id: fanout\n    together:\n      branches: [1]\n  - id: done\n    finish:\n      result: 0\n",
        );
        // The shared validation pipeline catches the invalid together IR:
        // join (node 1) is not after branch target (node 2).
        assert!(
            matches!(result, Err(ref errors) if errors.0.iter().any(|e| matches!(e, CompileError::Validation(vb_validate::ValidationError::LoopBodyStepOutOfRange { .. })))),
            "expected LoopBodyStepOutOfRange validation error, got: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_collect_to_collection_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: collect_case\nwhen:\n  manual: {}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: collect_values\n    collect:\n      source: 0\n      limit: 5\n      page_size: 2\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::CollectStart { source, limit, page_size, body, done }) if *source == SlotIdx::ZERO && *limit == 5 && *page_size == 2 && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::CollectPage { collector_slot, body, done }) if *collector_slot == SlotIdx::ZERO && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(3)).map(|node| &node.kind), Some(CompiledNodeKind::CollectFinish { collector_slot }) if *collector_slot == SlotIdx::ZERO)
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_reduce_to_reduction_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: reduce_case\nwhen:\n  manual: {}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: reduce_values\n    reduce:\n      input: 0\n      accumulator: 1\n      initial: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceStart { input, accumulator, initial, body, done }) if *input == SlotIdx::ZERO && *accumulator == SlotIdx::new(1) && *initial == ConstIdx::new(1) && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceNext { iterator_slot, accumulator, body, done }) if *iterator_slot == SlotIdx::new(1) && *accumulator == SlotIdx::new(1) && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(3)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceFinish { accumulator }) if *accumulator == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_repeat_to_attempt_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: repeat_case\nwhen:\n  manual: {}\nsteps:\n  - id: poll\n    repeat:\n      max_attempts: 3\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::ZERO).map(|node| &node.kind), Some(CompiledNodeKind::RepeatStart { max_attempts, body, done }) if *max_attempts == 3 && *body == StepIdx::new(1) && *done == StepIdx::new(2))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::RepeatAttempt { attempt_slot, body, done }) if *attempt_slot == SlotIdx::new(1) && *body == StepIdx::new(1) && *done == StepIdx::new(2))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::RepeatFinish { result }) if *result == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_rejects_oversized_source() {
        let limits = YamlLimits {
            max_source_bytes: 4,
            ..YamlLimits::default()
        };
        let result = YamlCompiler::new(limits).compile(b"name: too_large\n");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::SourceTooLarge { .. })))
        );
    }

    #[test]
    fn compiler_accepts_minimal_strict_workflow() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: strict_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "strict_minimal"));
    }

    #[test]
    fn compiler_lowers_yaml_set_to_set_const_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: set_case\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    set:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;

        assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
        assert_eq!(node.output, Some(SlotIdx::ZERO));
        assert_eq!(node.next, Some(StepIdx::new(1)));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_wait_until_to_wait_until_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: wait_case\nwhen:\n  manual: {}\nsteps:\n  - id: deadline\n    save:\n      value: 1\n  - id: wait_for_deadline\n    wait:\n      until: 0\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let node = workflow.node(StepIdx::new(1)).ok_or("missing wait node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO
            }
        ));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_ask_to_ask_and_resume_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: ask_case\nwhen:\n  manual: {}\nsteps:\n  - id: prompt\n    save:\n      value: 1\n  - id: ask_user\n    ask:\n      prompt: 0\n      answer: 1\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let ask = workflow.node(StepIdx::new(1)).ok_or("missing ask node")?;
        let resume = workflow
            .node(StepIdx::new(2))
            .ok_or("missing resume node")?;
        let finish = workflow
            .node(StepIdx::new(3))
            .ok_or("missing finish node")?;

        assert!(matches!(ask.kind, CompiledNodeKind::Ask { .. }));
        assert!(
            matches!(resume.kind, CompiledNodeKind::AskResume { answer } if answer == SlotIdx::new(1))
        );
        assert!(
            matches!(finish.kind, CompiledNodeKind::Finish { result } if result == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_run_to_do_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: run_case\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    run:\n      action: 7\n      input: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        assert_eq!(workflow.node_count(), 3);
        assert_eq!(workflow.slot_count(), 2);
        let node = workflow.node(StepIdx::new(1)).ok_or("missing run node")?;
        let finish = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::Do { action, input }
                if action == ActionId::new(7) && input == SlotIdx::ZERO
        ));
        assert_eq!(node.output, Some(SlotIdx::new(1)));
        assert_eq!(node.next, Some(StepIdx::new(2)));
        assert!(matches!(
            finish.kind,
            CompiledNodeKind::Finish { result } if result == SlotIdx::new(1)
        ));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_do_alias_to_do_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: do_case\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    do:\n      action: 11\n      input: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        assert_eq!(workflow.node_count(), 3);
        assert_eq!(workflow.slot_count(), 2);
        let node = workflow.node(StepIdx::new(1)).ok_or("missing do node")?;
        let finish = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::Do { action, input }
                if action == ActionId::new(11) && input == SlotIdx::ZERO
        ));
        assert_eq!(node.output, Some(SlotIdx::new(1)));
        assert_eq!(node.next, Some(StepIdx::new(2)));
        assert!(matches!(
            finish.kind,
            CompiledNodeKind::Finish { result } if result == SlotIdx::new(1)
        ));
        Ok(())
    }

    #[test]
    fn compiler_preserves_action_name_run_rejection() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: action_name\nwhen:\n  manual: {}\nsteps:\n  - id: call_action\n    run: shell.run\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnsupportedStepPrimitive { step: 0, primitive: "run" }))
        ));
    }

    #[test]
    fn compiler_rejects_action_schema_form_with_unknown_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: action_schema\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    run:\n      action: 7\n      input: 0\n      with: {}\n  - id: done\n    finish:\n      result: 1\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownStepPrimitiveField { step: 1, primitive: "run", field }) if field.as_ref() == "with")
        ));
    }

    #[test]
    fn compiler_attaches_default_resource_contract() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: resource_case\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        if workflow.resource_contract() == ResourceContract::DEFAULT {
            Ok(())
        } else {
            Err(format!(
                "unexpected resource contract: {:?}",
                workflow.resource_contract()
            ))
        }
    }

    #[test]
    fn compiler_rejects_empty_yaml_source() {
        let result = YamlCompiler::default().compile(b"   \n\t  ");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::EmptySource)))
        );
    }

    #[test]
    fn compiler_rejects_multiple_yaml_documents() {
        let result = YamlCompiler::default().compile(
            b"---\nversion: velvet-ballastics/v1\nname: first\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n---\nversion: velvet-ballastics/v1\nname: second\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::DocumentCount { count: 2 }))
        ));
    }

    #[test]
    fn compiler_rejects_yaml_merge_keys() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: merge_key\nwhen:\n  manual: {}\n<<:\n  steps: []\nsteps:\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MergeKeyForbidden { .. }))
        ));
    }

    // ── Round 2: Exact-assertion error variant tests ─────────────────────

    #[test]
    fn compile_returns_source_too_large_with_exact_fields() {
        let tiny_limits = YamlLimits {
            max_source_bytes: 10,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let source = b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0";
        let result = compiler.compile(source);
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::SourceTooLarge { actual, limit }) = errors.first() else {
            compile_test_fail!("expected SourceTooLarge, got {:?}", errors.first());
        };
        assert_eq!(*limit, 10);
        assert_eq!(*actual, source.len());
    }

    #[test]
    fn compile_returns_empty_source_for_empty_input() {
        let result = YamlCompiler::default().compile(b"");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::EmptySource)));
    }

    #[test]
    fn compile_returns_top_level_not_mapping_for_list_root() {
        let result = YamlCompiler::default().compile(b"- item1\n- item2");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::TopLevelNotMapping)
        ));
    }

    #[test]
    fn compile_returns_empty_steps_for_steps_with_empty_list() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::EmptySteps)));
    }

    #[test]
    fn compile_returns_invalid_version_for_wrong_version() {
        let result = YamlCompiler::default().compile(
            b"version: bad-version\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidVersion { actual }) = errors.first() else {
            compile_test_fail!("expected InvalidVersion, got {:?}", errors.first());
        };
        assert_eq!(actual.as_ref(), "bad-version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_version() {
        let result = YamlCompiler::default().compile(
            b"name: no_version\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "name");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_trigger\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "when");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_steps() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: no_steps\nwhen:\n  manual: {}");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "steps");
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_empty_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: empty_when\nwhen: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            compile_test_fail!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 0);
    }

    #[test]
    fn compile_returns_unknown_trigger_kind_for_invalid_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_trigger\nwhen:\n  teleport: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownTriggerKind { trigger }) = errors.first() else {
            compile_test_fail!("expected UnknownTriggerKind, got {:?}", errors.first());
        };
        assert_eq!(trigger.as_ref(), "teleport");
    }

    #[test]
    fn compile_returns_missing_step_id_for_step_without_id() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_id\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingStepId { step }) = errors.first() else {
            compile_test_fail!("expected MissingStepId, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_step_shape_for_non_mapping_step() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_step\nwhen:\n  manual: {}\nsteps:\n  - \"scalar\"",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::StepShape { step }) = errors.first() else {
            compile_test_fail!("expected StepShape, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_duplicate_step_id_for_same_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup_step\nwhen:\n  manual: {}\nsteps:\n  - id: same\n    save:\n      x: 1\n  - id: same\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DuplicateStepId { id }) = errors.first() else {
            compile_test_fail!("expected DuplicateStepId, got {:?}", errors.first());
        };
        assert_eq!(id.as_ref(), "same");
    }

    #[test]
    fn compile_returns_missing_step_primitive_for_step_without_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_prim\nwhen:\n  manual: {}\nsteps:\n  - id: empty_step",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingStepPrimitive { step }) = errors.first() else {
            compile_test_fail!("expected MissingStepPrimitive, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_unknown_step_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_field\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    unknown_field: 1\n    save:\n      x: 1",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownStepField { step, field }) = errors.first() else {
            compile_test_fail!("expected UnknownStepField, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
        assert_eq!(field.as_ref(), "unknown_field");
    }

    #[test]
    fn compile_returns_last_step_must_finish_for_non_finish_ending() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_finish\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::LastStepMustFinish)
        ));
    }

    #[test]
    fn compile_returns_unknown_top_level_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: extra\nwhen:\n  manual: {}\nunknown_root: true\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownTopLevelField { field }) = errors.first() else {
            compile_test_fail!("expected UnknownTopLevelField, got {:?}", errors.first());
        };
        assert_eq!(field.as_ref(), "unknown_root");
    }

    #[test]
    fn compile_returns_tag_forbidden_for_tagged_node() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: tagged\nwhen:\n  manual: {}\nsteps:\n  - id: !!tag done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::TagForbidden { .. })
        ));
    }

    #[test]
    fn compile_returns_float_forbidden_for_float_scalar() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: floaty\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 3.14",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::FloatForbidden)));
    }

    #[test]
    fn compile_returns_depth_limit_for_deeply_nested_yaml() {
        let tiny_limits = YamlLimits {
            max_depth: 3,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: deep\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\na:\n  b:\n    c:\n      d: deep",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DepthLimit { depth, limit }) = errors.first() else {
            compile_test_fail!("expected DepthLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 3);
        assert!(*depth > 3);
    }

    #[test]
    fn compile_returns_node_limit_for_many_nodes() {
        let tiny_limits = YamlLimits {
            max_nodes: 5,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      a: 1\n      b: 2\n      c: 3\n      d: 4\n      e: 5\n      f: 6\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::NodeLimit { limit }) = errors.first() else {
            compile_test_fail!("expected NodeLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
    }

    #[test]
    fn compile_returns_scalar_limit_for_long_scalar() {
        let tiny_limits = YamlLimits {
            max_scalar_bytes: 5,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: long_scalar\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nlabel: abcdefgh",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::ScalarLimit { actual, limit }) = errors.first() else {
            compile_test_fail!("expected ScalarLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
        assert!(*actual > 5);
    }

    #[test]
    fn compile_returns_duplicate_key_for_repeated_yaml_key() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DuplicateKey { key, .. }) = errors.first() else {
            compile_test_fail!("expected DuplicateKey, got {:?}", errors.first());
        };
        assert_eq!(key.as_ref(), "name");
    }

    #[test]
    fn compile_returns_invalid_name_for_reserved_step_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: reserved\nwhen:\n  manual: {}\nsteps:\n  - id: run\n    save:\n      x: 1\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidName { field, value }) = errors.first() else {
            compile_test_fail!("expected InvalidName, got {:?}", errors.first());
        };
        assert_eq!(*field, "step id");
        assert_eq!(value.as_ref(), "run");
    }

    #[test]
    fn compile_returns_multiple_step_primitives_for_two_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MultipleStepPrimitives { step }) = errors.first() else {
            compile_test_fail!("expected MultipleStepPrimitives, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_two_triggers() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi_trigger\nwhen:\n  manual: {}\n  ipc: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            compile_test_fail!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 2);
    }

    #[test]
    fn compile_returns_field_shape_for_bad_inputs_shape() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_inputs\nwhen:\n  manual: {}\ninputs: []\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::FieldShape { field, expected }) = errors.first() else {
            compile_test_fail!("expected FieldShape, got {:?}", errors.first());
        };
        assert_eq!(*field, "inputs");
        assert!(!expected.is_empty());
    }

    // ── Round 2: Compilation success path tests ──────────────────────────

    #[test]
    fn compile_produces_valid_workflow_for_minimal_source() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
    }

    #[test]
    fn compile_produces_valid_workflow_for_optional_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_non_default_workflow_digest() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_ne!(
            wf.digest(),
            vb_core::ids::WorkflowDigest::from_bytes([0u8; 32])
        );
    }

    #[test]
    fn compile_produces_matching_workflow_name() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_correct_entry_step_index() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_eq!(wf.entry(), vb_core::ids::StepIdx::ZERO);
    }

    #[test]
    fn compile_with_limits_respects_custom_source_limit() {
        let source = OPTIONAL_TOP_LEVEL_FIELDS_SOURCE;
        let limits = YamlLimits {
            max_source_bytes: source.len() + 1,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler { limits };
        let result = compiler.compile(source);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
    }

    #[test]
    fn compile_to_generated_rust_accepts_supported_subset() -> Result<(), String> {
        let workflow = supported_codegen_workflow()?;

        let source = compile_to_generated_rust(&workflow).map_err(|e| e.to_string())?;

        assert!(
            source.contains("pub fn drive"),
            "generated source must contain drive function"
        );
        Ok(())
    }

    #[test]
    fn compile_to_generated_rust_rejects_unsupported_ir_before_emit() -> Result<(), String> {
        let workflow = unsupported_codegen_workflow()?;

        let error = compile_to_generated_rust(&workflow)
            .err()
            .ok_or("unsupported IR unexpectedly generated source")?;

        assert!(
            error.to_string().contains("BuildList"),
            "unsupported IR error must name rejected feature, got: {error}"
        );
        Ok(())
    }

    #[test]
    fn compile_to_generated_rust_reports_subset_rejection_as_compile_error() -> Result<(), String> {
        let workflow = unsupported_codegen_workflow()?;

        let errors = compile_to_generated_rust(&workflow)
            .err()
            .ok_or("unsupported IR unexpectedly generated source")?;
        let first = errors
            .0
            .first()
            .ok_or("unsupported IR must produce a compile error")?;

        assert_eq!(first.diagnostic_code(), "INVALID_EXPRESSION");
        assert!(
            first
                .to_string()
                .contains("unsupported generated Rust IR feature"),
            "generated-mode subset rejection must be explicit, got: {first}"
        );
        Ok(())
    }

    fn supported_codegen_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("compile_codegen_supported"),
            digest: WorkflowDigest::from_bytes([0x31; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::I64(7)].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    fn unsupported_codegen_workflow() -> Result<CompiledWorkflow, String> {
        let parts = WorkflowParts {
            name: Box::<str>::from("compile_codegen_unsupported"),
            digest: WorkflowDigest::from_bytes([0x32; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(1)),
                    next: Some(StepIdx::new(1)),
                    kind: CompiledNodeKind::BuildList {
                        items: vec![SlotIdx::new(0)].into_boxed_slice(),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(1),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 2,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
        };
        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

    // ── Round 2: CompileError::code() tests ──────────────────────────────

    #[test]
    fn compile_error_code_returns_payload_too_large_for_source_too_large() {
        let err = CompileError::SourceTooLarge {
            actual: 100,
            limit: 50,
        };
        assert_eq!(err.code(), "PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn compile_error_code_returns_missing_required_field_for_empty_source() {
        let err = CompileError::EmptySource;
        assert_eq!(err.code(), "MISSING_REQUIRED_FIELD");
    }

    #[test]
    fn compile_error_code_returns_type_mismatch_for_top_level_not_mapping() {
        let err = CompileError::TopLevelNotMapping;
        assert_eq!(err.code(), "TYPE_MISMATCH");
    }

    #[test]
    fn compile_error_code_returns_duplicate_key_for_duplicate_key() {
        let err = CompileError::DuplicateKey {
            key: Box::from("test"),
            mark: SourceMark {
                index: 0,
                end_index: 0,
                line: 1,
                column: 1,
                available: true,
            },
        };
        assert_eq!(err.code(), "DUPLICATE_KEY");
    }

    #[test]
    fn compile_error_code_returns_limit_exceeded_for_depth_limit() {
        let err = CompileError::DepthLimit {
            depth: 10,
            limit: 5,
        };
        assert_eq!(err.code(), "LIMIT_EXCEEDED");
    }

    #[test]
    fn compile_error_code_returns_limit_exceeded_for_node_limit() {
        let err = CompileError::NodeLimit { limit: 100 };
        assert_eq!(err.code(), "LIMIT_EXCEEDED");
    }

    #[test]
    fn compile_error_code_returns_forbidden_yaml_for_alias() {
        let err = CompileError::AliasForbidden {
            mark: SourceMark {
                index: 0,
                end_index: 0,
                line: 1,
                column: 1,
                available: true,
            },
        };
        assert_eq!(err.code(), "FORBIDDEN_YAML_FEATURE");
    }
}
