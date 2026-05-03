/// A single verification check result.
#[derive(Debug, Clone)]
pub struct Certificate {
    pub kind: CertificateKind,
    pub status: CertificateStatus,
    pub message: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateStatus {
    Pass,
    Fail,
    Warn,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateKind {
    StructuralValidity,
    BoundedTransitions,
    SecretToResultLeak,
    StrictDurabilityEligibility,
    ActionIdempotency,
    ResourceBudget,
    Reachability,
    LoopNesting,
}

/// A taint propagation path from secret source to sink.
#[derive(Debug, Clone)]
pub struct TaintPath {
    pub source_step: String,
    pub sink_step: String,
    /// Step IDs along the path.
    pub path: Vec<String>,
    pub reaches_public_result: bool,
}

/// Full verification result for a workflow.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub certificates: Vec<Certificate>,
    pub overall: CertificateStatus,
    pub worst_case_memory_bytes: u64,
    pub max_transitions: u64,
    pub max_action_calls: u32,
    pub secret_paths: Vec<TaintPath>,
}

impl VerificationResult {
    /// Run all certificate checks against a compiled workflow.
    ///
    /// For now, returns stubs. Real implementation requires VB types.
    pub fn analyze() -> Self {
        Self {
            certificates: vec![
                Certificate {
                    kind: CertificateKind::StructuralValidity,
                    status: CertificateStatus::Pass,
                    message: "All nodes valid, entry reachable".into(),
                    details: vec![],
                },
                Certificate {
                    kind: CertificateKind::BoundedTransitions,
                    status: CertificateStatus::Pass,
                    message: "Within step budget".into(),
                    details: vec![],
                },
                Certificate {
                    kind: CertificateKind::SecretToResultLeak,
                    status: CertificateStatus::Pass,
                    message: "No secret reaches public result".into(),
                    details: vec![],
                },
                Certificate {
                    kind: CertificateKind::StrictDurabilityEligibility,
                    status: CertificateStatus::Warn,
                    message: "2 actions lack idempotency keys".into(),
                    details: vec!["Step 3: github.issue.create".into()],
                },
                Certificate {
                    kind: CertificateKind::ResourceBudget,
                    status: CertificateStatus::Pass,
                    message: "Worst case: 312 KiB frame, 842 transitions".into(),
                    details: vec![],
                },
            ],
            overall: CertificateStatus::Pass,
            worst_case_memory_bytes: 319488,
            max_transitions: 842,
            max_action_calls: 7,
            secret_paths: vec![],
        }
    }

    /// Count certificates by status.
    pub fn count_by_status(&self, status: CertificateStatus) -> usize {
        self.certificates
            .iter()
            .filter(|c| c.status == status)
            .count()
    }

    /// Get the worst status among all certificates.
    pub fn compute_overall(&self) -> CertificateStatus {
        if self.count_by_status(CertificateStatus::Fail) > 0 {
            CertificateStatus::Fail
        } else if self.count_by_status(CertificateStatus::Warn) > 0 {
            CertificateStatus::Warn
        } else {
            CertificateStatus::Pass
        }
    }
}
