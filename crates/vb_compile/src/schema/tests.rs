mod tests {
    use super::*;
    use crate::YamlCompiler;
    use saphyr::LoadableYamlNode;

    fn validate_inputs(inputs: &str) -> Result<(), CompileError> {
        let source = format!("version: velvet-ballastics/v1\ninputs:\n{inputs}\n");
        let docs = Yaml::load_from_str(&source)?;
        let Some(doc) = docs.first() else {
            return Err(CompileError::EmptySource);
        };
        match validate_input_schemas(doc) {
            Ok(()) => Ok(()),
            Err(errors) => match errors.first() {
                Some(error) => Err(error.clone()),
                None => Err(CompileError::EmptySource),
            },
        }
    }

    #[test]
    fn input_schema_rejects_unknown_fields() {
        let result = validate_inputs("  value:\n    is: text\n    kind: text\n");

        assert!(matches!(
            result,
            Err(CompileError::UnknownInputSchemaField { .. })
        ));
    }

    #[test]
    fn input_schema_rejects_invalid_bounds() {
        let result =
            validate_inputs("  value:\n    is: text\n    min_length: 9\n    max_length: 1\n");

        assert!(matches!(
            result,
            Err(CompileError::InvalidInputSchema { .. })
        ));
    }

    // ---------------------------------------------------------------------------
    // vb-yd5x RED PHASE: Shared IR parity tests
    // ---------------------------------------------------------------------------

    /// Minimal valid workflow for testing
    const VB_YD5X_MINIMAL_VALID_WORKFLOW: &[u8] = br#"
version: velvet-ballastics/v1
name: minimal_valid
when:
  manual: {}
steps:
  - id: start
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;

    /// Workflow with out-of-range slot reference (Gate 9)
    /// This uses a slot index that is out of bounds for the compiled workflow.
    /// The issue is the result slot 99 doesn't exist.
    const VB_YD5X_MALFORMED_SLOT_REF: &[u8] = br#"
version: velvet-ballastics/v1
name: bad_slot_ref
when:
  manual: {}
steps:
  - id: start
    save:
      value: 1
  - id: use_missing_slot
    for_each:
      input: 99
      item: 1
      limit: 10
  - id: done
    finish:
      result: 0
"#;

    /// Workflow with loop body step out of range (Gate 11)
    /// The together branches point to step 2 (join) but join is at node 1, not step 2.
    const VB_YD5X_MALFORMED_LOOP_BODY: &[u8] = br#"
version: velvet-ballastics/v1
name: bad_loop_body
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches: [2]
  - id: join
    finish:
      result: 0
"#;

    /// Workflow with duplicate step ID
    const VB_YD5X_MALFORMED_DUPLICATE_ID: &[u8] = br#"
version: velvet-ballastics/v1
name: duplicate_ids
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: build
    finish:
      result: 0
"#;

    /// Workflow with unknown reference
    const VB_YD5X_MALFORMED_UNKNOWN_REF: &[u8] = br#"
version: velvet-ballastics/v1
name: unknown_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $input.missing == true
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;

    /// Helper: validate via compile then shared pipeline
    fn vb_yd5x_validate_via_compile(source: &[u8]) -> Result<(), CompileErrors> {
        let compiled = YamlCompiler::default().compile(source)?;
        let parts = compiled.to_parts();
        vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))
    }

    #[test]
    fn vb_yd5x_valid_workflow_passes_both_paths() {
        let source = VB_YD5X_MINIMAL_VALID_WORKFLOW;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_ok(),
            "valid workflow must compile: {compile_result:?}"
        );
        assert!(
            validate_result.is_ok(),
            "valid workflow must pass shared validation: {validate_result:?}"
        );
    }

    #[test]
    fn vb_yd5x_malformed_slot_ref_fails_consistently() {
        let source = VB_YD5X_MALFORMED_SLOT_REF;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        // Both must fail
        assert!(
            compile_result.is_err(),
            "compile should fail for bad slot ref"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for bad slot ref"
        );
        // Both should produce the same error code
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(
            compile_code,
            Some("TYPE_MISMATCH"),
            "expected TYPE_MISMATCH"
        );
    }

    #[test]
    fn vb_yd5x_malformed_loop_body_fails_consistently() {
        let source = VB_YD5X_MALFORMED_LOOP_BODY;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_err(),
            "compile should fail for bad loop body"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for bad loop body"
        );
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(
            compile_code,
            Some("INVALID_THEN_TARGET"),
            "expected INVALID_THEN_TARGET"
        );
    }

    #[test]
    fn vb_yd5x_malformed_duplicate_id_fails_consistently() {
        let source = VB_YD5X_MALFORMED_DUPLICATE_ID;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_err(),
            "compile should fail for duplicate id"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for duplicate id"
        );
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(compile_code, Some("DUPLICATE_ID"), "expected DUPLICATE_ID");
    }

    #[test]
    fn vb_yd5x_malformed_unknown_ref_fails_consistently() {
        let source = VB_YD5X_MALFORMED_UNKNOWN_REF;
        let compile_result = YamlCompiler::default().compile(source);
        let validate_result = vb_yd5x_validate_via_compile(source);
        assert!(
            compile_result.is_err(),
            "compile should fail for unknown ref"
        );
        assert!(
            validate_result.is_err(),
            "validate should fail for unknown ref"
        );
        let compile_code = compile_result.unwrap_err().first().map(|e| e.code());
        let validate_code = validate_result.unwrap_err().first().map(|e| e.code());
        assert_eq!(
            compile_code, validate_code,
            "compile and validate should produce same code"
        );
        assert_eq!(
            compile_code,
            Some("UNKNOWN_REFERENCE"),
            "expected UNKNOWN_REFERENCE"
        );
    }

    #[test]
    fn vb_yd5x_diagnostic_codes_remain_stable() {
        // Test that error codes are stable across paths
        let test_cases = [
            (VB_YD5X_MALFORMED_SLOT_REF, "TYPE_MISMATCH"),
            (VB_YD5X_MALFORMED_LOOP_BODY, "INVALID_THEN_TARGET"),
            (VB_YD5X_MALFORMED_DUPLICATE_ID, "DUPLICATE_ID"),
            (VB_YD5X_MALFORMED_UNKNOWN_REF, "UNKNOWN_REFERENCE"),
        ];
        for (source, expected_code) in test_cases {
            let compile_result = YamlCompiler::default().compile(source);
            let validate_result = vb_yd5x_validate_via_compile(source);
            let compile_code = compile_result
                .as_ref()
                .err()
                .and_then(|e| e.first())
                .map(|e| e.code());
            let validate_code = validate_result
                .as_ref()
                .err()
                .and_then(|e| e.first())
                .map(|e| e.code());
            assert_eq!(
                compile_code, validate_code,
                "codes should match for {expected_code}"
            );
            assert_eq!(
                compile_code,
                Some(expected_code),
                "expected {expected_code}"
            );
        }
    }
}
