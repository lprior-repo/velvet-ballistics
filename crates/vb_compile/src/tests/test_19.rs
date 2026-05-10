#![forbid(unsafe_code)]
use super::helpers::*;

    #[test]
    fn finish_large_integer_compiled_as_literal_not_slot() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: huge_slot
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 65536
"#;
        let workflow = adv_compile_ok(source)?;
        // 65536 is > step index 0, so it's a literal, not a slot.
        // Produces 2 nodes: SetConst(65536) + Finish(slot 0)
        adv_ensure(
            workflow.node_count() == 2,
            "large integer finish should produce 2 nodes",
        )?;
        // Check constant pool contains the literal
        let node = workflow.node(StepIdx::new(0)).ok_or("missing node 0")?;
        match &node.kind {
            CompiledNodeKind::SetConst { value } => {
                let const_val = workflow.constant(*value).ok_or("missing constant")?;
                adv_ensure(
                    *const_val == ConstValue::I64(65536),
                    "constant should be I64(65536)",
                )
            }
            other => Err(format!("expected SetConst, got {other:?}")),
        }
    }

    /// Attack vector: Var referencing an accessor path ($vars.x.field) rejected.
    #[test]
    fn var_accessor_path_in_finish_rejected_with_unsupported_accessor() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: var_accessor
when:
  manual: {}
vars:
  data: 1
steps:
  - id: done
    finish:
      result: $vars.data.field
"#;
        let error = adv_compile_error(source)?;
        adv_ensure(
            matches!(error, CompileError::UnsupportedAccessorReference { .. }),
            "var accessor path did not produce UnsupportedAccessorReference",
        )
    }

    /// Attack vector: Validate that compile and parse_ast produce the same first
    /// diagnostic for a complex workflow with multiple issues (schema + reference).
    #[test]
    fn compile_parse_ast_parity_for_schema_then_reference_errors() -> Result<(), String> {
        let source = br#"version: velvet-ballastics/v1
name: parity_test
when:
  manual: {}
inputs:
  bad_field:
    is: text
    unknown_field: true
steps:
  - id: done
    finish:
      result: $input.missing
"#;
        adv_ensure(
            compile_error_text(source) == parse_ast_error_text(source),
            "compile and parse_ast diverged on schema+reference error",
        )
    }

    /// Attack vector: SlotCompiler constant pool overflow produces exact error.
    #[test]
    fn slot_compiler_constant_pool_overflow_rejected() -> Result<(), String> {
        let mut sc = SlotCompiler::new();
        // Fill up to u16::MAX + 1 (65536) constants; the 65537th push should fail
        fill_slot_compiler_constants(&mut sc)?;
        // Now the pool has 65536 entries; the next push should fail
        let result = sc.push_constant(ConstValue::I64(0));
        adv_ensure(
            result.is_err(),
            "constant pool overflow (65536 existing + 1 new) should produce an error",
        )
    }

    // =========================================================================
    // Phase 65 tests -- idempotency verification gate
    // =========================================================================

    fn make_contract(
        id: u16,
        side_effect: vb_core::SideEffect,
        retry_safety: vb_core::RetrySafety,
        idempotency: vb_core::Idempotency,
    ) -> vb_core::ActionContract {
        vb_core::ActionContract {
            id: ActionId::new(id),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency,
            side_effect,
            retry_safety,
        }
    }

    #[test]
    fn idempotency_no_side_effects_passes() -> Result<(), String> {
        let contracts = [
            make_contract(
                1,
                vb_core::SideEffect::None,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::DeterministicPure,
            ),
            make_contract(
                2,
                vb_core::SideEffect::None,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::DeterministicPure,
            ),
        ];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("expected Ok, got errors: {:?}", e.0))
    }

