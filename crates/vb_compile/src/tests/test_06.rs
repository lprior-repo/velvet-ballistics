#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs_manual() {
        let when_body = "  manual: {}\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
            "valid trigger should compile"
        );
    }

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs_webhook() {
        let when_body = "  webhook:\n    path: /github\n    method: POST\n    unique: request.header.X-GitHub-Delivery\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
            "valid trigger should compile"
        );
    }

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs_schedule() {
        let when_body = "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: UTC\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
            "valid trigger should compile"
        );
    }

    #[test]
    fn compiler_accepts_valid_workflow_trigger_configs_event() {
        let when_body = "  event:\n    name: customer.created\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(
            matches!(result, Ok(ref workflow) if workflow.name() == "fast_path"),
            "valid trigger should compile"
        );
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields_manual() {
        let when_body = "  manual:\n    extra: true\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields_webhook() {
        let when_body = "  webhook:\n    path: /github\n    method: POST\n    extra: true\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields_schedule() {
        let when_body = "  schedule:\n    cron: \"*/5 * * * *\"\n    extra: true\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_unknown_workflow_trigger_fields_event() {
        let when_body = "  event:\n    name: customer.created\n    extra: true\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::UnknownTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields_webhook_method() {
        let when_body = "  webhook:\n    method: POST\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields_webhook_path() {
        let when_body = "  webhook:\n    path: /github\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields_schedule_timezone() {
        let when_body = "  schedule:\n    timezone: UTC\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_missing_required_workflow_trigger_fields_event_empty() {
        let when_body = "  event: {}\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::MissingTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_webhook_relative_path() {
        let when_body = "  webhook:\n    path: github\n    method: POST\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_webhook_invalid_method() {
        let when_body = "  webhook:\n    path: /github\n    method: TRACE\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_webhook_numeric_path() {
        let when_body = "  webhook:\n    path: 42\n    method: POST\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_webhook_numeric_unique() {
        let when_body = "  webhook:\n    path: /github\n    method: POST\n    unique: 42\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_schedule_invalid_cron() {
        let when_body = "  schedule:\n    cron: \"0 0 0 0 0 0\"\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_schedule_numeric_cron() {
        let when_body = "  schedule:\n    cron: 42\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_schedule_numeric_timezone() {
        let when_body = "  schedule:\n    cron: \"*/5 * * * *\"\n    timezone: 42\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
    }

    #[test]
    fn compiler_rejects_invalid_workflow_trigger_field_values_event_numeric_name() {
        let when_body = "  event:\n    name: 42\n";
        let source = format!(
            "version: velvet-ballastics/v1\nname: fast_path\nwhen:\n{when_body}steps:\n  - finish:\n      result: 0\n"
        );
        let result = YamlCompiler::default().compile(source.as_bytes());

        assert!(matches!(
            result,
            Err(ref errors) if matches!(errors.first(), Some(CompileError::InvalidTriggerField { .. }))
        ));
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
