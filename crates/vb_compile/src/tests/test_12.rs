use super::helpers::*;

    #[test]
    fn compile_returns_duplicate_key_for_repeated_yaml_key() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: dup\nwhen:\n  manual: {}\nname: dup2\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::DuplicateKey { key, .. }) = errors.first() else {
            compile_test_fail!("expected DuplicateKey, got {:?}", errors.first());
        };
        assert_eq!(key.as_ref(), "name");
    }

    #[test]
    fn compile_returns_invalid_name_for_reserved_step_name() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: reserved\nwhen:\n  manual: {}\nsteps:\n  - id: run\n    save:\n      x: 1\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidName { field, value }) = errors.first() else {
            compile_test_fail!("expected InvalidName, got {:?}", errors.first());
        };
        assert_eq!(*field, "step id");
        assert_eq!(value.as_ref(), "run");
    }

    #[test]
    fn compile_returns_multiple_step_primitives_for_two_primitives() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi\nwhen:\n  manual: {}\nsteps:\n  - id: s1\n    save:\n      x: 1\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::MultipleStepPrimitives { step }) = errors.first() else {
            compile_test_fail!("expected MultipleStepPrimitives, got {:?}", errors.first());
        };
        assert_eq!(*step, 0);
    }

    #[test]
    fn compile_returns_invalid_trigger_count_for_two_triggers() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: multi_trigger\nwhen:\n  manual: {}\n  ipc: {}\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::InvalidTriggerCount { count }) = errors.first() else {
            compile_test_fail!("expected InvalidTriggerCount, got {:?}", errors.first());
        };
        assert_eq!(*count, 2);
    }

    #[test]
    fn compile_returns_field_shape_for_bad_inputs_shape() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: bad_inputs\nwhen:\n  manual: {}\ninputs: []\nsteps:\n  - id: done\n    finish:\n      result: 0",
        );
        let Err(errors) = result else {
            compile_test_fail!("expected error")
        };
        let Some(CompileError::FieldShape { field, expected }) = errors.first() else {
            compile_test_fail!("expected FieldShape, got {:?}", errors.first());
        };
        assert_eq!(*field, "inputs");
        assert!(!expected.is_empty());
    }

    // ── Round 2: Compilation success path tests ──────────────────────────

    #[test]
    fn compile_produces_valid_workflow_for_minimal_source() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
    }

    #[test]
    fn compile_produces_valid_workflow_for_optional_fields() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok, got {:?}", result)
        };
        assert_eq!(wf.node_count(), 2);
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_non_default_workflow_digest() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_ne!(
            wf.digest(),
            vb_core::ids::WorkflowDigest::from_bytes([0u8; 32])
        );
    }

    #[test]
    fn compile_produces_matching_workflow_name() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_eq!(wf.name(), "fast_path");
    }

    #[test]
    fn compile_produces_correct_entry_step_index() {
        let result = YamlCompiler::default().compile(OPTIONAL_TOP_LEVEL_FIELDS_SOURCE);
        let Ok(wf) = result else {
            compile_test_fail!("expected Ok")
        };
        assert_eq!(wf.entry(), vb_core::ids::StepIdx::ZERO);
    }

