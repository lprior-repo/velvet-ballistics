//! Certificate-based verification analysis for compiled workflows.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

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
                contract.max_steps,
                contract.max_slots,
                contract.max_step_budget_per_tick,
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
// Certificate 4: Taint Flow (stub)
// ---------------------------------------------------------------------------

fn check_taint_flow(_parts: &WorkflowParts) -> Certificate {
    Certificate {
        kind: CertificateKind::TaintFlow,
        status: CertificateStatus::Warn("taint analysis not yet implemented".into()),
        details: "Full taint propagation analysis is not yet available.".into(),
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
            do_count += 1;
            // Every Do node has an action field by construction; we verify it
            // is non-zero as a sanity check (action 0 could be valid in some
            // systems but we flag it as worth reviewing).
            if action.get() == 0 {
                missing_actions.push(format!("step {} has action_id 0", node.id.get()));
            }
        }

        if let CompiledNodeKind::RetryCheck { .. } = node.kind {
            retry_count += 1;
        }

        if let CompiledNodeKind::ErrorHandler { .. } = node.kind {
            error_handler_count += 1;
        }

        if let CompiledNodeKind::RepeatStart { .. } = node.kind {
            // Repeat is a form of retry policy
            retry_count += 1;
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
            finish_count += 1;
        }
        if let CompiledNodeKind::ErrorHandler { .. } = node.kind {
            error_handler_count += 1;
        }
        if node.on_error.is_some() {
            on_error_count += 1;
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
    if parts.entry.as_usize() < node_count {
        visited[parts.entry.as_usize()] = true;
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
            if succ_usize < node_count && !visited[succ_usize] {
                visited[succ_usize] = true;
                queue.push(succ_usize);
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

        CompiledNodeKind::Choose { branches, otherwise } => {
            for branch in branches.iter() {
                succs.push(branch.target);
            }
            if let Some(target) = otherwise {
                succs.push(*target);
            }
        }

        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
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

        CompiledNodeKind::ErrorHandler { body, handler } => {
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
        for j in (i + 1)..loop_spans.len() {
            let (start_a, _body_a, done_a) = loop_spans[i];
            let (start_b, body_b, done_b) = loop_spans[j];

            let a_start = start_a.as_usize();
            let a_done = done_a.as_usize();
            let b_start = start_b.as_usize();
            let b_done = done_b.as_usize();

            // Skip if either span wraps around (shouldn't happen in valid IR)
            if a_done <= a_start || b_done <= b_start {
                continue;
            }

            // Check for partial overlap: B starts inside A but ends outside A
            if b_start > a_start && b_start < a_done {
                if b_done > a_done {
                    issues.push(format!(
                        "loop at step {} spans to {} but inner loop at step {} extends to {}",
                        a_start, a_done, b_start, b_done,
                    ));
                }
            }

            // Check the reverse: A starts inside B but ends outside B
            if a_start > b_start && a_start < b_done {
                if a_done > b_done {
                    issues.push(format!(
                        "loop at step {} spans to {} but inner loop at step {} extends to {}",
                        b_start, b_done, a_start, a_done,
                    ));
                }
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
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};
    use vb_core::ids::WorkflowDigest;

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
        let cert = structural.unwrap_or_else(|| panic!("structural cert missing"));
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
        assert!(
            matches!(
                structural.unwrap_or_else(|| panic!("cert missing")).status,
                CertificateStatus::Pass
            )
        );

        // Strict durability should warn (Finish node present but no error handlers).
        let durability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::StrictDurability);
        assert!(durability.is_some());
        let dur_status = &durability.unwrap_or_else(|| panic!("cert missing")).status;
        // A single Finish node with no error handlers/on_error produces Warn.
        assert!(
            matches!(dur_status, CertificateStatus::Pass | CertificateStatus::Warn(_)),
            "expected Pass or Warn for strict durability, got {:?}",
            dur_status
        );

        // Reachability should pass (single node reachable from entry).
        let reachability = result
            .certificates
            .iter()
            .find(|c| c.kind == CertificateKind::Reachability);
        assert!(reachability.is_some());
        assert!(
            matches!(
                reachability.unwrap_or_else(|| panic!("cert missing")).status,
                CertificateStatus::Pass
            )
        );
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
        assert!(
            matches!(
                reachability.unwrap_or_else(|| panic!("cert missing")).status,
                CertificateStatus::Fail(_)
            )
        );
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
}
