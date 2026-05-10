#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compile_returns_empty_steps_for_steps_with_empty_list() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: empty\nwhen:\n  manual: {}\nsteps: []");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::EmptySteps)));
    }

    #[test]
    fn compile_returns_invalid_version_for_wrong_version() {
        let result = YamlCompiler::default().compile(
            b"version: bad-version\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidVersion { actual }) = errors.first() else {
            compile_test_fail!("expected InvalidVersion, got {:?}", errors.first());
        };
        assert_eq!(actual.as_ref(), "bad-version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_version() {
        let result = YamlCompiler::default().compile(
            b"name: no_version\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "version");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "name");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_trigger\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "when");
    }

    #[test]
    fn compile_returns_missing_field_for_absent_steps() {
        let result = YamlCompiler::default()
            .compile(b"version: velvet-ballastics/v1\nname: no_steps\nwhen:\n  manual: {}");
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingField { field }) = errors.first() else {
            compile_test_fail!("expected MissingField, got {:?}", errors.first());
        };
        assert_eq!(*field, "steps");
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_empty_when() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: empty_when\nwhen: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            compile_test_fail!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 0);
    }

    #[test]
    fn compile_returns_unknown_trigger_kind_for_invalid_trigger() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_trigger\nwhen:\n  teleport: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownTriggerKind { trigger }) = errors.first() else {
            compile_test_fail!("expected UnknownTriggerKind, got {:?}", errors.first());
        };
        assert_eq!(trigger.as_ref(), "teleport");
    }

    #[test]
    fn compile_returns_missing_step_id_for_step_without_id() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_id\nwhen:\n  manual: {}\nsteps:\n  - finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingStepId { step }) = errors.first() else {
            compile_test_fail!("expected MissingStepId, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_step_shape_for_non_mapping_step() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_step\nwhen:\n  manual: {}\nsteps:\n  - \"scalar\"",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::StepShape { step }) = errors.first() else {
            compile_test_fail!("expected StepShape, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

