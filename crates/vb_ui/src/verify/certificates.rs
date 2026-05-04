//! Certificate-based verification analysis for compiled workflows.
//!
//! Provides two verification APIs:
//! - **Certificate-based analysis** (`VerificationResult::analyze`): eight
//!   structural and semantic certificates for the verification screen.
//! - **Pre-flight checks** (`verify_workflow`): eight focused PASS/FAIL
//!   checks that validate compiled workflow parts before run admission.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

// ---------------------------------------------------------------------------
// Certificate-based verification types
// ---------------------------------------------------------------------------

/// Outcome of a single certificate check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateStatus {
    /// Check passed.
    Pass,
    /// Check failed with a reason.
    Fail(String),
    /// Check passed with a warning.
    Warn(String),
}

/// A single verification certificate.
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Which certificate kind was checked.
    pub kind: CertificateKind,
    /// Outcome of the check.
    pub status: CertificateStatus,
    /// Human-readable summary.
    pub details: String,
}

/// Kinds of verification certificates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateKind {
    /// Nodes non-empty, entry in bounds, node IDs match positions.
    StructuralValidity,
    /// max_steps and max_slots within acceptable bounds.
    Boundedness,
    /// slot_count <= max_slots, expressions/accessors within limits.
    ResourceBounds,
    /// Taint propagation analysis.
    TaintFlow,
    /// Action policy: Do nodes have action IDs, retry policies.
    ActionPolicy,
    /// Finish node exists, error handlers present.
    StrictDurability,
    /// All nodes reachable from entry.
    Reachability,
    /// Loop nesting is well-formed.
    LoopNesting,
}

/// Full verification result for a workflow.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// All certificate check results.
    pub certificates: Vec<Certificate>,
    /// Total number of checks performed.
    pub total_checks: usize,
    /// Number of passes.
    pub pass_count: usize,
    /// Number of failures.
    pub fail_count: usize,
    /// Number of warnings.
    pub warn_count: usize,
}

impl VerificationResult {
    /// Run all 8 certificate checks against a compiled workflow.
    pub fn analyze(parts: &WorkflowParts) -> Self {
        let certificates = vec![
            check_structural_validity(parts),
            check_boundedness(parts),
            check_resource_bounds(parts),
            check_taint_flow(parts),
            check_action_policy(parts),
            check_strict_durability(parts),
            check_reachability(parts),
            check_loop_nesting(parts),
        ];

        let total_checks = certificates.len();
        let pass_count = certificates
            .iter()
            .filter(|c| matches!(c.status, CertificateStatus::Pass))
            .count();
        let fail_count = certificates
            .iter()
            .filter(|c| matches!(c.status, CertificateStatus::Fail(_)))
            .count();
        let warn_count = certificates
            .iter()
            .filter(|c| matches!(c.status, CertificateStatus::Warn(_)))
            .count();

        Self {
            certificates,
            total_checks,
            pass_count,
            fail_count,
            warn_count,
        }
    }
}

// ---------------------------------------------------------------------------
// Pre-flight verification check types
// ---------------------------------------------------------------------------

/// Status of a single pre-flight verification check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed.
    Pass,
    /// Check failed.
    Fail,
    /// Check passed with a non-critical concern.
    Warn,
}

impl CheckStatus {
    /// Returns the worst of two statuses (Fail > Warn > Pass).
    fn merge_worst(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fail, _) | (_, Self::Fail) => Self::Fail,
            (Self::Warn, _) | (_, Self::Warn) => Self::Warn,
            (Self::Pass, Self::Pass) => Self::Pass,
        }
    }
}

/// One pre-flight verification check result.
#[derive(Debug, Clone)]
pub struct CertificateCheck {
    /// Human-readable name of the check.
    pub name: &'static str,
    /// Pass/fail/warn status.
    pub status: CheckStatus,
    /// Human-readable detail explaining the outcome.
    pub detail: String,
}

/// Aggregate report of all pre-flight verification checks.
#[derive(Debug, Clone)]
pub struct VerificationReport {
    /// Individual check results, one per pre-flight check.
    pub checks: Vec<CertificateCheck>,
    /// True when every check is Pass or Warn (no Fail).
    pub all_pass: bool,
    /// The worst status across all checks.
    pub worst_risk: CheckStatus,
}

/// Runs all 8 pre-flight verification checks against a compiled workflow.
///
/// The checks are:
/// 1. Structural validity (node array non-empty, entry in bounds, IDs match)
/// 2. Bounded transitions (resource contract bounds are non-zero and cover nodes)
/// 3. Secret-to-result leak (taint overlay analysis)
/// 4. Strict durability eligibility (action policy + journal mode)
/// 5. External action idempotency (action contract review)
/// 6. Worst-case memory budget (slot_count * max_frame_size)
/// 7. Max transitions (step count from IR)
/// 8. Max action calls (count of Do nodes)
#[must_use]
pub fn verify_workflow(parts: &WorkflowParts) -> VerificationReport {
    let checks = vec![
        check_preflight_structural_validity(parts),
        check_preflight_bounded_transitions(parts),
        check_preflight_secret_to_result_leak(parts),
        check_preflight_strict_durability_eligibility(parts),
        check_preflight_action_idempotency(parts),
        check_preflight_worst_case_memory_budget(parts),
        check_preflight_max_transitions(parts),
        check_preflight_max_action_calls(parts),
    ];

    let has_failure = checks.iter().any(|c| c.status == CheckStatus::Fail);
    let worst_risk = checks
        .iter()
        .map(|c| c.status)
        .fold(CheckStatus::Pass, CheckStatus::merge_worst);

    VerificationReport {
        checks,
        all_pass: !has_failure,
        worst_risk,
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 1: Structural validity
// ---------------------------------------------------------------------------

fn check_preflight_structural_validity(parts: &WorkflowParts) -> CertificateCheck {
    if parts.nodes.is_empty() {
        return CertificateCheck {
            name: "structural_validity",
            status: CheckStatus::Fail,
            detail: String::from("node array is empty"),
        };
    }

    let node_count = parts.nodes.len();
    if parts.entry.as_usize() >= node_count {
        return CertificateCheck {
            name: "structural_validity",
            status: CheckStatus::Fail,
            detail: format!(
                "entry step {} exceeds node count {}",
                parts.entry.get(),
                node_count,
            ),
        };
    }

    for (index, node) in parts.nodes.iter().enumerate() {
        if node.id.as_usize() != index {
            return CertificateCheck {
                name: "structural_validity",
                status: CheckStatus::Fail,
                detail: format!(
                    "node at position {} has id {} (mismatch)",
                    index,
                    node.id.get(),
                ),
            };
        }
    }

    CertificateCheck {
        name: "structural_validity",
        status: CheckStatus::Pass,
        detail: format!(
            "all {} nodes valid, entry {} in bounds",
            node_count,
            parts.entry.get(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 2: Bounded transitions
// ---------------------------------------------------------------------------

fn check_preflight_bounded_transitions(parts: &WorkflowParts) -> CertificateCheck {
    let contract = parts.resource_contract;

    if contract.max_steps == 0 {
        return CertificateCheck {
            name: "bounded_transitions",
            status: CheckStatus::Fail,
            detail: String::from("max_steps is zero in resource contract"),
        };
    }

    if contract.max_slots == 0 {
        return CertificateCheck {
            name: "bounded_transitions",
            status: CheckStatus::Fail,
            detail: String::from("max_slots is zero in resource contract"),
        };
    }

    if contract.max_step_budget_per_tick == 0 {
        return CertificateCheck {
            name: "bounded_transitions",
            status: CheckStatus::Fail,
            detail: String::from("max_step_budget_per_tick is zero"),
        };
    }

    let node_count = u16::try_from(parts.nodes.len()).unwrap_or(u16::MAX);
    if node_count > contract.max_steps {
        return CertificateCheck {
            name: "bounded_transitions",
            status: CheckStatus::Fail,
            detail: format!(
                "node count ({}) exceeds max_steps ({})",
                parts.nodes.len(),
                contract.max_steps,
            ),
        };
    }

    CertificateCheck {
        name: "bounded_transitions",
        status: CheckStatus::Pass,
        detail: format!(
            "max_steps={}, max_slots={}, budget_per_tick={}",
            contract.max_steps, contract.max_slots, contract.max_step_budget_per_tick,
        ),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 3: Secret-to-result leak
// ---------------------------------------------------------------------------

fn check_preflight_secret_to_result_leak(parts: &WorkflowParts) -> CertificateCheck {
    let empty_taint = std::collections::HashMap::new();
    let overlay = super::taint_overlay::compute_taint_overlay(parts, &empty_taint);

    if overlay.sources.is_empty() {
        return CertificateCheck {
            name: "secret_to_result_leak",
            status: CheckStatus::Pass,
            detail: String::from("no secret source nodes found in workflow"),
        };
    }

    if !overlay.finish_safe {
        let source_labels: Vec<String> = overlay
            .sources
            .iter()
            .map(|s| format!("step {}", s.get()))
            .collect();
        return CertificateCheck {
            name: "secret_to_result_leak",
            status: CheckStatus::Fail,
            detail: format!(
                "secret value from {} reaches Finish node",
                source_labels.join(", "),
            ),
        };
    }

    // Sources exist but are contained -- warning.
    let warning_count = overlay
        .paths
        .iter()
        .filter(|seg| seg.status == super::taint_overlay::TaintPathStatus::Warning)
        .count();

    CertificateCheck {
        name: "secret_to_result_leak",
        status: CheckStatus::Warn,
        detail: format!(
            "{} secret source(s) present but contained ({} warning propagation path(s))",
            overlay.sources.len(),
            warning_count,
        ),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 4: Strict durability eligibility
// ---------------------------------------------------------------------------

fn check_preflight_strict_durability_eligibility(parts: &WorkflowParts) -> CertificateCheck {
    let mut has_finish = false;
    let mut do_with_error_handler: usize = 0;
    let mut do_total: usize = 0;
    let mut error_handler_count: usize = 0;
    let mut on_error_count: usize = 0;

    for node in parts.nodes.iter() {
        match node.kind {
            CompiledNodeKind::Finish { .. } => {
                has_finish = true;
            }
            CompiledNodeKind::Do { .. } => {
                do_total = do_total.saturating_add(1);
                if node.on_error.is_some() {
                    do_with_error_handler = do_with_error_handler.saturating_add(1);
                }
            }
            CompiledNodeKind::ErrorHandler { .. } => {
                error_handler_count = error_handler_count.saturating_add(1);
            }
            _ => {}
        }
        if node.on_error.is_some() {
            on_error_count = on_error_count.saturating_add(1);
        }
    }

    if !has_finish {
        return CertificateCheck {
            name: "strict_durability_eligibility",
            status: CheckStatus::Fail,
            detail: String::from("no Finish node found"),
        };
    }

    if do_total > 0 && do_with_error_handler == 0 && error_handler_count == 0 {
        return CertificateCheck {
            name: "strict_durability_eligibility",
            status: CheckStatus::Warn,
            detail: format!(
                "{} Do node(s) without error handlers or journal mode; replay safety not guaranteed",
                do_total,
            ),
        };
    }

    CertificateCheck {
        name: "strict_durability_eligibility",
        status: CheckStatus::Pass,
        detail: format!(
            "Finish present, {} of {} Do nodes have error handlers, {} error handler nodes, {} on_error directives",
            do_with_error_handler, do_total, error_handler_count, on_error_count,
        ),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 5: External action idempotency
// ---------------------------------------------------------------------------

fn check_preflight_action_idempotency(parts: &WorkflowParts) -> CertificateCheck {
    let mut do_count: usize = 0;
    let mut actions_with_retry: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut all_action_ids: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut retry_count: usize = 0;

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { action, .. } = node.kind {
            do_count = do_count.saturating_add(1);
            all_action_ids.insert(action.get());
            if node.on_error.is_some() {
                actions_with_retry.insert(action.get());
            }
        }
        if let CompiledNodeKind::RetryCheck { .. } = node.kind {
            retry_count = retry_count.saturating_add(1);
        }
    }

    if do_count == 0 {
        return CertificateCheck {
            name: "action_idempotency",
            status: CheckStatus::Pass,
            detail: String::from("no Do nodes; idempotency not applicable"),
        };
    }

    let unguarded = all_action_ids.len().saturating_sub(actions_with_retry.len());

    if unguarded > 0 && retry_count == 0 {
        return CertificateCheck {
            name: "action_idempotency",
            status: CheckStatus::Warn,
            detail: format!(
                "{} action(s) without retry/error handling and no RetryCheck nodes",
                unguarded,
            ),
        };
    }

    CertificateCheck {
        name: "action_idempotency",
        status: CheckStatus::Pass,
        detail: format!(
            "{} Do nodes across {} distinct action(s), {} with error handling, {} retry policy(ies)",
            do_count,
            all_action_ids.len(),
            actions_with_retry.len(),
            retry_count,
        ),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 6: Worst-case memory budget
// ---------------------------------------------------------------------------

/// Maximum frame size in bytes used for worst-case memory budget estimation.
/// Each slot holds at most one value; we conservatively estimate 64 bytes
/// per slot (enough for an inline number/boolean/small string).
const MAX_FRAME_SIZE: u64 = 64;

fn check_preflight_worst_case_memory_budget(parts: &WorkflowParts) -> CertificateCheck {
    let slot_count = u64::from(parts.slot_count);
    let worst_case_bytes = slot_count.saturating_mul(MAX_FRAME_SIZE);

    // Use the resource contract's max_output_bytes as a reference ceiling.
    // If worst_case_bytes exceeds it, that is a warn (not fail) because the
    // actual values may be smaller than the per-slot maximum.
    let output_limit = u64::from(parts.resource_contract.max_output_bytes);

    if worst_case_bytes == 0 {
        return CertificateCheck {
            name: "worst_case_memory_budget",
            status: CheckStatus::Pass,
            detail: String::from("no slots allocated; memory budget is zero"),
        };
    }

    if worst_case_bytes > output_limit && output_limit > 0 {
        return CertificateCheck {
            name: "worst_case_memory_budget",
            status: CheckStatus::Warn,
            detail: format!(
                "worst-case {} bytes ({} slots x {} B/slot) exceeds max_output_bytes {}",
                worst_case_bytes, parts.slot_count, MAX_FRAME_SIZE, output_limit,
            ),
        };
    }

    CertificateCheck {
        name: "worst_case_memory_budget",
        status: CheckStatus::Pass,
        detail: format!(
            "worst-case {} bytes ({} slots x {} B/slot)",
            worst_case_bytes, parts.slot_count, MAX_FRAME_SIZE,
        ),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 7: Max transitions
// ---------------------------------------------------------------------------

fn check_preflight_max_transitions(parts: &WorkflowParts) -> CertificateCheck {
    let step_count = parts.nodes.len();
    let contract_limit = usize::from(parts.resource_contract.max_steps);

    if contract_limit == 0 {
        return CertificateCheck {
            name: "max_transitions",
            status: CheckStatus::Fail,
            detail: String::from("max_steps is zero; no transitions allowed"),
        };
    }

    if step_count > contract_limit {
        return CertificateCheck {
            name: "max_transitions",
            status: CheckStatus::Fail,
            detail: format!(
                "IR has {} steps but max_steps is {}",
                step_count, contract_limit,
            ),
        };
    }

    CertificateCheck {
        name: "max_transitions",
        status: CheckStatus::Pass,
        detail: format!("IR step count {} within max_steps {}", step_count, contract_limit),
    }
}

// ---------------------------------------------------------------------------
// Pre-flight check 8: Max action calls
// ---------------------------------------------------------------------------

fn check_preflight_max_action_calls(parts: &WorkflowParts) -> CertificateCheck {
    let mut do_count: usize = 0;

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { .. } = node.kind {
            do_count = do_count.saturating_add(1);
        }
    }

    // Use max_retry_attempts as a soft ceiling: if Do count exceeds it,
    // the workflow may overwhelm the action dispatch pipeline.
    let retry_ceiling = usize::from(parts.resource_contract.max_retry_attempts);

    if do_count > retry_ceiling && retry_ceiling > 0 {
        return CertificateCheck {
            name: "max_action_calls",
            status: CheckStatus::Warn,
            detail: format!(
                "{} Do nodes exceeds max_retry_attempts ceiling of {}",
                do_count, retry_ceiling,
            ),
        };
    }

    CertificateCheck {
        name: "max_action_calls",
        status: CheckStatus::Pass,
        detail: format!(
            "{} Do node(s) within max_retry_attempts ceiling of {}",
            do_count, retry_ceiling,
        ),
    }
}

// ---------------------------------------------------------------------------
// Certificate 1: Structural Validity
// ---------------------------------------------------------------------------

fn check_structural_validity(parts: &WorkflowParts) -> Certificate {
    // Check nodes non-empty
    if parts.nodes.is_empty() {
        return Certificate {
            kind: CertificateKind::StructuralValidity,
            status: CertificateStatus::Fail("node array is empty".into()),
            details: "A workflow must contain at least one node.".into(),
        };
    }

    // Check entry in bounds
    let node_count = parts.nodes.len();
    if parts.entry.as_usize() >= node_count {
        return Certificate {
            kind: CertificateKind::StructuralValidity,
            status: CertificateStatus::Fail(format!(
                "entry step {} exceeds node count {}",
                parts.entry.get(),
                node_count,
            )),
            details: "Entry step must reference a valid node index.".into(),
        };
    }

    // Check node IDs match positions
    for (index, node) in parts.nodes.iter().enumerate() {
        if node.id.as_usize() != index {
            return Certificate {
                kind: CertificateKind::StructuralValidity,
                status: CertificateStatus::Fail(format!(
                    "node at position {} has id {}",
                    index,
                    node.id.get(),
                )),
                details: "Every node id must equal its position in the node array.".into(),
            };
        }
    }

    Certificate {
        kind: CertificateKind::StructuralValidity,
        status: CertificateStatus::Pass,
        details: format!(
            "All {} nodes valid, entry {} in bounds",
            node_count,
            parts.entry.get(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Certificate 2: Boundedness
// ---------------------------------------------------------------------------

fn check_boundedness(parts: &WorkflowParts) -> Certificate {
    let contract = parts.resource_contract;
    let mut issues: Vec<String> = Vec::new();

    // Check max_steps is non-zero and reasonable
    if contract.max_steps == 0 {
        issues.push("max_steps is zero".into());
    }

    // Check max_slots is non-zero
    if contract.max_slots == 0 {
        issues.push("max_slots is zero".into());
    }

    // Check max_step_budget_per_tick is non-zero
    if contract.max_step_budget_per_tick == 0 {
        issues.push("max_step_budget_per_tick is zero".into());
    }

    // Check node count does not exceed max_steps
    let node_count = u16::try_from(parts.nodes.len()).unwrap_or(u16::MAX);
    if node_count > contract.max_steps {
        issues.push(format!(
            "node count ({}) exceeds max_steps ({})",
            parts.nodes.len(),
            contract.max_steps,
        ));
    }

    if issues.is_empty() {
        Certificate {
            kind: CertificateKind::Boundedness,
            status: CertificateStatus::Pass,
            details: format!(
                "max_steps={}, max_slots={}, budget_per_tick={}",
                contract.max_steps, contract.max_slots, contract.max_step_budget_per_tick,
            ),
        }
    } else {
        Certificate {
            kind: CertificateKind::Boundedness,
            status: CertificateStatus::Fail(issues.join("; ")),
            details: "Resource contract boundedness checks failed.".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Certificate 3: Resource Bounds
// ---------------------------------------------------------------------------

fn check_resource_bounds(parts: &WorkflowParts) -> Certificate {
    let contract = parts.resource_contract;
    let mut issues: Vec<String> = Vec::new();

    // slot_count <= max_slots
    if u32::from(parts.slot_count) > u32::from(contract.max_slots) {
        issues.push(format!(
            "slot_count ({}) exceeds max_slots ({})",
            parts.slot_count, contract.max_slots,
        ));
    }

    // expressions within max_expressions
    if parts.expressions.len() > usize::from(contract.max_expressions) {
        issues.push(format!(
            "expressions ({}) exceeds max_expressions ({})",
            parts.expressions.len(),
            contract.max_expressions,
        ));
    }

    // accessors within max_accessors
    if parts.accessors.len() > usize::from(contract.max_accessors) {
        issues.push(format!(
            "accessors ({}) exceeds max_accessors ({})",
            parts.accessors.len(),
            contract.max_accessors,
        ));
    }

    // constants within max_constants
    if parts.constants.len() > usize::from(contract.max_constants) {
        issues.push(format!(
            "constants ({}) exceeds max_constants ({})",
            parts.constants.len(),
            contract.max_constants,
        ));
    }

    if issues.is_empty() {
        Certificate {
            kind: CertificateKind::ResourceBounds,
            status: CertificateStatus::Pass,
            details: format!(
                "slots={}, expressions={}, accessors={}, constants={}",
                parts.slot_count,
                parts.expressions.len(),
                parts.accessors.len(),
                parts.constants.len(),
            ),
        }
    } else {
        Certificate {
            kind: CertificateKind::ResourceBounds,
            status: CertificateStatus::Fail(issues.join("; ")),
            details: "Resource budget exceeded.".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Certificate 4: Taint Flow
// ---------------------------------------------------------------------------

fn check_taint_flow(parts: &WorkflowParts) -> Certificate {
    let empty_taint = std::collections::HashMap::new();
    let overlay = super::taint_overlay::compute_taint_overlay(parts, &empty_taint);

    // No secret sources at all -- clean pass.
    if overlay.sources.is_empty() {
        return Certificate {
            kind: CertificateKind::TaintFlow,
            status: CertificateStatus::Pass,
            details: "No secret source nodes (WaitEvent/Ask) found in workflow.".into(),
        };
    }

    // Collect source step indices for human-readable messages.
    let source_labels: Vec<String> = overlay
        .sources
        .iter()
        .map(|s| format!("step {}", s.get()))
        .collect();

    // Check for direct paths from secret source to Finish.
    let dangerous_paths: Vec<&super::taint_overlay::TaintPathSegment> = overlay
        .paths
        .iter()
        .filter(|seg| seg.status == super::taint_overlay::TaintPathStatus::Dangerous)
        .collect();

    if !overlay.finish_safe {
        // At least one secret source can reach a Finish node.
        // Determine whether the path is direct (source -> Finish) or indirect
        // (source -> ... -> Finish).
        let direct_to_finish = dangerous_paths
            .iter()
            .any(|seg| overlay.sinks.contains(&seg.to));

        // For direct vs indirect: check if any path goes through intermediate
        // nodes before reaching a sink.
        let sink_set: std::collections::HashSet<StepIdx> = overlay.sinks.iter().copied().collect();
        let has_indirect = dangerous_paths.iter().any(|seg| {
            // This segment reaches a sink but there are other segments from the
            // same source to non-sink nodes -- meaning it goes through
            // intermediaries.
            !sink_set.contains(&seg.to)
                && seg.status == super::taint_overlay::TaintPathStatus::Dangerous
        });

        if has_indirect {
            Certificate {
                kind: CertificateKind::TaintFlow,
                status: CertificateStatus::Fail(format!(
                    "secret value from {} flows to Finish node through intermediate nodes",
                    source_labels.join(", "),
                )),
                details: format!(
                    "Indirect taint propagation: {} source(s), {} dangerous path segment(s), {} sink(s)",
                    overlay.sources.len(),
                    dangerous_paths.len(),
                    overlay.sinks.len(),
                ),
            }
        } else if direct_to_finish {
            Certificate {
                kind: CertificateKind::TaintFlow,
                status: CertificateStatus::Fail(format!(
                    "secret value from {} flows directly to Finish node",
                    source_labels.join(", "),
                )),
                details: format!(
                    "Direct taint: {} source(s), {} sink(s)",
                    overlay.sources.len(),
                    overlay.sinks.len(),
                ),
            }
        } else {
            // Dangerous paths exist but none directly land on a sink via a
            // single segment -- still a failure because the overlay reports
            // finish_safe == false.
            Certificate {
                kind: CertificateKind::TaintFlow,
                status: CertificateStatus::Fail(format!(
                    "secret value from {} reaches Finish node",
                    source_labels.join(", "),
                )),
                details: format!(
                    "{} source(s), {} path segment(s), {} sink(s)",
                    overlay.sources.len(),
                    overlay.paths.len(),
                    overlay.sinks.len(),
                ),
            }
        }
    } else {
        // finish_safe is true: sources exist but none reach a Finish node.
        // This is a warning because secret nodes are present but contained.
        let warning_paths: Vec<&super::taint_overlay::TaintPathSegment> = overlay
            .paths
            .iter()
            .filter(|seg| seg.status == super::taint_overlay::TaintPathStatus::Warning)
            .collect();

        if warning_paths.is_empty() {
            // Sources exist but they have no outgoing edges at all.
            Certificate {
                kind: CertificateKind::TaintFlow,
                status: CertificateStatus::Warn(format!(
                    "secret source(s) at {} have no outgoing propagation paths",
                    source_labels.join(", "),
                )),
                details: format!(
                    "{} secret source node(s) present but isolated; no taint propagation detected",
                    overlay.sources.len(),
                ),
            }
        } else {
            // Sources propagate to non-Finish nodes -- uncertain containment.
            Certificate {
                kind: CertificateKind::TaintFlow,
                status: CertificateStatus::Warn(format!(
                    "secret value from {} propagates to {} non-Finish node(s) but does not reach Finish",
                    source_labels.join(", "),
                    warning_paths.len(),
                )),
                details: format!(
                    "Indirect/uncertain propagation: {} source(s), {} warning segment(s), {} sink(s)",
                    overlay.sources.len(),
                    warning_paths.len(),
                    overlay.sinks.len(),
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Certificate 5: Action Policy
// ---------------------------------------------------------------------------

fn check_action_policy(parts: &WorkflowParts) -> Certificate {
    let mut do_count: usize = 0;
    let mut missing_actions: Vec<String> = Vec::new();
    let mut retry_count: usize = 0;
    let mut error_handler_count: usize = 0;

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Do { action, .. } = node.kind {
            do_count = do_count.saturating_add(1);
            // Every Do node has an action field by construction; we verify it
            // is non-zero as a sanity check (action 0 could be valid in some
            // systems but we flag it as worth reviewing).
            if action.get() == 0 {
                missing_actions.push(format!("step {} has action_id 0", node.id.get()));
            }
        }

        if let CompiledNodeKind::RetryCheck { .. } = node.kind {
            retry_count = retry_count.saturating_add(1);
        }

        if let CompiledNodeKind::ErrorHandler { .. } = node.kind {
            error_handler_count = error_handler_count.saturating_add(1);
        }

        if let CompiledNodeKind::RepeatStart { .. } = node.kind {
            // Repeat is a form of retry policy
            retry_count = retry_count.saturating_add(1);
        }
    }

    let mut warnings: Vec<String> = Vec::new();

    if do_count > 0 && retry_count == 0 && error_handler_count == 0 {
        warnings.push(format!(
            "{} Do nodes found but no retry policies or error handlers",
            do_count,
        ));
    }

    if missing_actions.is_empty() && warnings.is_empty() {
        Certificate {
            kind: CertificateKind::ActionPolicy,
            status: CertificateStatus::Pass,
            details: format!(
                "{} actions, {} retry policies, {} error handlers",
                do_count, retry_count, error_handler_count,
            ),
        }
    } else {
        let all_issues: Vec<String> = missing_actions.into_iter().chain(warnings).collect();
        Certificate {
            kind: CertificateKind::ActionPolicy,
            status: CertificateStatus::Warn(all_issues.join("; ")),
            details: "Action policy review completed with warnings.".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Certificate 6: Strict Durability
// ---------------------------------------------------------------------------

fn check_strict_durability(parts: &WorkflowParts) -> Certificate {
    let mut has_finish = false;
    let mut finish_count: usize = 0;
    let mut error_handler_count: usize = 0;
    let mut on_error_count: usize = 0;

    for node in parts.nodes.iter() {
        if let CompiledNodeKind::Finish { .. } = node.kind {
            has_finish = true;
            finish_count = finish_count.saturating_add(1);
        }
        if let CompiledNodeKind::ErrorHandler { .. } = node.kind {
            error_handler_count = error_handler_count.saturating_add(1);
        }
        if node.on_error.is_some() {
            on_error_count = on_error_count.saturating_add(1);
        }
    }

    if !has_finish {
        return Certificate {
            kind: CertificateKind::StrictDurability,
            status: CertificateStatus::Fail("no Finish node found".into()),
            details: "Workflow must have at least one Finish node to produce a result.".into(),
        };
    }

    let mut warnings: Vec<String> = Vec::new();

    if finish_count > 1 {
        warnings.push(format!("{} Finish nodes found (expected 1)", finish_count));
    }

    if error_handler_count == 0 && on_error_count == 0 {
        warnings.push("no error handlers or on_error directives found".into());
    }

    if warnings.is_empty() {
        Certificate {
            kind: CertificateKind::StrictDurability,
            status: CertificateStatus::Pass,
            details: format!(
                "Finish node present, {} error handlers, {} on_error directives",
                error_handler_count, on_error_count,
            ),
        }
    } else {
        Certificate {
            kind: CertificateKind::StrictDurability,
            status: CertificateStatus::Warn(warnings.join("; ")),
            details: "Strict durability check passed with warnings.".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Certificate 7: Reachability
// ---------------------------------------------------------------------------

fn check_reachability(parts: &WorkflowParts) -> Certificate {
    if parts.nodes.is_empty() {
        return Certificate {
            kind: CertificateKind::Reachability,
            status: CertificateStatus::Fail("no nodes to analyze".into()),
            details: "Empty workflow has no reachable nodes.".into(),
        };
    }

    let node_count = parts.nodes.len();
    let mut visited = vec![false; node_count];

    // BFS from entry
    let mut queue = vec![parts.entry.as_usize()];
    if parts.entry.as_usize() < node_count
        && let Some(slot) = visited.get_mut(parts.entry.as_usize())
    {
        *slot = true;
    }

    while let Some(idx) = queue.pop() {
        let node = match parts.nodes.get(idx) {
            Some(n) => n,
            None => continue,
        };

        // Collect all successor step indices from this node
        let successors = collect_successors(&node.kind, node.next, node.on_error);

        for succ in successors {
            let succ_usize = succ.as_usize();
            if succ_usize < node_count {
                let is_visited = visited.get(succ_usize).copied().unwrap_or(true);
                if !is_visited {
                    if let Some(slot) = visited.get_mut(succ_usize) {
                        *slot = true;
                    }
                    queue.push(succ_usize);
                }
            }
        }
    }

    let unreachable: Vec<String> = visited
        .iter()
        .enumerate()
        .filter(|(_, reached)| !*reached)
        .map(|(idx, _)| format!("step {}", idx))
        .collect();

    if unreachable.is_empty() {
        Certificate {
            kind: CertificateKind::Reachability,
            status: CertificateStatus::Pass,
            details: format!("All {} nodes reachable from entry", node_count),
        }
    } else {
        Certificate {
            kind: CertificateKind::Reachability,
            status: CertificateStatus::Fail(format!(
                "{} unreachable node(s): {}",
                unreachable.len(),
                unreachable.join(", "),
            )),
            details: "Every node must be reachable from the entry step.".into(),
        }
    }
}

/// Collect all successor step indices from a node kind.
fn collect_successors(
    kind: &CompiledNodeKind,
    next: Option<StepIdx>,
    on_error: Option<StepIdx>,
) -> Vec<StepIdx> {
    let mut succs: Vec<StepIdx> = Vec::new();

    // Linear fallthrough
    if let Some(n) = next {
        succs.push(n);
    }
    // Error handler
    if let Some(h) = on_error {
        succs.push(h);
    }

    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::Finish { .. } => {}

        CompiledNodeKind::Jump { target } => {
            succs.push(*target);
        }

        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.iter() {
                succs.push(branch.target);
            }
            if let Some(target) = otherwise {
                succs.push(*target);
            }
        }

        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.iter() {
                succs.push(branch.target);
            }
            if let Some(target) = otherwise {
                succs.push(*target);
            }
        }

        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            succs.push(*body);
            succs.push(*done);
        }

        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches.iter() {
                succs.push(*branch);
            }
            succs.push(*join);
        }

        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            succs.push(*entry);
            succs.push(*join);
        }

        CompiledNodeKind::RepeatCheck { done, .. } => {
            succs.push(*done);
        }

        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => {
            succs.push(*body);
            succs.push(*exhausted);
        }

        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            succs.push(*body);
            succs.push(*handler);
        }
    }

    succs
}

// ---------------------------------------------------------------------------
// Certificate 8: Loop Nesting
// ---------------------------------------------------------------------------

fn check_loop_nesting(parts: &WorkflowParts) -> Certificate {
    let mut issues: Vec<String> = Vec::new();
    let node_count = parts.nodes.len();

    // Track which nodes are loop entry points and their done targets.
    // Well-formed loops have a Start node whose done target is a Join/Finish
    // node that comes after the body. We check that loop spans don't improperly
    // cross by ensuring that inner loop done targets don't land outside an
    // outer loop's body span.
    let mut loop_spans: Vec<(StepIdx, StepIdx, StepIdx)> = Vec::new(); // (start, body, done)

    for node in parts.nodes.iter() {
        match node.kind {
            CompiledNodeKind::ForEachStart { body, done, .. }
            | CompiledNodeKind::CollectStart { body, done, .. }
            | CompiledNodeKind::ReduceStart { body, done, .. }
            | CompiledNodeKind::RepeatStart { body, done, .. } => {
                loop_spans.push((node.id, body, done));
            }
            CompiledNodeKind::TogetherStart { join, .. } => {
                // TogetherStart branches go through TogetherBranch entries
                loop_spans.push((node.id, node.id, join));
            }
            _ => {}
        }
    }

    // Check each pair of loop spans for improper nesting
    for i in 0..loop_spans.len() {
        let i_next = i.saturating_add(1);
        for j in i_next..loop_spans.len() {
            let (start_a, _body_a, done_a) = match loop_spans.get(i) {
                Some(&span) => span,
                None => continue,
            };
            let (start_b, body_b, done_b) = match loop_spans.get(j) {
                Some(&span) => span,
                None => continue,
            };

            let a_start = start_a.as_usize();
            let a_done = done_a.as_usize();
            let b_start = start_b.as_usize();
            let b_done = done_b.as_usize();

            // Skip if either span wraps around (shouldn't happen in valid IR)
            if a_done <= a_start || b_done <= b_start {
                continue;
            }

            // Check for partial overlap: B starts inside A but ends outside A
            if b_start > a_start && b_start < a_done && b_done > a_done {
                issues.push(format!(
                    "loop at step {} spans to {} but inner loop at step {} extends to {}",
                    a_start, a_done, b_start, b_done,
                ));
            }

            // Check the reverse: A starts inside B but ends outside B
            if a_start > b_start && a_start < b_done && a_done > b_done {
                issues.push(format!(
                    "loop at step {} spans to {} but inner loop at step {} extends to {}",
                    b_start, b_done, a_start, a_done,
                ));
            }

            // Check body targets are within parent span
            let body_b_usize = body_b.as_usize();
            if body_b_usize >= node_count {
                issues.push(format!(
                    "loop at step {} has body target {} out of bounds",
                    b_start, body_b_usize,
                ));
            }
        }
    }

    if issues.is_empty() {
        let loop_count = loop_spans.len();
        Certificate {
            kind: CertificateKind::LoopNesting,
            status: CertificateStatus::Pass,
            details: format!("{} loop(s) properly nested", loop_count),
        }
    } else {
        Certificate {
            kind: CertificateKind::LoopNesting,
            status: CertificateStatus::Fail(issues.join("; ")),
            details: "Loop nesting validation found improper span overlaps.".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::WorkflowDigest;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

    fn minimal_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    fn empty_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("empty").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    #[test]
    fn test_empty_nodes_fails_structural_validity() {
        let result = VerificationResult::analyze(&empty_parts());
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        let Some(cert) = structural else {
            assert!(false, "structural cert missing");
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "expected Fail for empty nodes, got {:?}",
            cert.status
        );
    }

    #[test]
    fn test_single_finish_node_passes_all() {
        let result = VerificationResult::analyze(&minimal_parts());
        // Structural validity should pass.
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        let Some(structural) = structural else {
            assert!(false, "cert missing");
            return;
        };
        assert!(matches!(
            structural.status,
            CertificateStatus::Pass
        ));

        // Strict durability should warn (Finish node present but no error handlers).
        let durability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        assert!(durability.is_some());
        let Some(durability) = durability else {
            assert!(false, "cert missing");
            return;
        };
        let dur_status = &durability.status;
        // A single Finish node with no error handlers/on_error produces Warn.
        assert!(
            matches!(
                dur_status,
                CertificateStatus::Pass | CertificateStatus::Warn(_)
            ),
            "expected Pass or Warn for strict durability, got {:?}",
            dur_status
        );

        // Reachability should pass (single node reachable from entry).
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        assert!(reachability.is_some());
        let Some(reachability) = reachability else {
            assert!(false, "cert missing");
            return;
        };
        assert!(matches!(
            reachability.status,
            CertificateStatus::Pass
        ));
    }

    #[test]
    fn test_unreachable_node_fails_reachability() {
        // Node 0 is a Nop with no next (entry), node 1 is a Finish but unreachable.
        let parts = WorkflowParts {
            name: String::from("unreachable").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Nop,
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };

        let result = VerificationResult::analyze(&parts);
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        assert!(reachability.is_some());
        let Some(reachability) = reachability else {
            assert!(false, "cert missing");
            return;
        };
        assert!(matches!(
            reachability.status,
            CertificateStatus::Fail(_)
        ));
    }

    #[test]
    fn test_analysis_counts_match() {
        let result = VerificationResult::analyze(&minimal_parts());
        // total_checks should equal the number of certificates.
        assert_eq!(result.total_checks, result.certificates.len());
        assert_eq!(result.total_checks, 8);

        // pass_count + fail_count + warn_count should equal total_checks.
        let sum = result.pass_count + result.fail_count + result.warn_count;
        assert_eq!(sum, result.total_checks);
    }

    // ========================================================================
    // Pre-flight verify_workflow tests
    // ========================================================================

    fn preflight_minimal_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("preflight-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    fn preflight_empty_parts() -> WorkflowParts {
        WorkflowParts {
            name: String::from("preflight-empty").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        }
    }

    // -- Pre-flight test 1: Structural validity --

    #[test]
    fn preflight_structural_validity_passes_for_valid_workflow() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("valid"));
    }

    #[test]
    fn preflight_structural_validity_fails_for_empty_nodes() {
        let report = verify_workflow(&preflight_empty_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("empty"));
    }

    #[test]
    fn preflight_structural_validity_fails_for_entry_out_of_bounds() {
        let mut parts = preflight_minimal_parts();
        parts.entry = StepIdx::new(99);
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("exceeds"));
    }

    #[test]
    fn preflight_structural_validity_fails_for_node_id_mismatch() {
        let mut parts = preflight_minimal_parts();
        // Create a node with wrong ID at position 0.
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(5), // wrong: should be 0
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("mismatch"));
    }

    // -- Pre-flight test 2: Bounded transitions --

    #[test]
    fn preflight_bounded_transitions_passes_for_default_contract() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
    }

    #[test]
    fn preflight_bounded_transitions_fails_for_zero_max_steps() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("max_steps"));
    }

    #[test]
    fn preflight_bounded_transitions_fails_for_zero_max_slots() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_slots = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("max_slots"));
    }

    #[test]
    fn preflight_bounded_transitions_fails_for_node_count_exceeding_max_steps() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 1;
        // Add extra nodes so node count > max_steps.
        let mut nodes = Vec::new();
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 { Some(StepIdx::new(i.saturating_add(1))) } else { None },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        nodes[4].kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("exceeds"));
    }

    // -- Pre-flight test 3: Secret-to-result leak --

    #[test]
    fn preflight_secret_to_result_leak_passes_for_clean_workflow() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "secret_to_result_leak");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("no secret"));
    }

    #[test]
    fn preflight_secret_to_result_leak_fails_for_secret_reaching_finish() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("leak-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "secret_to_result_leak");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail);
        assert!(c.detail.contains("Finish"));
    }

    // -- Pre-flight test 4: Strict durability eligibility --

    #[test]
    fn preflight_strict_durability_passes_for_safe_workflow() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: Some(StepIdx::new(2)),
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        let parts = WorkflowParts {
            name: String::from("durable-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "strict_durability_eligibility");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("error handler"));
    }

    #[test]
    fn preflight_strict_durability_warns_for_do_without_error_handler() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("non-durable-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "strict_durability_eligibility");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("error handler"));
    }

    // -- Pre-flight test 5: Action idempotency --

    #[test]
    fn preflight_action_idempotency_passes_for_no_do_nodes() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "action_idempotency");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("no Do nodes"));
    }

    #[test]
    fn preflight_action_idempotency_warns_for_unguarded_actions() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("idem-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "action_idempotency");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("retry"));
    }

    // -- Pre-flight test 6: Worst-case memory budget --

    #[test]
    fn preflight_memory_budget_passes_for_small_slot_count() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("64"));
    }

    #[test]
    fn preflight_memory_budget_passes_for_zero_slots() {
        let mut parts = preflight_minimal_parts();
        parts.slot_count = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("zero"));
    }

    #[test]
    fn preflight_memory_budget_warns_for_exceeding_output_limit() {
        let mut parts = preflight_minimal_parts();
        // Set a very low output limit so the budget exceeds it.
        // 100 slots * 64 bytes = 6400 bytes. max_output_bytes = 100.
        parts.slot_count = 100;
        parts.resource_contract.max_output_bytes = 100;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("exceeds"));
    }

    // -- Pre-flight test 7: Max transitions --

    #[test]
    fn preflight_max_transitions_passes_within_limit() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("within"));
    }

    #[test]
    fn preflight_max_transitions_fails_when_exceeding_limit() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 2;
        // Add extra nodes (3 > max_steps of 2).
        let mut nodes = Vec::new();
        for i in 0..3u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 2 { Some(StepIdx::new(i.saturating_add(1))) } else { None },
                on_error: None,
                error_slot: None,
                kind: if i < 2 {
                    CompiledNodeKind::Nop
                } else {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                },
            });
        }
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_transitions");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Fail, "detail: {}", c.detail);
        assert!(
            c.detail.contains("max_steps"),
            "expected 'max_steps' in detail, got: {}",
            c.detail,
        );
    }

    // -- Pre-flight test 8: Max action calls --

    #[test]
    fn preflight_max_action_calls_passes_within_ceiling() {
        let report = verify_workflow(&preflight_minimal_parts());
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_action_calls");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Pass);
        assert!(c.detail.contains("0 Do node"));
    }

    #[test]
    fn preflight_max_action_calls_warns_for_exceeding_ceiling() {
        let mut nodes = Vec::new();
        // Create 5 Do nodes with Finish at the end.
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: Some(StepIdx::new(i.saturating_add(1))),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: vb_core::ids::ActionId::new(u16::from(i).saturating_add(1)),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let mut parts = WorkflowParts {
            name: String::from("many-dos").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        // Default max_retry_attempts is 3, so 5 Do nodes will exceed it.
        parts.resource_contract.max_retry_attempts = 3;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_action_calls");
        assert!(check.is_some());
        let Some(c) = check else {
            assert!(false, "check missing");
            return;
        };
        assert_eq!(c.status, CheckStatus::Warn);
        assert!(c.detail.contains("5 Do nodes"));
    }

    // -- Integration test: verify_workflow produces 8 checks --

    #[test]
    fn preflight_verify_workflow_produces_eight_checks() {
        let report = verify_workflow(&preflight_minimal_parts());
        assert_eq!(report.checks.len(), 8);
    }

    #[test]
    fn preflight_verify_workflow_all_pass_report_fields() {
        let report = verify_workflow(&preflight_minimal_parts());
        // For a minimal Finish-only workflow with default contract, we expect
        // no failures (some checks may warn, but none should fail).
        assert!(report.all_pass, "all_pass should be true for minimal valid workflow, worst_risk={:?}", report.worst_risk);
        assert!(matches!(
            report.worst_risk,
            CheckStatus::Pass | CheckStatus::Warn
        ));
    }

    #[test]
    fn preflight_verify_workflow_empty_nodes_has_failures() {
        let report = verify_workflow(&preflight_empty_parts());
        assert!(!report.all_pass);
        assert_eq!(report.worst_risk, CheckStatus::Fail);
    }

    // -- CheckStatus merge_worst tests --

    #[test]
    fn check_status_merge_worst_fail_dominates() {
        assert_eq!(CheckStatus::Fail.merge_worst(CheckStatus::Pass), CheckStatus::Fail);
        assert_eq!(CheckStatus::Fail.merge_worst(CheckStatus::Warn), CheckStatus::Fail);
        assert_eq!(CheckStatus::Fail.merge_worst(CheckStatus::Fail), CheckStatus::Fail);
        assert_eq!(CheckStatus::Pass.merge_worst(CheckStatus::Fail), CheckStatus::Fail);
    }

    #[test]
    fn check_status_merge_worst_warn_dominates_pass() {
        assert_eq!(CheckStatus::Warn.merge_worst(CheckStatus::Pass), CheckStatus::Warn);
        assert_eq!(CheckStatus::Pass.merge_worst(CheckStatus::Warn), CheckStatus::Warn);
    }

    #[test]
    fn check_status_merge_worst_pass_pass() {
        assert_eq!(CheckStatus::Pass.merge_worst(CheckStatus::Pass), CheckStatus::Pass);
    }

    // ========================================================================
    // Certificate analysis: additional edge-case tests
    // ========================================================================

    #[test]
    fn analysis_entry_out_of_bounds_fails_structural() {
        let mut parts = minimal_parts();
        parts.entry = StepIdx::new(200);
        let result = VerificationResult::analyze(&parts);
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        let cert = structural.ok_or("missing").ok();
        if let Some(c) = cert {
            assert!(matches!(c.status, CertificateStatus::Fail(_)));
        }
    }

    #[test]
    fn analysis_node_id_mismatch_fails_structural() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(99), // mismatch: position 0 has id 99
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("id-mismatch").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let structural = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        assert!(structural.is_some());
        assert!(matches!(
            structural.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_zero_max_steps_fails_boundedness() {
        let mut parts = minimal_parts();
        parts.resource_contract.max_steps = 0;
        let result = VerificationResult::analyze(&parts);
        let boundedness = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Boundedness);
        assert!(boundedness.is_some());
        assert!(matches!(
            boundedness.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_node_count_exceeds_max_steps_fails_boundedness() {
        let mut parts = minimal_parts();
        parts.resource_contract.max_steps = 1;
        // Add extra nodes beyond max_steps.
        let mut nodes = Vec::new();
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 { Some(StepIdx::new(i.saturating_add(1))) } else { None },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        nodes[4].kind = CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        };
        parts.nodes = nodes.into_boxed_slice();
        let result = VerificationResult::analyze(&parts);
        let boundedness = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Boundedness);
        assert!(boundedness.is_some());
        assert!(matches!(
            boundedness.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_slot_count_exceeds_max_slots_fails_resource_bounds() {
        let mut parts = minimal_parts();
        parts.slot_count = 5000;
        parts.resource_contract.max_slots = 100;
        let result = VerificationResult::analyze(&parts);
        let rb = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::ResourceBounds);
        assert!(rb.is_some());
        assert!(matches!(
            rb.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_no_finish_node_fails_strict_durability() {
        let mut nodes = Vec::new();
        for i in 0..3u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 2 { Some(StepIdx::new(i.saturating_add(1))) } else { None },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            });
        }
        let parts = WorkflowParts {
            name: String::from("no-finish").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let dur = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        assert!(dur.is_some());
        assert!(matches!(
            dur.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_do_node_without_retry_or_error_warns_action_policy() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("do-no-retry").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ap = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::ActionPolicy);
        assert!(ap.is_some());
        assert!(matches!(
            ap.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Warn(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_taint_flow_warns_for_contained_sources() {
        // WaitEvent node without path to Finish (no next edge).
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::WaitEvent {
                event: SlotIdx::new(0),
                timeout_slot: None,
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("contained-secret").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let tf = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::TaintFlow);
        assert!(tf.is_some());
        // WaitEvent at step 0 has no outgoing edge, so source is contained -> Warn
        assert!(matches!(
            tf.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Warn(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_properly_nested_loops_pass() {
        // Outer loop: step 0 to step 4, inner loop: step 1 to step 3.
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 3,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("nested-loops").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        assert!(ln.is_some());
        assert!(matches!(
            ln.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Pass,
                ..
            })
        ));
    }

    #[test]
    fn analysis_improperly_nested_loops_fail() {
        // Outer loop: step 0 to step 3, inner loop: step 1 to step 5 (extends past outer).
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(2),
                limit: 3,
                page_size: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(6),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bad-nesting").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        assert!(ln.is_some());
        assert!(matches!(
            ln.ok_or("missing").ok(),
            Some(Certificate {
                status: CertificateStatus::Fail(_),
                ..
            })
        ));
    }

    #[test]
    fn analysis_collect_successors_for_jump_node() {
        let succs = collect_successors(
            &CompiledNodeKind::Jump {
                target: StepIdx::new(7),
            },
            None,
            None,
        );
        assert!(succs.contains(&StepIdx::new(7)));
    }

    #[test]
    fn analysis_collect_successors_for_together_start() {
        let succs = collect_successors(
            &CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
            None,
            None,
        );
        assert!(succs.contains(&StepIdx::new(1)));
        assert!(succs.contains(&StepIdx::new(2)));
        assert!(succs.contains(&StepIdx::new(3)));
    }

    #[test]
    fn analysis_collect_successors_includes_on_error() {
        let succs = collect_successors(
            &CompiledNodeKind::Nop,
            Some(StepIdx::new(1)),
            Some(StepIdx::new(5)),
        );
        assert!(succs.contains(&StepIdx::new(1)));
        assert!(succs.contains(&StepIdx::new(5)));
    }

    #[test]
    fn preflight_bounded_transitions_zero_budget_fails() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_step_budget_per_tick = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "bounded_transitions");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Fail);
            assert!(ch.detail.contains("budget_per_tick"));
        }
    }

    #[test]
    fn preflight_max_transitions_zero_steps_fails() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_steps = 0;
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_transitions");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Fail);
        }
    }

    #[test]
    fn preflight_strict_durability_no_finish_fails() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        let parts = WorkflowParts {
            name: String::from("no-finish-pf").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "strict_durability_eligibility");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Fail);
        }
    }

    #[test]
    fn preflight_action_idempotency_with_retry_check_passes() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(1),
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(1),
                body: StepIdx::new(0),
                exhausted: StepIdx::new(2),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("with-retry").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "action_idempotency");
        assert!(check.is_some());
        let c = check.ok_or("missing").ok();
        if let Some(ch) = c {
            assert_eq!(ch.status, CheckStatus::Pass);
        }
    }

    #[test]
    fn analysis_certificate_status_equality() {
        assert_eq!(CertificateStatus::Pass, CertificateStatus::Pass);
        assert_eq!(
            CertificateStatus::Fail(String::from("x")),
            CertificateStatus::Fail(String::from("x"))
        );
        assert_ne!(
            CertificateStatus::Fail(String::from("a")),
            CertificateStatus::Fail(String::from("b"))
        );
    }

    #[test]
    fn analysis_certificate_kind_copy_equality() {
        let kind = CertificateKind::TaintFlow;
        let copy = kind;
        assert_eq!(kind, copy);
    }

    // ========================================================================
    // Additional edge-case tests for coverage
    // ========================================================================

    /// Test 1: collect_successors for CollectFinish node returns no extra
    /// successors beyond next/on_error (it falls into the simple-arm match).
    #[test]
    fn collect_successors_collect_finish_returns_only_next_and_on_error() {
        let succs = collect_successors(
            &CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(5),
            },
            Some(StepIdx::new(10)),
            Some(StepIdx::new(20)),
        );
        // Should contain next and on_error, but no extra edges from the kind.
        assert!(
            succs.contains(&StepIdx::new(10)),
            "expected next=10 in successors"
        );
        assert!(
            succs.contains(&StepIdx::new(20)),
            "expected on_error=20 in successors"
        );
        assert_eq!(
            succs.len(),
            2,
            "CollectFinish should produce exactly 2 successors (next + on_error)"
        );
    }

    /// Test 2: collect_successors for ReduceFinish node returns no extra
    /// successors beyond next/on_error (it also falls into the simple-arm match).
    #[test]
    fn collect_successors_reduce_finish_returns_only_next_and_on_error() {
        let succs = collect_successors(
            &CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(7),
            },
            None,
            None,
        );
        // ReduceFinish with no next and no on_error should produce an empty vec.
        assert!(
            succs.is_empty(),
            "ReduceFinish with no next/on_error should produce no successors"
        );

        // With next only.
        let succs_next = collect_successors(
            &CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(7),
            },
            Some(StepIdx::new(3)),
            None,
        );
        assert_eq!(succs_next.len(), 1);
        assert!(succs_next.contains(&StepIdx::new(3)));
    }

    /// Test 3: check_reachability with disconnected nodes -- a graph where
    /// the entry leads to some nodes but other nodes have no path from entry.
    #[test]
    fn reachability_fails_with_disconnected_nodes() {
        let mut nodes = Vec::new();
        // Node 0: Nop -> Node 1
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        // Node 1: Finish
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        // Node 2: disconnected Nop (no one points to it)
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        // Node 3: disconnected Finish (no one points to it)
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("disconnected").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(cert) = reachability else {
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "expected Fail for disconnected nodes, got {:?}",
            cert.status
        );
        if let CertificateStatus::Fail(ref msg) = cert.status {
            // Should report 2 unreachable nodes (step 2 and step 3).
            assert!(
                msg.contains("2 unreachable"),
                "expected '2 unreachable' in message, got: {}",
                msg,
            );
        }
    }

    /// Test 4: check_boundedness with zero max_slots should fail.
    #[test]
    fn boundedness_fails_with_zero_max_slots() {
        let mut parts = minimal_parts();
        parts.resource_contract.max_slots = 0;
        let result = VerificationResult::analyze(&parts);
        let boundedness = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Boundedness);
        let Some(cert) = boundedness else {
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "expected Fail for zero max_slots, got {:?}",
            cert.status
        );
        if let CertificateStatus::Fail(ref msg) = cert.status {
            assert!(
                msg.contains("max_slots is zero"),
                "expected 'max_slots is zero' in failure, got: {}",
                msg,
            );
        }
    }

    /// Test 5: check_preflight_max_transitions with max_steps = u16::MAX
    /// should pass even with a non-trivial node count.
    #[test]
    fn preflight_max_transitions_passes_at_u16_max() {
        let mut nodes = Vec::new();
        // Build a chain of 500 nodes, well under u16::MAX.
        for i in 0..500u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 499 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: if i < 499 {
                    CompiledNodeKind::Nop
                } else {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                },
            });
        }
        let parts = WorkflowParts {
            name: String::from("big-steps").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_steps: u16::MAX,
                ..ResourceContract::DEFAULT
            },
            step_names: Vec::new().into_boxed_slice(),
        };
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_transitions");
        let Some(c) = check else {
            return;
        };
        assert_eq!(
            c.status,
            CheckStatus::Pass,
            "expected Pass for max_steps=u16::MAX with 500 nodes, got: {}",
            c.detail,
        );
    }

    /// Test 6: Empty node list -- both the certificate analysis and pre-flight
    /// verify_workflow should report failures for an empty node array.
    #[test]
    fn empty_node_list_fails_all_structural_checks() {
        let empty = empty_parts();

        // Certificate analysis: structural validity should fail.
        let cert_result = VerificationResult::analyze(&empty);
        let structural = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        let Some(cert) = structural else {
            return;
        };
        assert!(
            matches!(cert.status, CertificateStatus::Fail(_)),
            "certificate structural should Fail for empty nodes"
        );

        // Certificate analysis: reachability should also fail for empty nodes.
        let reach = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(reach_cert) = reach else {
            return;
        };
        assert!(
            matches!(reach_cert.status, CertificateStatus::Fail(_)),
            "certificate reachability should Fail for empty nodes"
        );

        // Pre-flight: structural_validity should fail.
        let pf_report = verify_workflow(&empty);
        assert!(
            !pf_report.all_pass,
            "pre-flight all_pass should be false for empty nodes"
        );
        let pf_struct = pf_report
            .checks
            .iter()
            .find(|c| c.name == "structural_validity");
        let Some(pf_s) = pf_struct else {
            return;
        };
        assert_eq!(
            pf_s.status,
            CheckStatus::Fail,
            "pre-flight structural should be Fail for empty nodes"
        );
    }

    /// Test 7: Single Finish node workflow should pass certificate analysis
    /// and pre-flight checks (reachable, structurally valid, durable enough).
    #[test]
    fn single_finish_node_workflow_passes_validation() {
        let parts = minimal_parts();

        // Certificate analysis: all key checks should pass or warn.
        let cert_result = VerificationResult::analyze(&parts);

        // Structural validity must pass.
        let structural = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StructuralValidity);
        let Some(s) = structural else {
            return;
        };
        assert!(
            matches!(s.status, CertificateStatus::Pass),
            "structural should pass for single Finish node"
        );

        // Reachability must pass (single node reachable from entry).
        let reach = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(r) = reach else {
            return;
        };
        assert!(
            matches!(r.status, CertificateStatus::Pass),
            "reachability should pass for single Finish node"
        );

        // Strict durability should pass or warn (Finish present, no error handlers).
        let dur = cert_result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        let Some(d) = dur else {
            return;
        };
        assert!(
            matches!(d.status, CertificateStatus::Pass | CertificateStatus::Warn(_)),
            "strict durability should Pass or Warn for single Finish, got {:?}",
            d.status,
        );

        // Pre-flight should report all_pass.
        let pf = verify_workflow(&parts);
        assert!(
            pf.all_pass,
            "pre-flight all_pass should be true for single Finish node, worst={:?}",
            pf.worst_risk,
        );
    }

    /// Test 8: ForEachStart creates correct successor edges (body + done
    /// in addition to next/on_error).
    #[test]
    fn collect_successors_for_each_start_includes_body_and_done() {
        let succs = collect_successors(
            &CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(5),
                done: StepIdx::new(20),
            },
            Some(StepIdx::new(99)),  // next
            Some(StepIdx::new(50)),  // on_error
        );

        // Should contain: next(99), on_error(50), body(5), done(20).
        assert!(
            succs.contains(&StepIdx::new(99)),
            "expected next=99"
        );
        assert!(
            succs.contains(&StepIdx::new(50)),
            "expected on_error=50"
        );
        assert!(
            succs.contains(&StepIdx::new(5)),
            "expected body=5"
        );
        assert!(
            succs.contains(&StepIdx::new(20)),
            "expected done=20"
        );
        assert_eq!(
            succs.len(),
            4,
            "ForEachStart should produce 4 successors, got {:?}",
            succs,
        );
    }

    // ========================================================================
    // BLACK HAT security-focused tests
    // ========================================================================

    /// BLACKHAT_cert_bfs_not_bfs [MEDIUM]: check_reachability uses Vec::pop()
    /// (DFS/LIFO), not VecDeque (BFS/FIFO), despite the comment saying "BFS
    /// from entry". This affects traversal order but not reachability
    /// correctness. The test documents the discrepancy.
    #[test]
    fn blackhat_cert_bfs_uses_vec_pop_which_is_dfs() {
        let mut nodes = Vec::new();
        // Linear chain: 0 -> 1 -> 2 -> 3 -> 4 (Finish)
        for i in 0..5u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 4 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: if i < 4 {
                    CompiledNodeKind::Nop
                } else {
                    CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    }
                },
            });
        }
        let parts = WorkflowParts {
            name: String::from("bh-bfs").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        let Some(cert) = reachability else { return };
        // Reachability is still correct despite DFS vs BFS.
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "reachability should still pass with DFS traversal"
        );
    }

    /// BLACKHAT_cert_loop_nesting_misses_reverse_overlap [MEDIUM]:
    /// check_loop_nesting iterates i from 0..N and j from i+1..N, checking
    /// both forward and reverse partial overlaps. However, the reverse check
    /// (`a_start > b_start && a_start < b_done && a_done > b_done`) is
    /// redundant because if B is after A in the array, B cannot start before A
    /// unless the indices are non-monotonic. The test confirms proper nesting
    /// detection still works for valid loops.
    #[test]
    fn blackhat_cert_loop_nesting_reverse_overlap_redundant() {
        // Two properly nested loops where inner done == outer done (valid).
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: SlotIdx::new(2),
                limit: 3,
                page_size: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(5),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-nesting").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 8,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        let Some(cert) = ln else { return };
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "properly nested loops should pass nesting check"
        );
    }

    /// BLACKHAT_cert_boundedness_u16_truncation [MEDIUM]: check_boundedness
    /// converts node count to u16 via `u16::try_from(parts.nodes.len())`
    /// clamping to u16::MAX. If a workflow has more than 65535 nodes, the
    /// comparison against max_steps (u16) silently succeeds because
    /// u16::MAX <= max_steps, even though the real node count exceeds it.
    #[test]
    fn blackhat_cert_boundedness_large_node_count_clamped_to_u16_max() {
        // We cannot actually create 65536+ nodes in a test (too much memory),
        // but we can verify the clamp logic directly.
        let large_count: usize = 70_000;
        let clamped = u16::try_from(large_count).unwrap_or(u16::MAX);
        assert_eq!(
            clamped,
            u16::MAX,
            "BLACKHAT [MEDIUM]: node count > u16::MAX is clamped to u16::MAX, \
             hiding overflow in boundedness check"
        );
        // With max_steps = u16::MAX, the clamped value would pass even though
        // the real count exceeds it. This test documents the truncation risk.
    }

    /// BLACKHAT_cert_max_action_calls_zero_ceiling [LOW]:
    /// check_preflight_max_action_calls uses max_retry_attempts as a "ceiling"
    /// for Do node count. When max_retry_attempts is 0, the condition
    /// `do_count > retry_ceiling && retry_ceiling > 0` never triggers,
    /// so any number of Do nodes silently passes. This means a workflow with
    /// 1000 Do nodes passes if max_retry_attempts is 0.
    #[test]
    fn blackhat_cert_max_action_calls_zero_ceiling_passes_any_count() {
        let mut parts = preflight_minimal_parts();
        parts.resource_contract.max_retry_attempts = 0;
        // Add 100 Do nodes -- should still pass because retry_ceiling is 0
        // and the check guards with `retry_ceiling > 0`.
        let mut nodes = Vec::new();
        for i in 0..100u16 {
            nodes.push(CompiledNode {
                id: StepIdx::new(i),
                output: None,
                next: if i < 99 {
                    Some(StepIdx::new(i.saturating_add(1)))
                } else {
                    None
                },
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Do {
                    action: vb_core::ids::ActionId::new(u16::from(i).saturating_add(1)),
                    input: SlotIdx::new(0),
                },
            });
        }
        nodes.push(CompiledNode {
            id: StepIdx::new(100),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        parts.nodes = nodes.into_boxed_slice();
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "max_action_calls");
        let Some(c) = check else { return };
        assert_eq!(
            c.status,
            CheckStatus::Pass,
            "BLACKHAT [LOW]: 100 Do nodes pass when max_retry_attempts=0 because the \
             ceiling check is bypassed"
        );
    }

    /// BLACKHAT_cert_worst_case_memory_zero_output_limit [LOW]:
    /// check_preflight_worst_case_memory_budget only warns when
    /// worst_case_bytes > output_limit AND output_limit > 0.
    /// When output_limit is 0, the condition fails and a workflow with massive
    /// slot usage passes the memory budget check without even a warning.
    #[test]
    fn blackhat_cert_memory_budget_zero_output_limit_no_warning() {
        let mut parts = preflight_minimal_parts();
        parts.slot_count = 10000;
        parts.resource_contract.max_output_bytes = 0;
        // 10000 * 64 = 640000 bytes, but output_limit=0 skips the check.
        let report = verify_workflow(&parts);
        let check = report
            .checks
            .iter()
            .find(|c| c.name == "worst_case_memory_budget");
        let Some(c) = check else { return };
        assert_eq!(
            c.status,
            CheckStatus::Pass,
            "BLACKHAT [LOW]: 10000 slots * 64 bytes passes when max_output_bytes=0 \
             because the warning check is skipped"
        );
    }

    /// BLACKHAT_cert_action_policy_action_id_zero [LOW]:
    /// check_action_policy flags Do nodes with action_id 0 as "missing".
    /// However, the code pushes a warning string but does not check whether
    /// the action_id is actually zero in any meaningful way. The test
    /// verifies that a Do node with action_id 0 produces a Warn.
    #[test]
    fn blackhat_cert_action_id_zero_produces_warning() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: vb_core::ids::ActionId::new(0), // action_id 0
                input: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(0),
                done: StepIdx::new(1),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-action-zero").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ap = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::ActionPolicy);
        let Some(cert) = ap else { return };
        assert!(
            matches!(cert.status, CertificateStatus::Warn(_)),
            "action_id 0 should produce a Warn in ActionPolicy"
        );
    }

    /// BLACKHAT_cert_strict_durability_multiple_finish_warns [LOW]:
    /// check_strict_durability warns when there is more than one Finish node.
    /// This is correct behavior but the test documents the edge case.
    #[test]
    fn blackhat_cert_strict_durability_multiple_finish_warns() {
        let mut nodes = Vec::new();
        // Two Finish nodes
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(1),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-multi-finish").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let dur = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        let Some(cert) = dur else { return };
        // Should warn about multiple Finish nodes.
        assert!(
            matches!(cert.status, CertificateStatus::Warn(_)),
            "multiple Finish nodes should produce a Warn"
        );
    }

    /// BLACKHAT_cert_loop_nesting_done_equals_start [MEDIUM]:
    /// When a loop's done target equals its start step (degenerate loop),
    /// the code skips it via `if a_done <= a_start`. This means a loop where
    /// done == start (zero-length span) is silently ignored rather than
    /// flagged as malformed.
    #[test]
    fn blackhat_cert_loop_nesting_done_equals_start_silently_ignored() {
        let mut nodes = Vec::new();
        // Degenerate loop: start=0, done=0 (self-referencing)
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(0), // done == start
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-done-eq-start").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        let Some(cert) = ln else { return };
        // The degenerate loop is silently ignored because a_done <= a_start.
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "BLACKHAT [MEDIUM]: degenerate loop (done==start) is silently ignored in nesting check"
        );
    }

    /// BLACKHAT_cert_together_start_body_equals_id [LOW]:
    /// TogetherStart loop spans use (node.id, node.id, join) which creates
    /// a zero-length body span. This is handled differently from other loops
    /// where body is a distinct field. The test confirms this edge case.
    #[test]
    fn blackhat_cert_together_start_body_field_reused() {
        let mut nodes = Vec::new();
        nodes.push(CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1)]),
                join: StepIdx::new(2),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(1),
                join: StepIdx::new(2),
                accumulator: SlotIdx::new(0),
            },
        });
        nodes.push(CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 1,
                accumulator: SlotIdx::new(0),
            },
        });
        let parts = WorkflowParts {
            name: String::from("bh-together-body").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let result = VerificationResult::analyze(&parts);
        let ln = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::LoopNesting);
        let Some(cert) = ln else { return };
        assert!(
            matches!(cert.status, CertificateStatus::Pass),
            "TogetherStart should pass loop nesting check"
        );
    }
}
