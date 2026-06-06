// vb-5iebh.1: Check-evidence value object for check-scope obligations.
#![forbid(unsafe_code)]
//! Check-evidence value object for check-scope obligations.

use core::fmt;

/// Check-evidence value object for check-scope obligations.
///
/// Captures the inputs required to validate a performance regression
/// check against a baseline threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckEvidence {
    /// Kind of check being performed (e.g., "latency", "throughput").
    pub check_kind: &'static str,
    /// Allowable regression threshold as percentage.
    pub threshold_pct: u64,
    /// Baseline execution time in microseconds.
    pub baseline_us: Option<u64>,
    /// Actual result execution time in microseconds.
    pub result_us: u64,
    /// Performance budget in microseconds.
    pub budget_us: u64,
}

impl CheckEvidence {
    /// Returns true if a baseline is present.
    #[must_use]
    pub fn has_baseline(&self) -> bool {
        self.baseline_us.is_some()
    }

    /// Returns the threshold delta in microseconds.
    /// Returns 0 if baseline is not available.
    #[must_use]
    pub fn threshold_delta(&self) -> u64 {
        let baseline = self.baseline_us.unwrap_or(0);
        baseline.saturating_mul(self.threshold_pct) / 100
    }
}

/// Errors from check-evidence validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckEvidenceError {
    /// Missing baseline measurement.
    MissingBaseline,
    /// Performance regression detected.
    RegressionDetected {
        /// Delta between result and baseline in microseconds.
        delta: u64,
    },
    /// Budget not configured (zero budget_us).
    EmptyBudget,
}

impl fmt::Display for CheckEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBaseline => write!(f, "missing baseline measurement"),
            Self::RegressionDetected { delta } => {
                write!(f, "performance regression detected: {} us delta", delta)
            }
            Self::EmptyBudget => write!(f, "budget not configured"),
        }
    }
}

impl CheckEvidence {
    /// Validates this evidence against the check scope obligations.
    /// Returns `Ok(())` if all required metadata is present and result
    /// is within the configured threshold.
    pub fn validate(&self) -> Result<(), CheckEvidenceError> {
        // Baseline check
        let baseline_us = match self.baseline_us {
            Some(b) => b,
            None => return Err(CheckEvidenceError::MissingBaseline),
        };

        // Budget check
        if self.budget_us == 0 {
            return Err(CheckEvidenceError::EmptyBudget);
        }

        // Regression check
        let threshold_delta = baseline_us.saturating_mul(self.threshold_pct) / 100;
        if self.result_us > baseline_us.saturating_add(threshold_delta) {
            let delta = self.result_us.saturating_sub(baseline_us);
            return Err(CheckEvidenceError::RegressionDetected { delta });
        }

        Ok(())
    }
}

#[cfg(kani)]
impl kani::Arbitrary for CheckEvidence {
    fn any() -> Self {
        CheckEvidence {
            check_kind: "arbitrary_check_kind",
            threshold_pct: kani::any(),
            baseline_us: kani::any(),
            result_us: kani::any(),
            budget_us: kani::any(),
        }
    }
}
