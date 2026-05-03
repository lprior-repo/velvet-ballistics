use super::helpers::*;

    #[test]
    fn deep_numeric_accessor_path_accepted_by_reference_pass() -> Result<(), String> {
        // Build a deeply nested numeric path: $slot.0.1.2.3.4.5.6.7.8.9.10.11.12.13.14.15
        let source = br#"version: velvet-ballastics/v1
name: deep_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
examples:
  - name: fixture
    value: $slot.0.1.2.3.4.5.6.7.8.9.10.11.12.13.14.15
"#;
        // Should pass reference validation because numeric accessor paths are allowed
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    /// Attack vector: Non-numeric accessor path segment rejected.
    #[test]
    fn non_numeric_accessor_path_in_slot_rejected_with_unsupported_accessor() -> Result<(), String>
    {
        let source = br#"version: velvet-ballastics/v1
name: field_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.field_name
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(error, CompileError::UnsupportedAccessorReference { root, path, .. }
                    if root.as_ref() == "slot.0" && path.as_ref() == "field_name"),
                "field accessor did not produce UnsupportedAccessorReference",
            )
        })
    }

    /// Attack vector: Illegal $steps.done reference in examples.
    #[test]
    fn steps_reference_in_examples_rejected_as_illegal() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: illegal_steps_ref
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
examples:
  - name: fixture
    value: $steps.done
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::IllegalReference { .. }),
            "steps reference did not produce IllegalReference diagnostic",
        )
    }

    /// Attack vector: $runtime.now in choose condition is rejected.
    #[test]
    fn runtime_now_in_choose_condition_rejected_as_illegal() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: runtime_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$runtime.now == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::IllegalReference { .. }),
            "runtime.now in choose did not produce IllegalReference",
        )
    }

    /// Attack vector: Bare $now reference is rejected.
    #[test]
    fn bare_now_reference_in_finish_rejected_as_illegal() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: bare_now
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $now
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::IllegalReference { reference } if reference.as_ref() == "$now"),
            "bare $now did not produce IllegalReference diagnostic",
        )
    }

    /// Attack vector: Unknown reference root $env.HOME rejected.
    #[test]
    fn unknown_reference_root_env_rejected_with_unknown_root() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: env_ref
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $env.HOME
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::UnknownReferenceRoot { root, .. } if root.as_ref() == "env"),
            "$env.HOME did not produce UnknownReferenceRoot with root=env",
        )
    }

    /// Attack vector: Secret reference in finish result leaks taint.
    #[test]
    fn secret_in_finish_object_rejected_with_taint_leak() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: taint_leak
when:
  manual: {}
secrets:
  key: SECRET_KEY
steps:
  - id: done
    finish:
      result:
        token: $secrets.key
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::SecretTaintLeak {
                        field: "finish.result"
                    }
                ),
                "secret in finish object did not produce taint leak",
            )
        })
    }

    /// Attack vector: Choose condition with non-boolean type (number literal in slot).
    #[test]
    fn choose_numeric_slot_condition_rejected_with_type_mismatch() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: num_choose
when:
  manual: {}
steps:
  - id: num
    save:
      value: 42
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::TypeMismatch {
                        field: "choose.condition",
                        expected: "boolean",
                        found: "number",
                    }
                ),
                "numeric slot condition did not produce type mismatch",
            )
        })
    }

    /// Attack vector: Finish slot referencing a forward (uninitialized) slot.
    #[test]
    fn finish_forward_slot_reference_rejected_with_unknown_slot() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: forward_slot
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 1
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::UnknownSlotType {
                        field: "finish.result",
                        slot: 1
                    }
                ),
                "forward finish slot did not produce unknown slot diagnostic",
            )
        })
    }

    /// Attack vector: Expression helper with wrong arity (contains with 3 args).
    /// Expression parsing accepts the call but arity is checked during lowering.
    /// In the Phase 0 pipeline, expressions are retained in the AST without
    /// bytecode lowering, so arity is only checked when expression lowering runs.
    #[test]
    fn expression_helper_wrong_arity_rejected_in_bytecode_lowering() -> Result<(), String> {
        use crate::expression::parse_expression;
        use crate::expression_bytecode::compile_expr_to_bytecode;

        let expr = parse_expression("contains(1, 2, 3)").map_err(|e| format!("parse: {e:?}"))?;
        let mut constants = Vec::new();
        let error = compile_expr_to_bytecode(&expr, &mut constants)
            .map(|_| "unexpected success".to_owned())
            .unwrap_or_else(|e| e.to_string());
        adv_ensure(
            error.contains("contains") && error.contains("expects 2") && error.contains("found 3"),
            "helper arity mismatch did not produce exact diagnostic",
        )
    }

    /// Attack vector: Expression parse error (incomplete expression) produces
    /// deterministic diagnostic with compile/parse parity.
