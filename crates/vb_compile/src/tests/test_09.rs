use super::helpers::*;

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

