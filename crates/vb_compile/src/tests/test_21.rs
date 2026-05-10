#![forbid(unsafe_code)]
// Edge-case tests for the compilation pipeline:
// - Empty workflow boundaries
// - Single-node workflows
// - Deeply nested ForEach/Together constructs
// - Resource contract validation boundaries
// - Idempotency gate edge cases
// - Emit/serialize round-trip edge cases

use super::helpers::*;

    // ── Single-node workflow edge cases ──────────────────────────────────────

    #[test]
    fn single_finish_step_with_boolean_result_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: single_bool
when:
  manual: {}
steps:
  - id: done
    finish:
      result: true
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.node_count() == 2, "single bool finish should be 2 nodes")?;
        // First node: SetConst(true), Second node: Finish(slot 0)
        let set_node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;
        match &set_node.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::Bool(true),
                    "constant should be Bool(true)",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        let finish_node = workflow.node(StepIdx::new(1)).ok_or("missing finish node")?;
        match &finish_node.kind {
            CompiledNodeKind::Finish { result } if result.get() == 0 => Ok(()),
            other => Err(format!("finish did not reference slot 0: {other:?}")),
        }
    }

    #[test]
    fn single_finish_step_with_null_result_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: single_null
when:
  manual: {}
steps:
  - id: done
    finish:
      result: null
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.node_count() == 2, "single null finish should be 2 nodes")?;
        let set_node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;
        match &set_node.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::Null,
                    "constant should be Null",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn single_finish_step_with_zero_integer_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: single_zero
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.node_count() == 2, "single zero finish should be 2 nodes")?;
        let set_node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;
        match &set_node.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(0),
                    "constant should be I64(0)",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn single_finish_step_with_negative_integer_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: single_neg
when:
  manual: {}
steps:
  - id: done
    finish:
      result: -1
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.node_count() == 2, "single neg finish should be 2 nodes")?;
        let set_node = workflow.node(StepIdx::new(0)).ok_or("missing set node")?;
        match &set_node.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(-1),
                    "constant should be I64(-1)",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        Ok(())
    }

    // ── Single finish with inputs/vars/secrets ──────────────────────────────

    #[test]
    fn single_finish_with_all_optional_fields_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: full_optional
when:
  manual: {}
inputs:
  user: text
vars:
  count: 0
secrets:
  api_key: API_KEY
result: {}
examples:
  - name: basic
    input:
      user: alice
steps:
  - id: done
    finish:
      result: true
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.name() == "full_optional", "name should match")?;
        Ok(())
    }

    #[test]
    fn single_finish_with_multiple_inputs_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: multi_input
when:
  manual: {}
inputs:
  user: text
  age: number
  active: boolean
  data: any
steps:
  - id: done
    finish:
      result: true
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.name() == "multi_input", "name should match")?;
        Ok(())
    }

    // ── Single save + finish edge cases ──────────────────────────────────────

    #[test]
    fn save_then_finish_with_slot_zero_reference_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: save_finish
when:
  manual: {}
steps:
  - id: build
    save:
      value: 42
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(workflow.node_count() == 2, "save + finish should be 2 nodes")?;
        let save = workflow.node(StepIdx::new(0)).ok_or("missing save node")?;
        match &save.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(42),
                    "constant should be I64(42)",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        let finish = workflow.node(StepIdx::new(1)).ok_or("missing finish node")?;
        match &finish.kind {
            CompiledNodeKind::Finish { result } if result.get() == 0 => Ok(()),
            other => Err(format!("finish should reference slot 0: {other:?}")),
        }
    }

    #[test]
    fn save_with_max_i64_value_compiles() -> Result<(), String> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: max_val\nwhen:\n  manual: {{}}\nsteps:\n  - id: build\n    save:\n      value: {}\n  - id: done\n    finish:\n      result: 0\n",
            i64::MAX
        );
        let workflow = adv_compile_ok(source.as_bytes())?;
        let save = workflow.node(StepIdx::new(0)).ok_or("missing save node")?;
        match &save.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(i64::MAX),
                    "constant should be I64(MAX)",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn save_with_negative_value_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: neg_save
when:
  manual: {}
steps:
  - id: build
    save:
      value: -100
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let save = workflow.node(StepIdx::new(0)).ok_or("missing save node")?;
        match &save.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(-100),
                    "constant should be I64(-100)",
                )?;
            }
            other => return Err(format!("expected SetConst, got {other:?}")),
        }
        Ok(())
    }

    // ── Deeply nested ForEach edge cases ────────────────────────────────────

    #[test]
    fn for_each_with_zero_limit_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: zero_limit
when:
  manual: {}
steps:
  - id: list
    save:
      value: 1
  - id: each
    for_each:
      input: 0
      item: 1
      limit: 0
  - id: done
    finish:
      result: 0
"#;
        // Zero limit should still compile -- the runtime will handle it
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    #[test]
    fn for_each_with_max_u32_limit_compiles() -> Result<(), String> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: max_limit\nwhen:\n  manual: {{}}\nsteps:\n  - id: list\n    save:\n      value: 1\n  - id: each\n    for_each:\n      input: 0\n      item: 1\n      limit: {}\n  - id: done\n    finish:\n      result: 0\n",
            u32::MAX
        );
        let workflow = adv_compile_ok(source.as_bytes())?;
        let start = workflow.node(StepIdx::new(1)).ok_or("missing for_each start")?;
        match &start.kind {
            CompiledNodeKind::ForEachStart { limit, .. } => {
                adv_ensure(*limit == u32::MAX, "limit should be u32::MAX")?;
            }
            other => return Err(format!("expected ForEachStart, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn for_each_with_at_once_zero_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: at_once_zero
when:
  manual: {}
steps:
  - id: list
    save:
      value: 1
  - id: each
    for_each:
      input: 0
      item: 1
      limit: 10
      at_once: 0
  - id: done
    finish:
      result: 0
"#;
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    // ── Deeply nested Together edge cases ───────────────────────────────────

    #[test]
    fn together_with_single_branch_compiles() -> Result<(), String> {
        // Together with a single branch is structurally valid but the validation
        // pipeline may reject it if the join is not after branch targets.
        // This test verifies the compile attempt is handled deterministically.
        let source = br#"version: velvet-ballastics/v1
name: single_branch
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches: [1]
  - id: done
    finish:
      result: 0
"#;
        // The result is deterministic -- either compile or validation error
        let result = YamlCompiler::default().compile(source);
        assert!(result.is_ok(), "compilation should succeed for valid YAML: {:?}", result);
        Ok(())
    }

    // ── Collect edge cases ──────────────────────────────────────────────────

    #[test]
    fn collect_with_zero_limit_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: collect_zero_limit
when:
  manual: {}
steps:
  - id: source
    save:
      value: 1
  - id: collect_values
    collect:
      source: 0
      limit: 0
      page_size: 1
  - id: done
    finish:
      result: 0
"#;
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    #[test]
    fn collect_with_zero_page_size_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: collect_zero_page
when:
  manual: {}
steps:
  - id: source
    save:
      value: 1
  - id: collect_values
    collect:
      source: 0
      limit: 5
      page_size: 0
  - id: done
    finish:
      result: 0
"#;
        let _workflow = adv_compile_ok(source)?;
        Ok(())
    }

    #[test]
    fn collect_with_max_limit_compiles() -> Result<(), String> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: collect_max\nwhen:\n  manual: {{}}\nsteps:\n  - id: source\n    save:\n      value: 1\n  - id: collect_values\n    collect:\n      source: 0\n      limit: {}\n      page_size: 100\n  - id: done\n    finish:\n      result: 0\n",
            u32::MAX
        );
        let workflow = adv_compile_ok(source.as_bytes())?;
        let start = workflow.node(StepIdx::new(1)).ok_or("missing collect start")?;
        match &start.kind {
            CompiledNodeKind::CollectStart { limit, .. } => {
                adv_ensure(*limit == u32::MAX, "limit should be u32::MAX")?;
            }
            other => return Err(format!("expected CollectStart, got {other:?}")),
        }
        Ok(())
    }

    // ── Reduce edge cases ───────────────────────────────────────────────────

    #[test]
    fn reduce_with_zero_accumulator_index_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: reduce_zero_acc
when:
  manual: {}
steps:
  - id: source
    save:
      value: 1
  - id: reduce_values
    reduce:
      input: 0
      accumulator: 0
      initial: 0
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let start = workflow.node(StepIdx::new(1)).ok_or("missing reduce start")?;
        match &start.kind {
            CompiledNodeKind::ReduceStart { accumulator, .. } => {
                adv_ensure(accumulator.get() == 0, "accumulator should be slot 0")?;
            }
            other => return Err(format!("expected ReduceStart, got {other:?}")),
        }
        Ok(())
    }

    // ── Repeat edge cases ───────────────────────────────────────────────────

    #[test]
    fn repeat_with_one_attempt_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: repeat_one
when:
  manual: {}
steps:
  - id: poll
    repeat:
      max_attempts: 1
  - id: done
    finish:
      result: 1
"#;
        let workflow = adv_compile_ok(source)?;
        let start = workflow.node(StepIdx::ZERO).ok_or("missing repeat start")?;
        match &start.kind {
            CompiledNodeKind::RepeatStart { max_attempts, .. } => {
                adv_ensure(*max_attempts == 1, "max_attempts should be 1")?;
            }
            other => return Err(format!("expected RepeatStart, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn repeat_with_max_u16_attempts_compiles() -> Result<(), String> {
        let source = format!(
            "version: velvet-ballastics/v1\nname: repeat_max\nwhen:\n  manual: {{}}\nsteps:\n  - id: poll\n    repeat:\n      max_attempts: {}\n  - id: done\n    finish:\n      result: 1\n",
            u16::MAX
        );
        let workflow = adv_compile_ok(source.as_bytes())?;
        let start = workflow.node(StepIdx::ZERO).ok_or("missing repeat start")?;
        match &start.kind {
            CompiledNodeKind::RepeatStart { max_attempts, .. } => {
                adv_ensure(*max_attempts == u16::MAX, "max_attempts should be u16::MAX")?;
            }
            other => return Err(format!("expected RepeatStart, got {other:?}")),
        }
        Ok(())
    }

    // ── Wait edge cases ─────────────────────────────────────────────────────

    #[test]
    fn wait_until_with_zero_deadline_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: wait_zero
when:
  manual: {}
steps:
  - id: deadline
    save:
      value: 0
  - id: wait_for_deadline
    wait:
      until: 0
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let wait = workflow.node(StepIdx::new(1)).ok_or("missing wait node")?;
        match &wait.kind {
            CompiledNodeKind::WaitUntil { deadline_slot } => {
                adv_ensure(deadline_slot.get() == 0, "deadline slot should be 0")?;
            }
            other => return Err(format!("expected WaitUntil, got {other:?}")),
        }
        Ok(())
    }

    // ── Ask edge cases ──────────────────────────────────────────────────────

    #[test]
    fn ask_with_same_prompt_and_answer_slots_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: ask_same_slots
when:
  manual: {}
steps:
  - id: prompt
    save:
      value: 1
  - id: ask_user
    ask:
      prompt: 0
      answer: 0
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let ask = workflow.node(StepIdx::new(1)).ok_or("missing ask node")?;
        match &ask.kind {
            CompiledNodeKind::Ask { prompt, timeout_slot } => {
                adv_ensure(prompt.get() == 0, "prompt slot should be 0")?;
                adv_ensure(timeout_slot.is_none(), "timeout should be None")?;
            }
            other => return Err(format!("expected Ask, got {other:?}")),
        }
        Ok(())
    }

    // ── Choose edge cases ───────────────────────────────────────────────────

    #[test]
    fn choose_both_branches_same_target_compiles() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: choose_same
when:
  manual: {}
steps:
  - id: flag
    save:
      value: true
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let choose = workflow.node(StepIdx::new(1)).ok_or("missing choose node")?;
        match &choose.kind {
            CompiledNodeKind::ChooseSlot { branches, otherwise } => {
                adv_ensure(branches.len() == 2, "should have 2 branches")?;
                adv_ensure(otherwise.is_none(), "otherwise should be None")?;
            }
            other => return Err(format!("expected ChooseSlot, got {other:?}")),
        }
        Ok(())
    }

    // ── Resource contract boundary tests ────────────────────────────────────

    #[test]
    fn default_resource_contract_matches_compiled_workflow() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: contract_check
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        adv_ensure(
            workflow.resource_contract() == ResourceContract::DEFAULT,
            "default workflow should have DEFAULT resource contract",
        )
    }

    #[test]
    fn workflow_digest_is_nonzero_for_minimal_source() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: digest_test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let workflow = adv_compile_ok(source)?;
        let digest = workflow.digest();
        adv_ensure(
            digest != WorkflowDigest::from_bytes([0u8; 32]),
            "digest should not be all zeros",
        )
    }

    // ── Idempotency gate boundary tests ─────────────────────────────────────

    #[test]
    fn idempotency_empty_contracts_list_passes() -> Result<(), String> {
        let contracts: [vb_core::ActionContract; 0] = [];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("empty contracts should pass: {:?}", e.0))
    }

    #[test]
    fn idempotency_all_side_effect_none_passes() -> Result<(), String> {
        let contracts = [
            make_contract(1, vb_core::SideEffect::None, vb_core::RetrySafety::Safe, vb_core::Idempotency::DeterministicPure),
            make_contract(2, vb_core::SideEffect::None, vb_core::RetrySafety::Safe, vb_core::Idempotency::DeterministicPure),
            make_contract(3, vb_core::SideEffect::None, vb_core::RetrySafety::KeyRequired, vb_core::Idempotency::IdempotentExternal),
        ];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("all-None side effects should pass: {:?}", e.0))
    }

    #[test]
    fn idempotency_side_effect_sends_with_key_required_idempotent_passes() -> Result<(), String> {
        let contracts = [make_contract(
            10,
            vb_core::SideEffect::Sends,
            vb_core::RetrySafety::KeyRequired,
            vb_core::Idempotency::IdempotentExternal,
        )];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("Sends+KeyRequired+IdempotentExternal should pass: {:?}", e.0))
    }

    #[test]
    fn idempotency_side_effect_creates_unsafe_rejected() -> Result<(), String> {
        let contracts = [make_contract(
            20,
            vb_core::SideEffect::Creates,
            vb_core::RetrySafety::Unsafe,
            vb_core::Idempotency::IdempotentExternal,
        )];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("Creates+Unsafe should be rejected")),
            Err(errors) => {
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation { action, side_effect, .. } => {
                        adv_ensure(*action == ActionId::new(20), "action should be 20")?;
                        adv_ensure(*side_effect == vb_core::SideEffect::Creates, "side effect should be Creates")
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    #[test]
    fn idempotency_side_effect_destroys_at_least_once_rejected() -> Result<(), String> {
        let contracts = [make_contract(
            30,
            vb_core::SideEffect::Destroys,
            vb_core::RetrySafety::Safe,
            vb_core::Idempotency::AtLeastOnceExternal,
        )];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("Destroys+AtLeastOnceExternal should be rejected")),
            Err(errors) => {
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation { action, .. } => {
                        adv_ensure(*action == ActionId::new(30), "action should be 30")
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    #[test]
    fn idempotency_side_effect_writes_safe_idempotent_passes() -> Result<(), String> {
        let contracts = [make_contract(
            40,
            vb_core::SideEffect::Writes,
            vb_core::RetrySafety::Safe,
            vb_core::Idempotency::IdempotentExternal,
        )];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("Writes+Safe+IdempotentExternal should pass: {:?}", e.0))
    }

    #[test]
    fn idempotency_multiple_violations_accumulate() -> Result<(), String> {
        let contracts = [
            make_contract(1, vb_core::SideEffect::Writes, vb_core::RetrySafety::Unsafe, vb_core::Idempotency::IdempotentExternal),
            make_contract(2, vb_core::SideEffect::Sends, vb_core::RetrySafety::Unsafe, vb_core::Idempotency::IdempotentExternal),
            make_contract(3, vb_core::SideEffect::None, vb_core::RetrySafety::Safe, vb_core::Idempotency::DeterministicPure),
        ];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("expected violations for actions 1 and 2")),
            Err(errors) => {
                adv_ensure(errors.len() == 2, "should have exactly 2 violations")?;
                let actions: Vec<u16> = errors.iter().filter_map(|e| {
                    match e {
                        CompileError::IdempotencyViolation { action, .. } => Some(action.get()),
                        _ => None,
                    }
                }).collect();
                adv_ensure(actions.contains(&1), "should contain action 1")?;
                adv_ensure(actions.contains(&2), "should contain action 2")?;
                Ok(())
            }
        }
    }

    // ── Emit/serialize round-trip edge cases ─────────────────────────────────

    #[test]
    fn emit_compiled_artifact_round_trips_minimal_workflow() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: round_trip
when:
  manual: {}
steps:
  - id: done
    finish:
      result: true
"#;
        let workflow = adv_compile_ok(source)?;
        let artifact = super::emit_compiled_artifact(&workflow)
            .map_err(|e| format!("emit failed: {e}"))?;
        adv_ensure(!artifact.is_empty(), "artifact should not be empty")?;
        Ok(())
    }

    #[test]
    fn emit_compiled_artifact_round_trips_multi_step_workflow() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: multi_round
when:
  manual: {}
inputs:
  user: text
vars:
  count: 0
steps:
  - id: build
    save:
      value: 42
  - id: build2
    save:
      value: true
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let artifact = super::emit_compiled_artifact(&workflow)
            .map_err(|e| format!("emit failed: {e}"))?;
        adv_ensure(!artifact.is_empty(), "artifact should not be empty")?;
        Ok(())
    }

    // ── Lowering function edge cases ────────────────────────────────────────

    #[test]
    fn lower_for_each_records_slots_in_builder() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let nodes = super::lower_for_each(
            StepIdx::new(0),
            SlotIdx::new(0),
            SlotIdx::new(1),
            10,
            StepIdx::new(1),
            StepIdx::new(2),
            &mut builder,
        )
        .map_err(|e| e.to_string())?;
        adv_ensure(nodes.len() == 2, "for_each should produce 2 nodes")?;
        match &nodes.first().ok_or("missing start")?.kind {
            CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } => {
                adv_ensure(input.get() == 0, "input should be 0")?;
                adv_ensure(item_slot.get() == 1, "item_slot should be 1")?;
                adv_ensure(*limit == 10, "limit should be 10")?;
                adv_ensure(body.get() == 1, "body should be step 1")?;
                adv_ensure(done.get() == 2, "done should be step 2")?;
            }
            other => return Err(format!("expected ForEachStart, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn lower_together_with_single_branch_produces_two_nodes() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let nodes = super::lower_together(
            StepIdx::new(0),
            vec![StepIdx::new(1)],
            StepIdx::new(2),
            &mut builder,
        )
        .map_err(|e| e.to_string())?;
        adv_ensure(nodes.len() == 2, "together should produce 2 nodes (start + join)")?;
        match &nodes.first().ok_or("missing start")?.kind {
            CompiledNodeKind::TogetherStart { branches, join } => {
                adv_ensure(branches.len() == 1, "should have 1 branch")?;
                adv_ensure(join.get() == 2, "join should be step 2")?;
            }
            other => return Err(format!("expected TogetherStart, got {other:?}")),
        }
        match &nodes.get(1).ok_or("missing join")?.kind {
            CompiledNodeKind::TogetherJoin { branch_count, accumulator } => {
                adv_ensure(*branch_count == 1, "branch_count should be 1")?;
                adv_ensure(accumulator.get() > 0, "accumulator should be allocated")?;
            }
            other => return Err(format!("expected TogetherJoin, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn lower_collect_records_source_slot() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let nodes = super::lower_collect(
            StepIdx::new(0),
            SlotIdx::new(0),
            5,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            &mut builder,
        )
        .map_err(|e| e.to_string())?;
        adv_ensure(nodes.len() == 3, "collect should produce 3 nodes")?;
        match &nodes.first().ok_or("missing start")?.kind {
            CompiledNodeKind::CollectStart { source, limit, page_size, body, done } => {
                adv_ensure(source.get() == 0, "source should be slot 0")?;
                adv_ensure(*limit == 5, "limit should be 5")?;
                adv_ensure(*page_size == 2, "page_size should be 2")?;
                adv_ensure(body.get() == 1, "body should be step 1")?;
                adv_ensure(done.get() == 2, "done should be step 2")?;
            }
            other => return Err(format!("expected CollectStart, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn lower_reduce_records_input_and_accumulator_slots() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let nodes = super::lower_reduce(
            StepIdx::new(0),
            SlotIdx::new(0),
            SlotIdx::new(1),
            ConstIdx::new(0),
            StepIdx::new(1),
            StepIdx::new(2),
            &mut builder,
        )
        .map_err(|e| e.to_string())?;
        adv_ensure(nodes.len() == 3, "reduce should produce 3 nodes")?;
        match &nodes.first().ok_or("missing start")?.kind {
            CompiledNodeKind::ReduceStart { input, accumulator, initial, body, done } => {
                adv_ensure(input.get() == 0, "input should be slot 0")?;
                adv_ensure(accumulator.get() == 1, "accumulator should be slot 1")?;
                adv_ensure(initial.get() == 0, "initial should be const 0")?;
                adv_ensure(body.get() == 1, "body should be step 1")?;
                adv_ensure(done.get() == 2, "done should be step 2")?;
            }
            other => return Err(format!("expected ReduceStart, got {other:?}")),
        }
        Ok(())
    }

    #[test]
    fn lower_wait_until_records_deadline_slot() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let node = super::lower_wait(
            StepIdx::new(0),
            super::WaitKind::Until { deadline: SlotIdx::new(3) },
            &mut builder,
        );
        match &node.kind {
            CompiledNodeKind::WaitUntil { deadline_slot } => {
                adv_ensure(deadline_slot.get() == 3, "deadline should be slot 3")
            }
            other => Err(format!("expected WaitUntil, got {other:?}")),
        }
    }

    #[test]
    fn lower_wait_event_records_event_and_timeout_slots() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let node = super::lower_wait(
            StepIdx::new(0),
            super::WaitKind::Event { event: SlotIdx::new(1), timeout: Some(SlotIdx::new(2)) },
            &mut builder,
        );
        match &node.kind {
            CompiledNodeKind::WaitEvent { event, timeout_slot } => {
                adv_ensure(event.get() == 1, "event should be slot 1")?;
                adv_ensure(timeout_slot.is_some(), "timeout should be Some")?;
                adv_ensure(timeout_slot.map_or(false, |t| t.get() == 2), "timeout should be slot 2")
            }
            other => Err(format!("expected WaitEvent, got {other:?}")),
        }
    }

    #[test]
    fn lower_wait_event_without_timeout_compiles() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let node = super::lower_wait(
            StepIdx::new(0),
            super::WaitKind::Event { event: SlotIdx::new(1), timeout: None },
            &mut builder,
        );
        match &node.kind {
            CompiledNodeKind::WaitEvent { event, timeout_slot } => {
                adv_ensure(event.get() == 1, "event should be slot 1")?;
                adv_ensure(timeout_slot.is_none(), "timeout should be None")
            }
            other => Err(format!("expected WaitEvent, got {other:?}")),
        }
    }

    // ── Lower_together overflow edge case ───────────────────────────────────

    #[test]
    fn lower_together_rejects_too_many_branches() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        // u16::MAX + 1 branches should overflow
        let branches: Vec<StepIdx> = (0..=u16::MAX)
            .filter_map(|i| StepIdx::new_checked(i))
            .collect();
        // This vec has u16::MAX + 1 elements which exceeds u16
        let result = super::lower_together(
            StepIdx::new(0),
            branches,
            StepIdx::new(1),
            &mut builder,
        );
        match result {
            Err(CompileError::PrimitiveLoweringLimitExceeded { primitive, field, .. }) => {
                adv_ensure(primitive == "together", "primitive should be together")?;
                adv_ensure(field == "branches", "field should be branches")
            }
            other => Err(format!("expected PrimitiveLoweringLimitExceeded, got {other:?}")),
        }
    }

    // ── SlotCompiler edge cases ─────────────────────────────────────────────

    #[test]
    fn slot_compiler_expression_overflow_rejected() -> Result<(), String> {
        let mut sc = SlotCompiler::new();
        fill_slot_compiler_expressions(&mut sc)?;
        let empty_ops: Box<[vb_core::workflow::ExprOp]> = Box::from([]);
        let prog = ExprProgram::try_from_ops(empty_ops)
            .unwrap_or_else(|_| ExprProgram { ops: Box::from([]), max_stack: 0 });
        let result = sc.push_expression(prog);
        adv_ensure(result.is_err(), "expression table overflow should be rejected")
    }

    #[test]
    fn slot_compiler_accessor_overflow_rejected() -> Result<(), String> {
        let mut sc = SlotCompiler::new();
        fill_slot_compiler_accessors(&mut sc)?;
        let prog = vb_core::AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::from([]),
        };
        let result = sc.push_accessor(prog);
        adv_ensure(result.is_err(), "accessor table overflow should be rejected")
    }

    #[test]
    fn slot_compiler_slot_count_no_slots_recorded() -> Result<(), String> {
        let sc = SlotCompiler::new();
        let count = sc.slot_count().map_err(|e| e.to_string())?;
        adv_ensure(count == 0, "empty compiler should have 0 slot count")
    }

    #[test]
    fn slot_compiler_slot_count_tracks_max_slot() -> Result<(), String> {
        let mut sc = SlotCompiler::new();
        sc.record_slot(SlotIdx::new(0));
        sc.record_slot(SlotIdx::new(5));
        sc.record_slot(SlotIdx::new(3));
        let count = sc.slot_count().map_err(|e| e.to_string())?;
        adv_ensure(count == 6, "slot count should be max_slot + 1 = 6")
    }

    #[test]
    fn slot_compiler_build_parts_produces_valid_parts() -> Result<(), String> {
        let mut sc = SlotCompiler::new();
        let const_idx = sc.push_constant(ConstValue::I64(42))
            .map_err(|e| e.to_string())?;
        sc.record_slot(SlotIdx::new(0));
        sc.push_node(CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::SetConst { value: const_idx },
        });
        let parts = sc.build_parts("test", WorkflowDigest::from_bytes([0u8; 32]))
            .map_err(|e| e.to_string())?;
        adv_ensure(parts.slot_count == 1, "slot count should be 1")?;
        adv_ensure(parts.constants.len() == 1, "should have 1 constant")?;
        adv_ensure(parts.nodes.len() == 1, "should have 1 node")?;
        Ok(())
    }

    // ── Choose lowering edge case: empty branch table ────────────────────────

    #[test]
    fn lower_choose_rejects_empty_branches_and_no_otherwise() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let result = super::lower_choose(
            StepIdx::new(0),
            vec![],
            None,
            &mut builder,
        );
        match result {
            Err(CompileError::Workflow(WorkflowError::EmptyBranchTable)) => Ok(()),
            other => Err(format!("expected EmptyBranchTable, got {other:?}")),
        }
    }

    #[test]
    fn lower_choose_accepts_empty_branches_with_otherwise() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        let result = super::lower_choose(
            StepIdx::new(0),
            vec![],
            Some(StepIdx::new(1)),
            &mut builder,
        );
        match result {
            Ok(node) => {
                match &node.kind {
                    CompiledNodeKind::ChooseSlot { branches, otherwise } => {
                        adv_ensure(branches.is_empty(), "branches should be empty")?;
                        adv_ensure(otherwise.is_some(), "otherwise should be Some")?;
                    }
                    other => return Err(format!("expected ChooseSlot, got {other:?}")),
                }
                Ok(())
            }
            Err(e) => Err(format!("unexpected error: {e:?}")),
        }
    }

    // ── Compile error display edge cases ────────────────────────────────────

    #[test]
    fn compile_errors_display_shows_multiple_errors() -> Result<(), String> {
        let errors = CompileErrors(vec![
            CompileError::EmptySource,
            CompileError::FloatForbidden,
        ]);
        let display = errors.to_string();
        adv_ensure(display.contains("EmptySource") || display.contains("empty") || display.contains("non-empty"), "should mention empty source")?;
        adv_ensure(display.contains("floating-point") || display.contains("FloatForbidden"), "should mention floating-point")?;
        Ok(())
    }

    #[test]
    fn compile_errors_len_and_is_empty() -> Result<(), String> {
        let empty = CompileErrors(vec![]);
        adv_ensure(empty.is_empty(), "empty errors should be empty")?;
        adv_ensure(empty.len() == 0, "empty errors should have len 0")?;

        let nonempty = CompileErrors(vec![CompileError::EmptySource]);
        adv_ensure(!nonempty.is_empty(), "non-empty errors should not be empty")?;
        adv_ensure(nonempty.len() == 1, "should have len 1")?;
        Ok(())
    }

    // ── Compile step overflow edge cases ─────────────────────────────────────

    #[test]
    fn lower_repeat_rejects_step_overflow() -> Result<(), String> {
        let mut builder = SlotCompiler::new();
        // StepIdx::MAX + 1 should overflow when computing attempt_slot
        let result = super::lower_repeat(
            StepIdx::MAX,
            3,
            StepIdx::new(0),
            &mut builder,
        );
        match result {
            Err(CompileError::PrimitiveLoweringLimitExceeded { primitive, field, .. }) => {
                adv_ensure(primitive == "repeat", "primitive should be repeat")?;
                adv_ensure(field == "attempt_slot", "field should be attempt_slot")
            }
            Err(CompileError::SlotIndexOutOfRange { .. }) => Ok(()), // also acceptable
            other => Err(format!("expected step overflow error, got {other:?}")),
        }
    }

    // ── YAML limits boundary tests ──────────────────────────────────────────

    #[test]
    fn yaml_limits_zero_depth_rejects_nested_yaml() -> Result<(), String> {
        let limits = YamlLimits {
            max_depth: 0,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler::new(limits);
        // Even minimal YAML has some depth; this should reject it
        let source = b"version: velvet-ballastics/v1\nname: zero_depth\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let result = compiler.compile(source);
        adv_ensure(result.is_err(), "zero depth limit should reject any workflow")
    }

    #[test]
    fn yaml_limits_zero_scalar_bytes_rejects_any_scalar() -> Result<(), String> {
        let limits = YamlLimits {
            max_scalar_bytes: 0,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler::new(limits);
        let source = b"version: velvet-ballastics/v1\nname: tiny_scalar\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let result = compiler.compile(source);
        adv_ensure(result.is_err(), "zero scalar limit should reject any scalar")
    }

    #[test]
    fn yaml_limits_zero_mapping_entries_rejects_workflow() -> Result<(), String> {
        let limits = YamlLimits {
            max_mapping_entries: 0,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler::new(limits);
        let source = b"version: velvet-ballastics/v1\nname: no_mappings\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let result = compiler.compile(source);
        adv_ensure(result.is_err(), "zero mapping limit should reject any workflow")
    }

    #[test]
    fn yaml_limits_zero_sequence_len_rejects_steps() -> Result<(), String> {
        let limits = YamlLimits {
            max_sequence_len: 0,
            ..YamlLimits::default()
        };
        let compiler = YamlCompiler::new(limits);
        let source = b"version: velvet-ballastics/v1\nname: no_seq\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let result = compiler.compile(source);
        adv_ensure(result.is_err(), "zero sequence limit should reject steps")
    }

    // ── compile_workflow convenience function edge cases ────────────────────

    #[test]
    fn compile_workflow_convenience_compiles_minimal() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: convenience\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let workflow = super::compile_workflow(source)
            .map_err(|e| format!("compile_workflow failed: {e}"))?;
        adv_ensure(workflow.name() == "convenience", "name should match")
    }

    #[test]
    fn compile_workflow_with_contracts_rejects_orphan_contract() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: contract_orphan\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        // Provide a contract that has no matching Do node
        let contracts = [make_contract(
            99,
            vb_core::SideEffect::None,
            vb_core::RetrySafety::Safe,
            vb_core::Idempotency::DeterministicPure,
        )];
        let result = super::compile_workflow_with_contracts(source, &contracts);
        adv_ensure(result.is_err(), "orphan contract should be rejected")
    }

    // ── Validate IR edge cases ──────────────────────────────────────────────

    #[test]
    fn validate_ir_rejects_empty_parts() -> Result<(), String> {
        let parts = WorkflowParts {
            name: Box::from("empty"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: Box::from([]),
            expressions: Box::from([]),
            accessors: Box::from([]),
            constants: Box::from([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        let result = super::validate_ir(parts);
        adv_ensure(result.is_err(), "empty parts should be rejected by validation")
    }

    // ── Compute compiled digest determinism ──────────────────────────────────

    #[test]
    fn compute_compiled_digest_empty_and_nonempty_differ() -> Result<(), String> {
        let d1 = super::compute_compiled_digest(b"");
        let d2 = super::compute_compiled_digest(b"a");
        adv_ensure(d1 != d2, "empty and non-empty sources should produce different digests")
    }

    #[test]
    fn compute_compiled_digest_same_input_same_output() -> Result<(), String> {
        let d1 = super::compute_compiled_digest(b"test_data");
        let d2 = super::compute_compiled_digest(b"test_data");
        adv_ensure(d1 == d2, "same input should produce same digest")
    }

    // ── Compile to generated Rust edge cases ────────────────────────────────

    #[test]
    fn compile_to_generated_rust_accepts_minimal_workflow() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: codegen_min\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let workflow = adv_compile_ok(source)?;
        let generated = super::compile_to_generated_rust(&workflow)
            .map_err(|e| format!("generated rust failed: {e}"))?;
        adv_ensure(!generated.is_empty(), "generated source should not be empty")?;
        Ok(())
    }

    // ── Build accessor table and constant pool ──────────────────────────────

    #[test]
    fn build_accessor_table_returns_empty_for_simple_workflow() -> Result<(), String> {
        let source = b"version: velvet-ballastics/v1\nname: no_accessors\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
        let workflow = adv_compile_ok(source)?;
        let parts = workflow.to_parts();
        let table = super::build_accessor_table(&parts);
        adv_ensure(table.is_empty(), "simple workflow should have no accessors")
    }

    #[test]
    fn build_constant_pool_returns_constants_from_workflow() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: has_constants
when:
  manual: {}
steps:
  - id: build
    save:
      value: 42
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let parts = workflow.to_parts();
        let pool = super::build_constant_pool(&parts);
        adv_ensure(!pool.is_empty(), "workflow with constants should have non-empty pool")?;
        let has_42 = pool.iter().any(|c| *c == ConstValue::I64(42));
        adv_ensure(has_42, "pool should contain I64(42)")
    }

    #[test]
    fn build_slot_layout_returns_correct_count() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: slot_layout
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: build2
    save:
      value: 2
  - id: done
    finish:
      result: 0
"#;
        let workflow = adv_compile_ok(source)?;
        let parts = workflow.to_parts();
        let layout = super::build_slot_layout(&parts);
        adv_ensure(layout >= 1, "slot layout should have at least 1 slot")
    }
