#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn malformed_expression_produces_deterministic_parse_error() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: bad_expr
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "1 +"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
        adv_ensure(
            compile_error_text(source) == parse_ast_error_text(source),
            "compile and parse_ast diverged on malformed expression",
        )
    }

    /// Attack vector: Two steps writing to the same slot index.
    /// In Phase 0, save steps write to their step index as slot.
    /// Steps 0 and 1 write to slot 0 and slot 1 respectively, so no collision.
    /// But a finish referencing slot 0 when step 0 saved value 1 is valid.
    /// Test that the compiler handles slot layout correctly.
    #[test]
    fn slot_layout_two_saves_finish_reads_first_slot() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: slot_layout
when:
  manual: {}
steps:
  - id: first
    save:
      value: 10
  - id: second
    save:
      value: 20
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let node = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;
        // The finish should read slot 0 (from first save step)
        match &node.kind {
            CompiledNodeKind::Finish { result } if result.get() == 0 => Ok(()),
            other => Err(format!("finish did not reference slot 0: {other:?}")),
        }
    }

    /// Attack vector: Non-last finish step rejected with exact diagnostic.
    #[test]
    fn finish_in_middle_position_rejected_with_step_field_shape() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: mid_finish
when:
  manual: {}
steps:
  - id: early
    finish:
      result: 0
  - id: late
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::StepFieldShape {
                        step: 0,
                        field: "finish",
                        expected: "the last step",
                    }
                ),
                "mid-position finish did not produce exact StepFieldShape diagnostic",
            )
        })
    }

    /// Attack vector: Choose with negative branch target rejected.
    #[test]
    fn choose_negative_branch_target_rejected_with_out_of_range() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: neg_target
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: true
      on_true: -1
      on_false: 1
  - id: done
    finish:
      result: 0
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::BranchTargetOutOfRange { value: -1 }),
            "negative branch target did not produce BranchTargetOutOfRange",
        )
    }

    /// Attack vector: Choose with branch target exceeding step count.
    #[test]
    fn choose_branch_target_exceeding_step_count_rejected() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: exceed_target
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::UnknownStepTarget { step: 1, target: 3 }
                ),
                "branch target exceeding step count did not produce UnknownStepTarget",
            )
        })
    }

    /// Attack vector: Multiple diagnostics in a single pass -- reference errors
    /// in examples and steps should accumulate.
    #[test]
    fn multiple_reference_errors_accumulate_in_compile() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: multi_error
when:
  manual: {}
inputs:
  user: text
examples:
  - name: bad1
    value: $input.missing_one
  - name: bad2
    value: $input.missing_two
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        let result = YamlCompiler::default().compile(source);
        let Err(errors) = result else {
            return Err("expected compile error".to_owned());
        };
        // Should have at least 2 errors (one for each missing input reference)
        adv_ensure(
            errors.len() >= 2,
            "expected at least 2 accumulated reference errors",
        )?;
        adv_ensure(
            errors.iter().all(|e| {
                matches!(e, CompileError::UnknownReferenceName { kind: "input", .. })
            }),
            "accumulated error was not an input reference error",
        )
    }
    }

    /// Attack vector: Expression with deeply nested parentheses hits depth limit.
    #[test]
    fn deeply_nested_expression_hits_parse_depth_limit() -> Result<(), String> {
        let depth = 70;
        let opens = "(".repeat(depth);
        let closes = ")".repeat(depth);
        let expr = format!("{opens}true{closes}");
        let source = format!(
            "version: velvet-ballastics/v1\nname: deep_expr\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition: \"{expr}\"\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n"
        );
        let error = adv_compile_error(source.as_bytes())?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLimitExceeded {
                    limit: "parse depth",
                    ..
                }
            ),
            "deeply nested expression did not hit parse depth limit",
        )
    }

    /// Attack vector: Expression exceeding token limit rejected.
    #[test]
    fn long_expression_hits_token_limit() -> Result<(), String> {
        // Generate an expression with more than 256 tokens
        let parts: Vec<&str> = (0..300).map(|_| "1").collect();
        let expr = parts.join(" + ");
        let source = format!(
            "version: velvet-ballastics/v1\nname: token_limit\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition: \"{expr}\"\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n"
        );
        let error = adv_compile_error(source.as_bytes())?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLimitExceeded {
                    limit: "token count",
                    ..
                }
            ),
            "long expression did not hit token count limit",
        )
    }

    /// Attack vector: Expression exceeding source length limit rejected.
    #[test]
    fn oversized_expression_hits_source_length_limit() -> Result<(), String> {
        // 4096+ character expression
        let expr = "1".repeat(4097);
        let source = format!(
            "version: velvet-ballastics/v1\nname: expr_len\nwhen:\n  manual: {{}}\nsteps:\n  - id: route\n    choose:\n      condition: \"{expr}\"\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish:\n      result: true\n"
        );
        let error = adv_compile_error(source.as_bytes())?;
        adv_ensure(
            matches!(
                error,
                CompileError::ExpressionLimitExceeded {
                    limit: "source length",
                    ..
                }
            ),
            "oversized expression did not hit source length limit",
        )
    }

    /// Attack vector: Choose with self-referencing target rejected.
    #[test]
    fn choose_self_referencing_target_rejected_with_backward_branch() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: self_ref
when:
  manual: {}
steps:
  - id: first
    save:
      value: true
  - id: route
    choose:
      condition: true
      on_true: 1
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        adv_ensure_parity(source, |error| {
            adv_ensure(
                matches!(
                    error,
                    CompileError::BackwardBranchTarget { step: 1, target: 1 }
                ),
                "self-referencing branch did not produce exact backward target diagnostic",
            )
        })
    }

    /// Attack vector: Finish with integer 65536 that exceeds u16 slot range.
    /// Since 65536 > step index 0, it's treated as a literal value, not a slot.
    /// The Phase 0 compiler emits it as ConstValue::I64(65536) and compiles.
