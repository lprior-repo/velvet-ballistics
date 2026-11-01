//! Migration regression coverage: verifies all old public module paths
//! are still accessible after extracting vb_expr and vb_yaml into
//! separate crates and re-exporting them through vb_compile.
//!
//! Each test uses actual, real APIs — not fabricated signatures.
//! The purpose is compile-time reachability + one sharp behavioral check.
//!
//! Production section: MASTER §77.17 IR-version classification. Compiled
//! workflows carry no version field yet, so classification takes both
//! versions as parameters; this is the seam the IR-version wiring lands
//! on when the field exists.

use vb_core::compat::{CompatBoundary, CompatDecision, CompatDigests, CompatInputs, SchemaVersion};

/// Current IR schema version declared by this batch. The compiled IR has
/// no version field today; this constant is the consumer side of the
/// classification until one lands.
pub const CURRENT_IR_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

/// Classifies an IR artifact version pair at the
/// [`CompatBoundary::IrVersion`] boundary using the §77.17 algebra.
/// Callers that journal the decision supply real digests; storage-side
/// callers compute the rule digest with
/// `vb_storage::compat::compat_rule_digest`. Nothing here fabricates one.
#[must_use]
pub fn classify_ir_version(
    held: SchemaVersion,
    required: SchemaVersion,
    migration_available: bool,
    replay_only: bool,
    digests: CompatDigests,
) -> CompatDecision {
    CompatDecision::decide(
        CompatInputs {
            boundary: CompatBoundary::IrVersion,
            held,
            required,
            migration_available,
            replay_only,
        },
        digests,
    )
}

/// True when the classified IR version may proceed to live-effect dispatch.
#[must_use]
pub fn ir_version_allows_live_effects(decision: CompatDecision) -> bool {
    decision.class.authorizes_live_effect()
}

#[cfg(test)]
mod expr_module_paths {
    // Compile-time checks: old expr module paths must resolve via re-exports
    use crate::expr_bytecode;
    use crate::expr_eval;
    use crate::expr_lexer;
    use crate::expr_parser;
    use crate::expr_stack_ops;
    use crate::expr_typecheck;

    /// Verify all re-exported expr module paths resolve at compile time.
    #[test]
    fn expr_modules_resolve() {
        // These are compile-time only: function pointers prove the modules exist
        let _ = expr_bytecode::compile_expr_to_bytecode;
        let _ = expr_eval::eval_expr_program;
        let _ = expr_lexer::lex_expr;
        let _ = expr_parser::parse_expr;
        let _ = expr_stack_ops::push_value;
        let _ = expr_typecheck::typecheck_expr;
    }

    /// Behavioral check: lexer handles a simple identifier via old path.
    #[test]
    fn expr_lexer_behavioral_check() -> Result<(), Box<dyn std::error::Error>> {
        let tokens = expr_lexer::lex_expr("x")?;
        let has_identifier = tokens
            .iter()
            .any(|t| matches!(t, expr_lexer::Token::Identifier(_)));
        assert!(has_identifier, "Expected an identifier token");
        Ok(())
    }

    /// Full pipeline check: 1 + 2 = I64(3) via old module paths.
    #[test]
    fn expr_full_pipeline_via_old_paths() -> Result<(), Box<dyn std::error::Error>> {
        let tokens = expr_lexer::lex_expr("1 + 2")?;
        let ast = expr_parser::parse_expr(&tokens)?;
        // Use compile_expr_with_pool to get both the program and constants
        let mut constants: Vec<vb_core::ConstValue> = Vec::new();
        let program = expr_bytecode::compile_expr_with_pool(&ast, &mut constants)?;
        let result = expr_eval::eval_expr_program(&program, &[], &constants)?;
        assert_eq!(result, vb_core::SlotValue::I64(3));
        Ok(())
    }
}

#[cfg(test)]
mod yaml_module_paths {
    // Compile-time checks: old yaml module paths must resolve via re-exports
    use crate::yaml_error;
    use crate::yaml_events;
    use crate::yaml_limits;
    use crate::yaml_profile;
    use crate::yaml_source_map;

    /// Verify all re-exported yaml module paths resolve at compile time.
    #[test]
    fn yaml_modules_resolve() {
        let _ = yaml_events::collect_events;
        let _ = yaml_profile::validate_yaml_profile;
        let _ = yaml_source_map::build_source_map;
        let _ = yaml_limits::YamlLimits::default;
    }

    /// Behavioral check: validate_yaml_profile rejects anchors via old path.
    #[test]
    fn yaml_profile_rejects_anchors() {
        let result = yaml_profile::validate_yaml_profile("value: &anchor test\n");
        assert!(
            matches!(result, Err(yaml_error::YamlError::AnchorAliasMerge)),
            "Expected AnchorAliasMerge, got {:?}",
            result
        );
    }

    /// Behavioral check: validate_yaml_profile rejects duplicate keys via old path.
    #[test]
    fn yaml_profile_rejects_duplicate_keys() {
        let result = yaml_profile::validate_yaml_profile("version: a\nversion: b\n");
        assert!(
            matches!(result,
                Err(yaml_error::YamlError::DuplicateKey { ref key }) if key.as_ref() == "version"
            ),
            "Expected DuplicateKey for version, got {result:?}"
        );
    }

    /// Behavioral check: validate_yaml_profile rejects multiple documents via old path.
    #[test]
    fn yaml_profile_rejects_multiple_documents() {
        let result = yaml_profile::validate_yaml_profile("a: 1\n---\nb: 2\n");
        assert!(
            matches!(
                result,
                Err(yaml_error::YamlError::MultipleDocuments { count: 2 })
            ),
            "Expected two documents, got {result:?}"
        );
    }

    /// Compile-time check that yaml_ast module is accessible via old path.
    #[test]
    fn yaml_ast_module_accessible() {
        use crate::yaml_ast;
        // Verify the module exists by referencing its types
        let _ = std::any::type_name::<yaml_ast::StepPrimitive>();
    }
}

#[cfg(test)]
// No allow block: the forbid-level lints cannot be allowed back, and these
// tests use only assertions, so no allow is needed.
mod ir_compat_tests {
    use super::*;
    use vb_core::compat::{CompatDigests, CompatibilityClass};

    fn digests() -> CompatDigests {
        CompatDigests {
            rule_digest: [7u8; 32],
            old_schema_digest: [1u8; 32],
            new_schema_digest: [2u8; 32],
        }
    }

    #[test]
    fn current_ir_version_is_v1() {
        assert_eq!(CURRENT_IR_VERSION, SchemaVersion::new(1, 0, 0));
    }

    #[test]
    fn identical_ir_allows_live_effects() {
        let decision = classify_ir_version(
            CURRENT_IR_VERSION,
            CURRENT_IR_VERSION,
            false,
            false,
            digests(),
        );
        assert_eq!(decision.class, CompatibilityClass::Identical);
        assert_eq!(decision.boundary, CompatBoundary::IrVersion);
        assert!(ir_version_allows_live_effects(decision));
        assert!(decision.authorize_live_effect().is_ok());
    }

    #[test]
    fn replay_only_ir_blocks_live_effects() {
        let decision = classify_ir_version(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            false,
            true,
            digests(),
        );
        assert_eq!(decision.class, CompatibilityClass::ReplayOnly);
        assert!(!ir_version_allows_live_effects(decision));
        assert!(decision.authorize_replay().is_ok());
    }

    #[test]
    fn migration_required_ir_blocks_dispatch() {
        let decision = classify_ir_version(
            SchemaVersion::new(1, 0, 0),
            SchemaVersion::new(2, 0, 0),
            true,
            false,
            digests(),
        );
        assert_eq!(decision.class, CompatibilityClass::MigrationRequired);
        assert!(decision.blocks_dispatch());
    }
}
