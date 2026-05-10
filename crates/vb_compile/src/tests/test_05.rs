#![forbid(unsafe_code)]
use super::helpers::*;

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
    fn compiler_rejects_invalid_step_ids_empty() {
        let id = "";
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

    #[test]
    fn compiler_rejects_invalid_step_ids_build_result() {
        let id = "BuildResult";
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

    #[test]
    fn compiler_rejects_invalid_step_ids_kebab_case() {
        let id = "build-result";
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

    #[test]
    fn compiler_rejects_invalid_step_ids_finish() {
        let id = "finish";
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
    fn compiler_rejects_unsupported_phase_zero_step_control_fields_if() {
        let control = "if";
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

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields_with() {
        let control = "with";
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

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields_try_again() {
        let control = "try_again";
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

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields_on_error() {
        let control = "on_error";
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

    #[test]
    fn compiler_rejects_unsupported_phase_zero_step_control_fields_then() {
        let control = "then";
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
    fn compiler_rejects_invalid_workflow_trigger_shapes_scalar_manual() {
        let source = b"version: velvet-ballastics/v1\nname: fast_path\nwhen: manual\nsteps:\n  - finish:\n      result: 0\n";
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_shapes_empty_map() {
        let source = b"version: velvet-ballastics/v1\nname: fast_path\nwhen: {}\nsteps:\n  - finish:\n      result: 0\n";
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_shapes_multiple_triggers() {
        let source = b"version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  manual: {}\n  event: {}\nsteps:\n  - finish:\n      result: 0\n";
        let result = YamlCompiler::default().compile(source);

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::FieldShape { .. } | CompileError::InvalidTriggerCount { .. }))
        ));
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
    fn compiler_rejects_scalar_workflow_trigger_config_manual() {
        let trigger = "manual";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
            "trigger {trigger} config must be mapping-shaped"
        );
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config_webhook() {
        let trigger = "webhook";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
            "trigger {trigger} config must be mapping-shaped"
        );
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config_schedule() {
        let trigger = "schedule";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
            "trigger {trigger} config must be mapping-shaped"
        );
    }

    #[test]
    fn compiler_rejects_scalar_workflow_trigger_config_event() {
        let trigger = "event";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n  {trigger}: true\nsteps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::TriggerShape { .. }))),
            "trigger {trigger} config must be mapping-shaped"
        );
    }
