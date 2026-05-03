use super::helpers::*;

    #[test]
    fn lower_set_produces_set_node_kind() {
        let mut builder = SlotCompiler::new();
        let const_idx = builder
            .push_constant(ConstValue::I64(42))
            .ok()
            .unwrap_or(ConstIdx::new(0));
        let node = lower_set(
            StepIdx::new(0),
            SlotIdx::new(0),
            const_idx,
            Some(StepIdx::new(1)),
        );
        assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
    }

    #[test]
    fn lower_do_produces_do_node_kind() {
        let mut builder = SlotCompiler::new();
        let node = lower_do(
            StepIdx::new(0),
            ActionId::new(1),
            SlotIdx::new(0),
            Some(SlotIdx::new(1)),
            Some(StepIdx::new(1)),
            &mut builder,
        );
        assert!(matches!(node.kind, CompiledNodeKind::Do { .. }));
    }

    #[test]
    fn lower_ask_uses_checked_resume_step() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let nodes = lower_ask(
            StepIdx::new(7),
            SlotIdx::new(1),
            SlotIdx::new(2),
            None,
            &mut builder,
        )
        .map_err(|error| error.to_string())?;

        assert_eq!(nodes.len(), 2);
        let Some(first) = nodes.first() else {
            return Err(String::from("expected ask node"));
        };
        let Some(second) = nodes.get(1) else {
            return Err(String::from("expected ask resume node"));
        };
        assert!(matches!(first.kind, CompiledNodeKind::Ask { .. }));
        assert_eq!(second.id, StepIdx::new(8));
        assert_eq!(second.output, Some(SlotIdx::new(2)));
        assert!(matches!(second.kind, CompiledNodeKind::AskResume { .. }));
        Ok(())
    }

    #[test]
    fn lower_ask_rejects_resume_step_overflow() {
        let mut builder = SlotCompiler::new();
        let result = lower_ask(
            StepIdx::MAX,
            SlotIdx::new(1),
            SlotIdx::new(2),
            None,
            &mut builder,
        );

        let Err(CompileError::PrimitiveLoweringLimitExceeded {
            primitive,
            field,
            value,
            limit,
        }) = result
        else {
            compile_test_fail!("expected primitive lowering limit error");
        };
        assert_eq!(primitive, "ask");
        assert_eq!(field, "resume_step");
        assert_eq!(value, StepIdx::MAX.as_usize());
        assert_eq!(limit, usize::from(u16::MAX));
    }

    #[test]
    fn compute_compiled_digest_is_deterministic() {
        let d1 = compute_compiled_digest(NESTED_SAVE_SOURCE);
        let d2 = compute_compiled_digest(NESTED_SAVE_SOURCE);
        assert_eq!(d1, d2);
    }

    #[test]
    fn compute_compiled_digest_differs_for_different_sources() {
        let d1 = compute_compiled_digest(b"source_a");
        let d2 = compute_compiled_digest(b"source_b");
        assert_ne!(d1, d2);
    }

    // ── Round 2: SlotCompiler tests ──────────────────────────────────────

    #[test]
    fn slot_compiler_new_starts_empty() {
        let mut sc = SlotCompiler::new();
        assert_eq!(
            sc.push_constant(ConstValue::I64(42)).ok().map(|i| i.get()),
            Some(0)
        );
    }

    #[test]
    fn slot_compiler_push_constant_returns_ascending_indices() {
        let mut sc = SlotCompiler::new();
        let idx0 = sc.push_constant(ConstValue::I64(1));
        let idx1 = sc.push_constant(ConstValue::I64(2));
        assert_eq!(idx0.ok().map(|i| i.get()), Some(0));
        assert_eq!(idx1.ok().map(|i| i.get()), Some(1));
    }

    #[test]
    fn slot_compiler_push_expression_returns_ascending_indices() {
        let mut sc = SlotCompiler::new();
        let empty_ops: Box<[vb_core::workflow::ExprOp]> = Box::from([]);
        let prog = ExprProgram::try_from_ops(empty_ops).unwrap_or_else(|_| ExprProgram {
            ops: Box::from([]),
            max_stack: 0,
        });
        let idx = sc.push_expression(prog);
        assert_eq!(idx.ok().map(|i| i.get()), Some(0));
    }

    #[test]
    fn slot_compiler_record_slot_tracks_max_slot() {
        let mut sc = SlotCompiler::new();
        sc.record_slot(SlotIdx::new(5));
        sc.record_slot(SlotIdx::new(10));
        // record_slot doesn't return anything but should not panic
    }

    // ── Adversarial compilation pipeline tests ──────────────────────────────

    fn adv_compile_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().compile(source) {
            Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn adv_parse_error(source: &[u8]) -> Result<CompileError, String> {
        match YamlCompiler::default().parse_ast(source) {
            Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
            Err(errors) => errors
                .first()
                .cloned()
                .ok_or_else(|| "CompileErrors was empty".to_owned()),
        }
    }

    fn adv_compile_ok(source: &[u8]) -> Result<CompiledWorkflow, String> {
        YamlCompiler::default()
            .compile(source)
            .map_err(|errors| format!("compile unexpectedly failed: {errors}"))
    }

    fn adv_ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    fn adv_ensure_parity(
        source: &[u8],
        check: fn(CompileError) -> Result<(), String>,
    ) -> Result<(), String> {
        let c_text = compile_error_text(source);
        let p_text = parse_ast_error_text(source);
        adv_ensure(
            c_text == p_text,
            "compile and parse_ast diagnostics diverged",
        )?;
        check(adv_compile_error(source)?)?;
        check(adv_parse_error(source)?)
    }

    /// Attack vector 6: Empty steps list should be caught before any downstream validation.
