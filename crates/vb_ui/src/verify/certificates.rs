#![forbid(unsafe_code)]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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

    let unguarded = all_action_ids
        .len()
        .saturating_sub(actions_with_retry.len());

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
        detail: format!(
            "IR step count {} within max_steps {}",
            step_count, contract_limit
        ),
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
pub(crate) fn collect_successors(
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

#[path = "certificates_tests.rs"]
mod tests;
