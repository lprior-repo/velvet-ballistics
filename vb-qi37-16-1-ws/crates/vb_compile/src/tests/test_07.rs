#![forbid(unsafe_code)]
use super::helpers::*;

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
    fn compiler_accepts_for_each_with_at_once_field() -> Result<(), String> {
        let source = "version: velvet-ballastics/v1\nname: for_each_with_at_once\nwhen:\n  manual: {}\nsteps:\n  - id: list\n    save:\n      value: [1, 2, 3]\n  - id: each\n    for_each:\n      input: 0\n      item: 1\n      limit: 10\n      at_once: 5\n  - id: done\n    finish:\n      result: 0\n";
        let workflow = YamlCompiler::default()
            .compile(source.as_bytes())
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let start = workflow
            .node(StepIdx::new(1))
            .ok_or("missing for_each start")?;
        assert!(
            matches!(start.kind, CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } if input == SlotIdx::ZERO && item_slot == SlotIdx::new(1) && limit == 10 && body == StepIdx::new(2) && done == StepIdx::new(3)),
            "for_each start node must have correct structure"
        );
        Ok(())
    }

