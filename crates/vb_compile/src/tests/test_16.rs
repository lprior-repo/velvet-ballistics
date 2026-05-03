use super::helpers::*;

    #[test]
    fn empty_steps_list_rejected_with_exact_error() -> Result<(), String> {
        let source =
            b"version: velvet-ballastics/v1\nname: empty_case\nwhen:\n  manual: {}\nsteps: []\n";
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::EmptySteps),
            "empty steps did not produce EmptySteps diagnostic",
        )
    }

    /// Attack vector 17: Workflow with only a single finish step and no other steps.
    #[test]
    fn single_finish_step_only_workflow_compiles_cleanly() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: single_finish\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: true\n";
        let workflow = adv_compile_ok(source)?;
        // Should produce 2 nodes: SetConst(true) + Finish(slot 0)
        adv_ensure(
            workflow.node_count() == 2,
            "single finish should produce 2 IR nodes",
        )
    }

    /// Attack vector 11: Missing finish step -- last step is a save.
    #[test]
    fn missing_finish_step_rejected_with_exact_last_step_must_finish() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: no_finish
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::LastStepMustFinish),
            "missing finish did not produce LastStepMustFinish",
        )
    }

    /// Attack vector 7: Finish step references an input not declared but used.
    #[test]
    fn finish_referencing_undeclared_input_rejected_by_reference_pass() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: missing_input_ref
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $input.nonexistent
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(
                error,
                CompileError::UnknownReferenceName { kind: "input", .. }
            ),
            "undeclared input reference did not produce UnknownReferenceName diagnostic",
        )
    }

    /// Attack vector 8: Choose branches creating unreachable dead code (both branches skip a step).
    #[test]
    fn choose_both_branches_skip_produces_unreachable_step() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: unreachable_dead_code
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
      on_false: 3
  - id: dead
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::UnreachableStep { step: 2 }),
            "unreachable dead step did not produce exact UnreachableStep diagnostic",
        )
    }

    /// Attack vector 1 (approximation): Source byte limit hit produces SourceTooLarge.
    #[test]
    fn oversized_workflow_source_rejected_with_source_too_large() -> Result<(), String> {
        let tiny_limits = YamlLimits {
            max_source_bytes: 100,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler::new(tiny_limits);
        let mut source =
            String::from("version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n");
        // Add enough steps to exceed 100 bytes
        for i in 0..20 {
            source.push_str(&format!("  - id: s{i}\n    save:\n      value: 1\n"));
        }
        source.push_str("  - id: done\n    finish:\n      result: 0\n");
        let result = compiler.compile(source.as_bytes());
        let Err(errors) = result else {
            return Err("expected compile error for oversized source".to_owned());
        };
        adv_ensure(
            matches!(errors.first(), Some(CompileError::SourceTooLarge { .. })),
            "oversized source did not produce SourceTooLarge",
        )
    }

    /// Attack vector 9 (approximation): Constant pool overflow through many save steps.
    /// With default limits this is too large to test, but we verify the constant
    /// pool tracks correctly for a modest number of steps.
    #[test]
    fn many_save_steps_compile_with_correct_node_count() -> Result<(), String> {
        let mut source = String::from(
            "version: velvet-ballastics/v1\nname: many_saves\nwhen:\n  manual: {}\nsteps:\n",
        );
        let step_count: usize = 50;
        for i in 0..step_count {
            source.push_str(&format!("  - id: s{i}\n    save:\n      value: {i}\n"));
        }
        // Finish with literal 0 (treated as slot 0, which is written by save step 0)
        source.push_str("  - id: done\n    finish:\n      result: 0\n");
        let workflow = adv_compile_ok(source.as_bytes())?;
        // Each save produces 1 node, finish with slot 0 produces 1 node
        let expected = step_count + 1;
        adv_ensure(
            usize::from(workflow.node_count()) == expected,
            "node count mismatch for many saves",
        )
    }

    /// Attack vector 12: Choose condition referencing undefined input via expression string.
    #[test]
    fn choose_expression_referencing_undefined_input_rejected() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: choose_undefined_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: "$input.nonexistent == true"
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(
                error,
                CompileError::UnknownReferenceName { kind: "input", .. }
            ),
            "undefined input in choose expression did not produce reference diagnostic",
        )
    }

    /// Attack vector 5: Reference resolution with shadowed-looking variable names.
    /// Step IDs and input names should not collide.
    #[test]
    fn step_id_does_not_shadow_input_reference() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: shadow_test
when:
  manual: {}
inputs:
  value: text
steps:
  - id: value
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        // This should compile fine because step IDs and input references
        // are in separate namespaces ($input.value vs step id "value").
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    /// Attack vector 3 approximation: Nested choose creates multiple branch targets.
    #[test]
    fn nested_choose_branches_compile_with_correct_ir() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: nested_choose
when:
  manual: {}
steps:
  - id: outer_flag
    save:
      value: true
  - id: inner_flag
    save:
      value: false
  - id: route_outer
    choose:
      condition: 0
      on_true: 3
      on_false: 4
  - id: route_inner
    choose:
      condition: 1
      on_true: 5
      on_false: 5
  - id: alt_path
    save:
      value: 2
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        // 3 saves + 2 chooses + 1 finish(slot 0) = 6 nodes
        let expected = 6u16;
        adv_ensure(
            workflow.node_count() == expected,
            "nested choose did not produce correct node count",
        )
    }

    /// Attack vector 10: Accessor path with deeply nested numeric segments.
