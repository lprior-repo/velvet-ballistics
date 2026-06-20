//! Repair-hint generation from incident analysis data.

/// Build repair hints based on the failure code, side effects, and failed step.
pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[super::model::SideEffect],
    failed_at_step: Option<u16>,
) -> Vec<String> {
    let mut hints: Vec<String> = Vec::new();

    match failure_code {
        "RunFailed" => {
            hints.push("investigate step output and engine logs for the failed step".to_string());
            if !side_effects.is_empty() {
                hints.push(
                    "review side effects that completed before failure for compensating actions"
                        .to_string(),
                );
            }
            if let Some(step) = failed_at_step {
                hints.push(format!(
                    "consider retry from step {step} using the retry command"
                ));
            }
        }
        "RunCancelled" => {
            hints.push("run was cancelled; check if cancellation was intentional".to_string());
            if !side_effects.is_empty() {
                hints.push("review completed side effects for partial cleanup needs".to_string());
            }
        }
        _ => {}
    }

    hints
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "test assertions use indices into fixed-size repair hint vectors"
)]
mod tests {
    use super::super::model::SideEffect;
    use super::super::model::SideEffectCertainty;
    use super::*;

    // ---- T-009: RunFailed repair hints (1 hint) ----
    #[test]
    fn t_009_run_failed_1_hint() {
        let hints = build_repair_hints("RunFailed", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0],
            "investigate step output and engine logs for the failed step"
        );
    }

    // ---- T-010: RunFailed repair hints (3 hints) ----
    #[test]
    fn t_010_run_failed_3_hints() {
        let side_effects = vec![SideEffect {
            step: 1,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("RunFailed", &side_effects, Some(3));
        assert_eq!(hints.len(), 3);
        assert_eq!(
            hints[0],
            "investigate step output and engine logs for the failed step"
        );
        assert_eq!(
            hints[1],
            "review side effects that completed before failure for compensating actions"
        );
        assert_eq!(
            hints[2],
            "consider retry from step 3 using the retry command"
        );
    }

    // ---- T-011: RunCancelled repair hints (1 hint) ----
    #[test]
    fn t_011_run_cancelled_1_hint() {
        let hints = build_repair_hints("RunCancelled", &[], None);
        assert_eq!(hints.len(), 1);
        assert_eq!(
            hints[0],
            "run was cancelled; check if cancellation was intentional"
        );
    }

    // ---- T-012: RunCancelled repair hints (2 hints) ----
    #[test]
    fn t_012_run_cancelled_2_hints() {
        let side_effects = vec![SideEffect {
            step: 2,
            action: 0,
            certainty: SideEffectCertainty::Confirmed,
        }];
        let hints = build_repair_hints("RunCancelled", &side_effects, None);
        assert_eq!(hints.len(), 2);
        assert_eq!(
            hints[0],
            "run was cancelled; check if cancellation was intentional"
        );
        assert_eq!(
            hints[1],
            "review completed side effects for partial cleanup needs"
        );
    }

    // ---- T-013: Unknown failure code (0 hints) ----
    #[test]
    fn t_013_unknown_failure_code() {
        let hints = build_repair_hints("UnknownError", &[], None);
        assert!(hints.is_empty());
    }
}
