use super::helpers::*;

    #[test]
    fn idempotency_side_effect_safe_retry_passes() -> Result<(), String> {
        let contracts = [make_contract(
            10,
            vb_core::SideEffect::Writes,
            vb_core::RetrySafety::Safe,
            vb_core::Idempotency::IdempotentExternal,
        )];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("expected Ok, got errors: {:?}", e.0))
    }

    #[test]
    fn idempotency_side_effect_unsafe_retry_rejected() -> Result<(), String> {
        let contracts = [make_contract(
            20,
            vb_core::SideEffect::Writes,
            vb_core::RetrySafety::Unsafe,
            vb_core::Idempotency::AtLeastOnceExternal,
        )];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("expected error for unsafe retry, got Ok")),
            Err(errors) => {
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation {
                        action,
                        side_effect,
                        ..
                    } => {
                        if *action != ActionId::new(20) {
                            return Err(String::from("wrong action id"));
                        }
                        if *side_effect != vb_core::SideEffect::Writes {
                            return Err(String::from("wrong side effect"));
                        }
                        Ok(())
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    #[test]
    fn idempotency_non_idempotent_side_effect_rejected() -> Result<(), String> {
        let contracts = [make_contract(
            30,
            vb_core::SideEffect::Sends,
            vb_core::RetrySafety::KeyRequired,
            vb_core::Idempotency::AtLeastOnceExternal,
        )];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from(
                "expected error for non-idempotent side effect, got Ok",
            )),
            Err(errors) => {
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation {
                        action,
                        side_effect,
                        reason,
                    } => {
                        if *action != ActionId::new(30) {
                            return Err(String::from("wrong action id"));
                        }
                        if *side_effect != vb_core::SideEffect::Sends {
                            return Err(String::from("wrong side effect"));
                        }
                        let reason_ref: &str = &reason;
                        if !reason_ref.contains("AtLeastOnceExternal") {
                            return Err(String::from("reason should mention AtLeastOnceExternal"));
                        }
                        Ok(())
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    #[test]
    fn idempotency_idempotent_side_effect_passes() -> Result<(), String> {
        let contracts = [
            make_contract(
                40,
                vb_core::SideEffect::Creates,
                vb_core::RetrySafety::KeyRequired,
                vb_core::Idempotency::IdempotentExternal,
            ),
            make_contract(
                41,
                vb_core::SideEffect::Destroys,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::IdempotentExternal,
            ),
        ];
        super::check_idempotency_gates(&contracts)
            .map_err(|e| format!("expected Ok, got errors: {:?}", e.0))
    }

    #[test]
    fn idempotency_mixed_actions_partial_rejection() -> Result<(), String> {
        let contracts = [
            make_contract(
                50,
                vb_core::SideEffect::None,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::DeterministicPure,
            ),
            make_contract(
                51,
                vb_core::SideEffect::Writes,
                vb_core::RetrySafety::Safe,
                vb_core::Idempotency::IdempotentExternal,
            ),
            make_contract(
                52,
                vb_core::SideEffect::Destroys,
                vb_core::RetrySafety::Unsafe,
                vb_core::Idempotency::AtLeastOnceExternal,
            ),
        ];
        let result = super::check_idempotency_gates(&contracts);
        match result {
            Ok(()) => Err(String::from("expected error for unsafe action, got Ok")),
            Err(errors) => {
                if errors.as_slice().len() != 1 {
                    return Err(format!(
                        "expected exactly 1 error, got {}",
                        errors.as_slice().len()
                    ));
                }
                let first = errors.first().ok_or("errors should not be empty")?;
                match first {
                    CompileError::IdempotencyViolation { action, .. } => {
                        if *action != ActionId::new(52) {
                            return Err(String::from("expected violation for action 52 only"));
                        }
                        Ok(())
                    }
                    other => Err(format!("expected IdempotencyViolation, got {other:?}")),
                }
            }
        }
    }

    // ── SECURITY: Gate 12 bypass prevention tests ──────────────────────

    /// SECURITY: compile_workflow_with_contracts must reject mismatched contracts.
    ///
    /// Attack vector: Before the fix, `compile_workflow_with_contracts` did NOT
    /// run gate 12 (action contract completeness). A caller could provide
    /// contracts that had no corresponding Do nodes, or a workflow with Do
    /// nodes that had no contracts, and both would be accepted.
    ///
    /// This test verifies that gate 12 is now run during
    /// compile_workflow_with_contracts by providing an orphan contract
    /// (one that has no matching Do node in the workflow).

