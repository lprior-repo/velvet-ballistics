#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compile_returns_duplicate_step_id_for_same_ids() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup_step\nwhen:\n  manual: {}\nsteps:\n  - id: same\n    save:\n      x: 1\n  - id: same\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DuplicateStepId { id }) = errors.first() else {
            compile_test_fail!("expected DuplicateStepId, got {:?}", errors.first());
        };
        assert_eq!(id.as_ref(), "same");
    }

    #[test]
    fn compile_returns_missing_step_primitive_for_step_without_primitive() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_prim\nwhen:\n  manual: {}\nsteps:\n  - id: empty_step",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MissingStepPrimitive { step }) = errors.first() else {
            compile_test_fail!("expected MissingStepPrimitive, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_unknown_step_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_field\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    unknown_field: 1\n    save:\n      x: 1",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownStepField { step, field }) = errors.first() else {
            compile_test_fail!("expected UnknownStepField, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
        assert_eq!(field.as_ref(), "unknown_field");
    }

    #[test]
    fn compile_returns_last_step_must_finish_for_non_finish_ending() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: no_finish\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::LastStepMustFinish)
        ));
    }

    #[test]
    fn compile_returns_unknown_top_level_field_for_invalid_field() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: extra\nwhen:\n  manual: {}\nunknown_root: true\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::UnknownTopLevelField { field }) = errors.first() else {
            compile_test_fail!("expected UnknownTopLevelField, got {:?}", errors.first());
        };
        assert_eq!(field.as_ref(), "unknown_root");
    }

    #[test]
    fn compile_returns_tag_forbidden_for_tagged_node() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: tagged\nwhen:\n  manual: {}\nsteps:\n  - id: !!tag done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(
            errors.first(),
            Some(CompileError::TagForbidden { .. })
        ));
    }

    #[test]
    fn compile_returns_float_forbidden_for_float_scalar() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: floaty\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 3.14",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        assert!(matches!(errors.first(), Some(CompileError::FloatForbidden)));
    }

    #[test]
    fn compile_returns_depth_limit_for_deeply_nested_yaml() {
        let tiny_limits = YamlLimits {
            max_depth: 3,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: deep\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\na:\n  b:\n    c:\n      d: deep",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DepthLimit { depth, limit }) = errors.first() else {
            compile_test_fail!("expected DepthLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 3);
        assert!(*depth > 3);
    }

    #[test]
    fn compile_returns_node_limit_for_many_nodes() {
        let tiny_limits = YamlLimits {
            max_nodes: 5,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: big\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      a: 1\n      b: 2\n      c: 3\n      d: 4\n      e: 5\n      f: 6\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::NodeLimit { limit }) = errors.first() else {
            compile_test_fail!("expected NodeLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
    }

    #[test]
    fn compile_returns_scalar_limit_for_long_scalar() {
        let tiny_limits = YamlLimits {
            max_scalar_bytes: 5,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler {
            limits: tiny_limits,
        };
        let result = compiler.compile(
            b"version: velvet-ballastics/v1\nname: long_scalar\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\nlabel: abcdefgh",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::ScalarLimit { actual, limit }) = errors.first() else {
            compile_test_fail!("expected ScalarLimit, got {:?}", errors.first());
        };
        assert_eq!(*limit, 5);
        assert!(*actual > 5);
    }

