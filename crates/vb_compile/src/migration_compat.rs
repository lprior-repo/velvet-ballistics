//! Migration regression coverage: verifies all old public module paths
//! are still accessible after extracting vb_expr and vb_yaml into
//! separate crates and re-exporting them through vb_compile.
//!
//! Each test uses actual, real APIs — not fabricated signatures.
//! The purpose is compile-time reachability + one sharp behavioral check.

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
