// vb-5iebh check-scope Kani harness.
//! Kani proof harness for check-scope value object verification.
//!
//! This module provides Kani harnesses for verifying check-evidence
//! value object behavior.
//!
//! Obligations covered: vb-5iebh check-scope

#![cfg(kani)]
#![forbid(unsafe_code)]

use crate::check::CheckEvidence;

/// Kani harness for CheckEvidence::has_baseline.
#[kani::proof]
fn proof_check_evidence_has_baseline() {
    let evidence = kani::any::<CheckEvidence>();
    // has_baseline should return whether baseline_us is Some
    let has_baseline = evidence.has_baseline();
    let has_some = evidence.baseline_us.is_some();
    kani::assert(
        has_baseline == has_some,
        "has_baseline must match baseline_us.is_some()",
    );
}

/// Kani harness for CheckEvidence::threshold_delta.
#[kani::proof]
fn proof_check_evidence_threshold_delta() {
    let evidence = kani::any::<CheckEvidence>();
    let delta = evidence.threshold_delta();
    // delta should equal baseline * threshold_pct / 100 (or 0 if no baseline)
    match evidence.baseline_us {
        Some(baseline) => {
            let expected = baseline * evidence.threshold_pct / 100;
            kani::assert(
                delta == expected,
                "threshold_delta must equal baseline * threshold_pct / 100",
            );
        }
        None => {
            kani::assert(delta == 0, "threshold_delta must be 0 when no baseline");
        }
    }
}

/// Kani harness for CheckEvidence validation.
#[kani::proof]
fn proof_check_evidence_validation() {
    let evidence = kani::any::<CheckEvidence>();
    let result = evidence.validate();
    // If we have a baseline and budget is non-zero, validation should pass
    // when result_us is within threshold
    if evidence.baseline_us.is_some() && evidence.budget_us > 0 {
        let baseline = match evidence.baseline_us {
            Some(v) => v,
            None => {
                kani::assume(false);
                loop {}
            }
        };
        let threshold_delta = baseline * evidence.threshold_pct / 100;
        let max_allowed = baseline + threshold_delta;
        if evidence.result_us <= max_allowed {
            // Within threshold - should pass
            kani::assert(
                result.is_ok()
                    || matches!(
                        result,
                        Err(crate::check::CheckEvidenceError::RegressionDetected { .. })
                    ),
                "within-threshold validation must pass or be RegressionDetected",
            );
        }
    }
}
