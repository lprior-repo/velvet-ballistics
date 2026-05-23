#![forbid(unsafe_code)]
//! Durability/replay panel for verifying workflow replay safety.
//!
//! Examines Do nodes in a compiled workflow and checks whether their actions
//! are safe to replay after a crash or restart. Durability is inferred from
//! structural context surrounding each Do node: error handler presence,
//! retry-check coverage, and timeout/RepeatStart wrapping.
//!
//! ## Phase 2B -- Durability Verification Panel Data Model
//!
//! Provides the data model for the durability verification panel in the
//! Verification/Certificate screen. The panel reports on durability level
//! classification, individual durability checks, resource bounds compliance,
//! and resource usage metrics.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

// ---------------------------------------------------------------------------
// Re-export cyberpunk color constants from screen.rs for panel coloring.
// ---------------------------------------------------------------------------

pub use super::screen::{
    CARD_BG, NEON_CYAN, NEON_GREEN, NEON_ORANGE, NEON_RED, TEXT_DIM, TEXT_PRIMARY,
};

/// Overall replay risk classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayRisk {
    /// All checks passed; workflow is safe to replay.
    Safe,
    /// Minor issues found; replay is likely safe but not guaranteed.
    LowRisk,
    /// Significant concerns; replay may produce incorrect results.
    HighRisk,
    /// Critical issues; replay is not safe.
    Unsafe,
}

/// Result of a single durability check.
#[derive(Debug, Clone)]
pub struct DurabilityCheck {
    /// Human-readable label identifying this check.
    pub label: String,
    /// Whether the check passed.
    pub passed: bool,
    /// Human-readable detail explaining the result.
    pub detail: String,
}

/// Panel of durability checks for replay safety analysis.
#[derive(Debug, Clone)]
pub struct DurabilityPanel {
    checks: Vec<DurabilityCheck>,
}

impl DurabilityPanel {
    /// Creates an empty durability panel with no checks.
    #[must_use]
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Builds a durability panel by examining Do nodes in a workflow.
    ///
    /// Checks performed:
    /// - **journal_before_dispatch**: Every Do node should have an `on_error`
    ///   handler so that journaling occurs before the action is dispatched.
    /// - **completion_before_mutation**: Same structural guarantee -- Do nodes
    ///   with `on_error` handlers ensure completion is recorded before the
    ///   workflow mutates further state.
    /// - **reconciliation_risk**: Any Do node that is reachable from a
    ///   `RetryCheck` node has an idempotency/reconciliation concern.
    /// - **timeout_coverage**: Each Do node should be wrapped in an
    ///   `ErrorHandler` or `RepeatStart` for timeout coverage, or have an
    ///   `on_error` handler.
    #[must_use]
    pub fn from_workflow(nodes: &[CompiledNode]) -> Self {
        let mut checks = Vec::new();
        if nodes.is_empty() {
            checks.push(DurabilityCheck {
                label: String::from("journal_before_dispatch"),
                passed: true,
                detail: String::from("no Do nodes in empty workflow"),
            });
            checks.push(DurabilityCheck {
                label: String::from("completion_before_mutation"),
                passed: true,
                detail: String::from("no Do nodes in empty workflow"),
            });
            checks.push(DurabilityCheck {
                label: String::from("reconciliation_risk"),
                passed: true,
                detail: String::from("no retry-exposed Do nodes"),
            });
            checks.push(DurabilityCheck {
                label: String::from("timeout_coverage"),
                passed: true,
                detail: String::from("no Do nodes in empty workflow"),
            });
            return Self { checks };
        }

        let do_indices = collect_do_node_indices(nodes);
        let retry_targets = collect_retry_check_targets(nodes);

        // --- journal_before_dispatch ---
        let journal = check_journal_before_dispatch(nodes, &do_indices);
        checks.push(journal);

        // --- completion_before_mutation ---
        let completion = check_completion_before_mutation(nodes, &do_indices);
        checks.push(completion);

        // --- reconciliation_risk ---
        let reconciliation = check_reconciliation_risk(nodes, &do_indices, &retry_targets);
        checks.push(reconciliation);

        // --- timeout_coverage ---
        let timeout = check_timeout_coverage(nodes, &do_indices);
        checks.push(timeout);

        Self { checks }
    }

    /// Returns all durability checks.
    #[must_use]
    pub fn checks(&self) -> &[DurabilityCheck] {
        &self.checks
    }

    /// Returns true if every durability check passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns indices of all failed checks.
    #[must_use]
    pub fn failed_checks(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let mut i = 0;
        while i < self.checks.len() {
            if let Some(check) = self.checks.get(i)
                && !check.passed
            {
                result.push(i);
            }
            i = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        result
    }

    /// Returns the overall replay risk level based on failed checks.
    #[must_use]
    pub fn replay_risk_level(&self) -> ReplayRisk {
        if self.checks.is_empty() {
            return ReplayRisk::Safe;
        }
        let fail_count = self.failed_checks().len();
        if fail_count == 0 {
            return ReplayRisk::Safe;
        }
        let has_timeout_failure = self
            .checks
            .iter()
            .any(|c| !c.passed && c.label == "timeout_coverage");
        let has_reconciliation_failure = self
            .checks
            .iter()
            .any(|c| !c.passed && c.label == "reconciliation_risk");
        // Reconciliation risk with retry-exposed Do nodes is the most
        // dangerous because replay can produce duplicate side effects.
        if has_reconciliation_failure && has_timeout_failure {
            return ReplayRisk::Unsafe;
        }
        if has_reconciliation_failure {
            return ReplayRisk::HighRisk;
        }
        if fail_count > 1 {
            return ReplayRisk::HighRisk;
        }
        ReplayRisk::LowRisk
    }
}

/// Collects the array indices of all Do nodes in the workflow.
fn collect_do_node_indices(nodes: &[CompiledNode]) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i)
            && matches!(node.kind, CompiledNodeKind::Do { .. })
        {
            indices.push(i);
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    indices
}

/// Collects all StepIdx targets reachable from RetryCheck nodes.
///
/// A RetryCheck node has a `body` field that points to the step to retry.
/// Any Do node whose step index appears as a RetryCheck body target is
/// considered retry-exposed.
fn collect_retry_check_targets(nodes: &[CompiledNode]) -> Vec<StepIdx> {
    let mut targets = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i)
            && let CompiledNodeKind::RetryCheck { body, .. } = node.kind
        {
            targets.push(body);
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    targets
}

/// Checks that all Do nodes have an on_error handler (journal_before_dispatch).
fn check_journal_before_dispatch(nodes: &[CompiledNode], do_indices: &[usize]) -> DurabilityCheck {
    let mut missing = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx)
            && node.on_error.is_none()
        {
            missing.push(node.id.get());
        }
    }
    if missing.is_empty() {
        DurabilityCheck {
            label: String::from("journal_before_dispatch"),
            passed: true,
            detail: if do_indices.is_empty() {
                String::from("no Do nodes found")
            } else {
                format!("all {} Do nodes have on_error handlers", do_indices.len())
            },
        }
    } else {
        DurabilityCheck {
            label: String::from("journal_before_dispatch"),
            passed: false,
            detail: format!(
                "{} Do node(s) without on_error handler: step(s) {}",
                missing.len(),
                format_u16_slice(&missing)
            ),
        }
    }
}

/// Checks that all Do nodes have an on_error handler (completion_before_mutation).
///
/// This is the same structural check as journal_before_dispatch but framed
/// differently: the on_error handler ensures that the completion of a Do
/// action is recorded before the workflow can mutate further state.
fn check_completion_before_mutation(
    nodes: &[CompiledNode],
    do_indices: &[usize],
) -> DurabilityCheck {
    let mut missing = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx)
            && node.on_error.is_none()
        {
            missing.push(node.id.get());
        }
    }
    if missing.is_empty() {
        DurabilityCheck {
            label: String::from("completion_before_mutation"),
            passed: true,
            detail: if do_indices.is_empty() {
                String::from("no Do nodes found")
            } else {
                format!(
                    "all {} Do nodes ensure completion before mutation",
                    do_indices.len()
                )
            },
        }
    } else {
        DurabilityCheck {
            label: String::from("completion_before_mutation"),
            passed: false,
            detail: format!(
                "{} Do node(s) without completion guard: step(s) {}",
                missing.len(),
                format_u16_slice(&missing)
            ),
        }
    }
}

/// Checks whether any Do node is reachable from a RetryCheck and thus has
/// an idempotency/reconciliation concern on replay.
fn check_reconciliation_risk(
    nodes: &[CompiledNode],
    do_indices: &[usize],
    retry_targets: &[StepIdx],
) -> DurabilityCheck {
    if retry_targets.is_empty() {
        return DurabilityCheck {
            label: String::from("reconciliation_risk"),
            passed: true,
            detail: String::from("no retry-exposed Do nodes"),
        };
    }
    let mut at_risk = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx)
            && retry_targets.contains(&node.id)
        {
            at_risk.push(node.id.get());
        }
    }
    if at_risk.is_empty() {
        DurabilityCheck {
            label: String::from("reconciliation_risk"),
            passed: true,
            detail: String::from("no Do nodes under retry paths"),
        }
    } else {
        DurabilityCheck {
            label: String::from("reconciliation_risk"),
            passed: false,
            detail: format!(
                "{} Do node(s) under RetryCheck without idempotency guarantee: step(s) {}",
                at_risk.len(),
                format_u16_slice(&at_risk)
            ),
        }
    }
}

/// Checks whether each Do node has timeout coverage.
///
/// Timeout coverage is provided by:
/// - An `on_error` handler on the Do node itself, OR
/// - Being the `body` target of a `RepeatStart` or `RepeatAttempt` node, OR
/// - Being within an `ErrorHandler` node's body.
fn check_timeout_coverage(nodes: &[CompiledNode], do_indices: &[usize]) -> DurabilityCheck {
    let error_handler_bodies = collect_error_handler_bodies(nodes);
    let repeat_bodies = collect_repeat_bodies(nodes);

    let mut uncovered = Vec::new();
    for &idx in do_indices {
        if let Some(node) = nodes.get(idx) {
            if node.on_error.is_some() {
                continue;
            }
            if error_handler_bodies.contains(&node.id) || repeat_bodies.contains(&node.id) {
                continue;
            }
            uncovered.push(node.id.get());
        }
    }
    if uncovered.is_empty() {
        DurabilityCheck {
            label: String::from("timeout_coverage"),
            passed: true,
            detail: if do_indices.is_empty() {
                String::from("no Do nodes found")
            } else {
                format!("all {} Do nodes have timeout coverage", do_indices.len())
            },
        }
    } else {
        DurabilityCheck {
            label: String::from("timeout_coverage"),
            passed: false,
            detail: format!(
                "{} Do node(s) without timeout coverage: step(s) {}",
                uncovered.len(),
                format_u16_slice(&uncovered)
            ),
        }
    }
}

/// Collects all step indices that are the `body` of an ErrorHandler node.
fn collect_error_handler_bodies(nodes: &[CompiledNode]) -> Vec<StepIdx> {
    let mut bodies = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i)
            && let CompiledNodeKind::ErrorHandler { body, .. } = node.kind
        {
            bodies.push(body);
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    bodies
}

/// Collects all step indices that are the body of RepeatStart or RepeatAttempt nodes.
fn collect_repeat_bodies(nodes: &[CompiledNode]) -> Vec<StepIdx> {
    let mut bodies = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        if let Some(node) = nodes.get(i) {
            match &node.kind {
                CompiledNodeKind::RepeatStart { body, .. } => {
                    bodies.push(*body);
                }
                CompiledNodeKind::RepeatAttempt { body, .. } => {
                    bodies.push(*body);
                }
                _ => {}
            }
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    bodies
}

/// Formats a slice of u16 values as a comma-separated string.
fn format_u16_slice(values: &[u16]) -> String {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < values.len() {
        if let Some(&v) = values.get(i) {
            parts.push(v.to_string());
        }
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }
    parts.join(", ")
}

impl Default for DurabilityPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Phase 2B: Durability Verification Panel Data Model
// ===========================================================================

/// Durability guarantee level for a workflow.
///
/// Each level represents an increasingly strong guarantee about how
/// workflow state is persisted and recovered after a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum DurabilityLevel {
    /// No explicit journaling; best-effort execution only.
    /// State may be lost or duplicated on crash/restart.
    #[default]
    BestEffort,
    /// Actions are journaled before dispatch so that replay can
    /// detect and skip already-completed work.
    Journaled,
    /// Full strict mode: journaling, completion guards, and
    /// idempotency guarantees are all enforced.
    Strict,
}

impl DurabilityLevel {
    /// Returns the human-readable label for this durability level.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::BestEffort => "BestEffort",
            Self::Journaled => "Journaled",
            Self::Strict => "Strict",
        }
    }

    /// Returns the cyberpunk hex color associated with this level.
    #[must_use]
    pub fn color(&self) -> &'static str {
        match self {
            Self::BestEffort => NEON_ORANGE,
            Self::Journaled => NEON_CYAN,
            Self::Strict => NEON_GREEN,
        }
    }

    /// Returns a numeric rank for comparison: higher is more durable.
    #[must_use]
    pub fn rank(&self) -> u8 {
        match self {
            Self::BestEffort => 0,
            Self::Journaled => 1,
            Self::Strict => 2,
        }
    }
}

/// One check in the Phase 2B durability verification panel.
///
/// Each check validates a specific durability property and reports
/// whether it passed, along with a human-readable detail string.
#[derive(Debug, Clone)]
pub struct DurabilityVerifyCheck {
    /// The durability level this check is associated with.
    pub level: DurabilityLevel,
    /// Whether the check passed.
    pub passed: bool,
    /// Human-readable detail explaining the result.
    pub detail: String,
}

impl DurabilityVerifyCheck {
    /// Creates a new durability verification check.
    #[must_use]
    pub fn new(level: DurabilityLevel, passed: bool, detail: &str) -> Self {
        Self {
            level,
            passed,
            detail: String::from(detail),
        }
    }
}

/// Resource budget limits for the durability verification panel.
///
/// Defines the maximum allowed memory, CPU time, and wall-clock time
/// for the verification process itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceBudgetBounds {
    /// Maximum memory usage in megabytes.
    pub max_memory_mb: u64,
    /// Maximum CPU time in milliseconds.
    pub max_cpu_ms: u64,
    /// Maximum wall-clock time in milliseconds.
    pub max_wall_ms: u64,
}

impl ResourceBudgetBounds {
    /// Creates new resource budget bounds.
    #[must_use]
    pub fn new(max_memory_mb: u64, max_cpu_ms: u64, max_wall_ms: u64) -> Self {
        Self {
            max_memory_mb,
            max_cpu_ms,
            max_wall_ms,
        }
    }

    /// Returns default resource bounds used when no custom bounds are specified.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            max_memory_mb: 512,
            max_cpu_ms: 5000,
            max_wall_ms: 10_000,
        }
    }
}

impl Default for ResourceBudgetBounds {
    fn default() -> Self {
        Self::defaults()
    }
}

/// A single resource usage metric comparing actual usage against a limit.
#[derive(Debug, Clone)]
pub struct DurabilityResourceMetric {
    /// Human-readable name of this metric (e.g. "memory", "cpu_time", "wall_time").
    pub name: String,
    /// Actual usage measured.
    pub used: u64,
    /// Configured limit.
    pub limit: u64,
}

impl DurabilityResourceMetric {
    /// Creates a new resource metric.
    #[must_use]
    pub fn new(name: &str, used: u64, limit: u64) -> Self {
        Self {
            name: String::from(name),
            used,
            limit,
        }
    }

    /// Returns true if usage is within the configured limit.
    #[must_use]
    pub fn within_bounds(&self) -> bool {
        self.used <= self.limit
    }

    /// Returns the fraction of limit used, clamped to avoid division issues.
    /// Returns 0.0 if limit is zero.
    #[must_use]
    pub fn utilization(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        // Use f64 conversion via From -- u64 -> f64 is always safe.
        let used_f = f64::from(u32::try_from(self.used).unwrap_or(u32::MAX));
        let limit_f = f64::from(u32::try_from(self.limit).unwrap_or(u32::MAX));
        if limit_f == 0.0 {
            return 0.0;
        }
        let ratio = used_f / limit_f;
        if ratio > 1.0 { 1.0 } else { ratio }
    }

    /// Returns the cyberpunk color based on utilization.
    #[must_use]
    pub fn status_color(&self) -> &'static str {
        if self.within_bounds() {
            if self.utilization() > 0.9 {
                NEON_ORANGE
            } else {
                NEON_GREEN
            }
        } else {
            NEON_RED
        }
    }
}

/// Complete durability verification report for the Phase 2B panel.
///
/// Aggregates the overall durability level, individual check results,
/// resource usage metrics, and a timestamp.
#[derive(Debug, Clone)]
pub struct DurabilityReport {
    /// The overall durability level for this workflow.
    pub level: DurabilityLevel,
    /// Individual durability verification checks.
    pub checks: Vec<DurabilityVerifyCheck>,
    /// Resource usage metrics for the verification process.
    pub resource_metrics: Vec<DurabilityResourceMetric>,
    /// Timestamp of this report in microseconds since epoch.
    pub timestamp_micros: u64,
}

impl DurabilityReport {
    /// Creates a new durability report.
    #[must_use]
    pub fn new(
        level: DurabilityLevel,
        checks: Vec<DurabilityVerifyCheck>,
        resource_metrics: Vec<DurabilityResourceMetric>,
        timestamp_micros: u64,
    ) -> Self {
        Self {
            level,
            checks,
            resource_metrics,
            timestamp_micros,
        }
    }

    /// Creates an empty report with default values and no checks.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            level: DurabilityLevel::default(),
            checks: Vec::new(),
            resource_metrics: Vec::new(),
            timestamp_micros: 0,
        }
    }

    /// Returns true if all durability checks passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    /// Returns the number of checks that passed.
    #[must_use]
    pub fn pass_count(&self) -> usize {
        self.checks.iter().filter(|c| c.passed).count()
    }

    /// Returns the number of checks that failed.
    #[must_use]
    pub fn fail_count(&self) -> usize {
        self.checks.iter().filter(|c| !c.passed).count()
    }

    /// Returns true if all resource metrics are within bounds.
    #[must_use]
    pub fn resources_within_bounds(&self) -> bool {
        self.resource_metrics.iter().all(|m| m.within_bounds())
    }

    /// Returns a summary string, e.g. "3/5 checks passed".
    #[must_use]
    pub fn summary(&self) -> String {
        let total = self.checks.len();
        let passed = self.pass_count();
        format!("{}/{} checks passed", passed, total)
    }
}

impl Default for DurabilityReport {
    fn default() -> Self {
        Self::empty()
    }
}

// ---------------------------------------------------------------------------
// Phase 2B helper functions
// ---------------------------------------------------------------------------

/// Determines the effective durability level for a workflow based on
/// the presence of error handlers, journaling infrastructure, and
/// idempotency guarantees.
///
/// Returns `Strict` if all Do nodes have on_error handlers and no
/// retry-exposed Do nodes exist. Returns `Journaled` if all Do nodes
/// have on_error handlers but retry-exposed nodes are present. Returns
/// `BestEffort` otherwise.
#[must_use]
pub fn check_durability_level(panel: &DurabilityPanel) -> DurabilityLevel {
    let checks = panel.checks();
    let journal_passed = checks
        .iter()
        .any(|c| c.label == "journal_before_dispatch" && c.passed);
    let completion_passed = checks
        .iter()
        .any(|c| c.label == "completion_before_mutation" && c.passed);
    let recon_passed = checks
        .iter()
        .any(|c| c.label == "reconciliation_risk" && c.passed);
    let timeout_passed = checks
        .iter()
        .any(|c| c.label == "timeout_coverage" && c.passed);

    let has_journal_and_completion = journal_passed && completion_passed;

    if has_journal_and_completion && recon_passed && timeout_passed {
        DurabilityLevel::Strict
    } else if has_journal_and_completion {
        DurabilityLevel::Journaled
    } else {
        DurabilityLevel::BestEffort
    }
}

/// Computes resource usage metrics from measured values and configured bounds.
///
/// Returns a vector of `DurabilityResourceMetric` instances comparing each
/// measured value against its configured limit.
#[must_use]
pub fn compute_resource_usage(
    memory_used_mb: u64,
    cpu_used_ms: u64,
    wall_used_ms: u64,
    bounds: &ResourceBudgetBounds,
) -> Vec<DurabilityResourceMetric> {
    vec![
        DurabilityResourceMetric::new("memory", memory_used_mb, bounds.max_memory_mb),
        DurabilityResourceMetric::new("cpu_time", cpu_used_ms, bounds.max_cpu_ms),
        DurabilityResourceMetric::new("wall_time", wall_used_ms, bounds.max_wall_ms),
    ]
}

/// Builds a complete `DurabilityReport` from a `DurabilityPanel`, measured
/// resource usage, resource bounds, and a timestamp.
#[must_use]
pub fn build_durability_report(
    panel: &DurabilityPanel,
    memory_used_mb: u64,
    cpu_used_ms: u64,
    wall_used_ms: u64,
    bounds: &ResourceBudgetBounds,
    timestamp_micros: u64,
) -> DurabilityReport {
    let level = check_durability_level(panel);
    let checks: Vec<DurabilityVerifyCheck> = panel
        .checks()
        .iter()
        .map(|c| DurabilityVerifyCheck::new(level, c.passed, &c.detail))
        .collect();
    let resource_metrics =
        compute_resource_usage(memory_used_mb, cpu_used_ms, wall_used_ms, bounds);

    DurabilityReport::new(level, checks, resource_metrics, timestamp_micros)
}

#[path = "durability_tests.rs"]
mod tests;
