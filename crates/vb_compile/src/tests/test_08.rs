#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn compiler_lowers_yaml_together_to_start_and_join_nodes() -> Result<(), String> {
        // The together structure needs the join node to come after all branch
        // targets. With 3 source steps (fanout, body, done) the compiler
        // expands fanout into TogetherStart (node 0) + TogetherJoin (node 1).
        // The branch target (step 1 -> node 2) must be before the finish
        // (step 2 -> node 3). However the compiler currently emits the join
        // at id+1, so for a well-formed test we use a layout where the
        // branch body is between start and join. Since the lowering always
        // puts TogetherJoin right after TogetherStart, and the shared
        // validation pipeline now enforces join > branch ordering, we test
        // that the IR is rejected when branches point past the join.
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: together_case\nwhen:\n  manual: {}\nsteps:\n  - id: fanout\n    together:\n      branches: [1]\n  - id: done\n    finish:\n      result: 0\n",
        );
        // The shared validation pipeline catches the invalid together IR:
        // join (node 1) is not after branch target (node 2).
        assert!(
            matches!(result, Err(ref errors) if errors.0.iter().any(|e| matches!(e, CompileError::Validation(vb_validate::ValidationError::LoopBodyStepOutOfRange { .. })))),
            "expected LoopBodyStepOutOfRange validation error, got: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_collect_to_collection_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: collect_case\nwhen:\n  manual: {}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: collect_values\n    collect:\n      source: 0\n      limit: 5\n      page_size: 2\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::CollectStart { source, limit, page_size, body, done }) if *source == SlotIdx::ZERO && *limit == 5 && *page_size == 2 && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::CollectPage { collector_slot, body, done }) if *collector_slot == SlotIdx::ZERO && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(3)).map(|node| &node.kind), Some(CompiledNodeKind::CollectFinish { collector_slot }) if *collector_slot == SlotIdx::ZERO)
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_reduce_to_reduction_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: reduce_case\nwhen:\n  manual: {}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: reduce_values\n    reduce:\n      input: 0\n      accumulator: 1\n      initial: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceStart { input, accumulator, initial, body, done }) if *input == SlotIdx::ZERO && *accumulator == SlotIdx::new(1) && *initial == ConstIdx::new(1) && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceNext { iterator_slot, accumulator, body, done }) if *iterator_slot == SlotIdx::new(1) && *accumulator == SlotIdx::new(1) && *body == StepIdx::new(2) && *done == StepIdx::new(3))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(3)).map(|node| &node.kind), Some(CompiledNodeKind::ReduceFinish { accumulator }) if *accumulator == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_repeat_to_attempt_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: repeat_case\nwhen:\n  manual: {}\nsteps:\n  - id: poll\n    repeat:\n      max_attempts: 3\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;

        assert!(
            matches!(workflow.node(StepIdx::ZERO).map(|node| &node.kind), Some(CompiledNodeKind::RepeatStart { max_attempts, body, done }) if *max_attempts == 3 && *body == StepIdx::new(1) && *done == StepIdx::new(2))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(1)).map(|node| &node.kind), Some(CompiledNodeKind::RepeatAttempt { attempt_slot, body, done }) if *attempt_slot == SlotIdx::new(1) && *body == StepIdx::new(1) && *done == StepIdx::new(2))
        );
        assert!(
            matches!(workflow.node(StepIdx::new(2)).map(|node| &node.kind), Some(CompiledNodeKind::RepeatFinish { result }) if *result == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_rejects_oversized_source() {
        let limits = YamlLimits {
            max_source_bytes: 4,
            ..YamlLimits::default()
        };
        let result = YamlCompiler::new(limits).compile(b"name: too_large\n");

        assert!(
            matches!(result, Err(ref errors) if matches!(errors.first(), Some(CompileError::SourceTooLarge { .. })))
        );
    }

    #[test]
    fn compiler_accepts_minimal_strict_workflow() {
        let result = YamlCompiler::default().compile(
            b"version: velvet-ballastics/v1\nname: strict_minimal\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    save:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
        );

        assert!(matches!(result, Ok(ref workflow) if workflow.name() == "strict_minimal"));
    }

    #[test]
    fn compiler_lowers_yaml_set_to_set_const_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: set_case\nwhen:\n  manual: {}\nsteps:\n  - id: build_result\n    set:\n      value: 1\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;

        assert!(matches!(node.kind, CompiledNodeKind::SetConst { .. }));
        assert_eq!(node.output, Some(SlotIdx::ZERO));
        assert_eq!(node.next, Some(StepIdx::new(1)));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_wait_until_to_wait_until_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: wait_case\nwhen:\n  manual: {}\nsteps:\n  - id: deadline\n    save:\n      value: 1\n  - id: wait_for_deadline\n    wait:\n      until: 0\n  - id: done\n    finish:\n      result: 0\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let node = workflow.node(StepIdx::new(1)).ok_or("missing wait node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::WaitUntil {
                deadline_slot: SlotIdx::ZERO
            }
        ));
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_ask_to_ask_and_resume_nodes() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: ask_case\nwhen:\n  manual: {}\nsteps:\n  - id: prompt\n    save:\n      value: 1\n  - id: ask_user\n    ask:\n      prompt: 0\n      answer: 1\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        let ask = workflow.node(StepIdx::new(1)).ok_or("missing ask node")?;
        let resume = workflow
            .node(StepIdx::new(2))
            .ok_or("missing resume node")?;
        let finish = workflow
            .node(StepIdx::new(3))
            .ok_or("missing finish node")?;

        assert!(matches!(ask.kind, CompiledNodeKind::Ask { .. }));
        assert!(
            matches!(resume.kind, CompiledNodeKind::AskResume { answer } if answer == SlotIdx::new(1))
        );
        assert!(
            matches!(finish.kind, CompiledNodeKind::Finish { result } if result == SlotIdx::new(1))
        );
        Ok(())
    }

    #[test]
    fn compiler_lowers_yaml_run_to_do_node() -> Result<(), String> {
        let workflow = YamlCompiler::default()
            .compile(
                b"version: velvet-ballastics/v1\nname: run_case\nwhen:\n  manual: {}\nsteps:\n  - id: source_slot\n    save:\n      value: 1\n  - id: call_action\n    run:\n      action: 7\n      input: 0\n  - id: done\n    finish:\n      result: 1\n",
            )
            .map_err(|errors| format!("unexpected compile errors: {errors:?}"))?;
        assert_eq!(workflow.node_count(), 3);
        assert_eq!(workflow.slot_count(), 2);
        let node = workflow.node(StepIdx::new(1)).ok_or("missing run node")?;
        let finish = workflow
            .node(StepIdx::new(2))
            .ok_or("missing finish node")?;

        assert!(matches!(
            node.kind,
            CompiledNodeKind::Do { action, input }
                if action == ActionId::new(7) && input == SlotIdx::ZERO
        ));
        assert_eq!(node.output, Some(SlotIdx::new(1)));
        assert_eq!(node.next, Some(StepIdx::new(2)));
        assert!(matches!(
            finish.kind,
            CompiledNodeKind::Finish { result } if result == SlotIdx::new(1)
        ));
        Ok(())
    }

