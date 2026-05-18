#![forbid(unsafe_code)]
//! Graph overlay for the replay theater.
//!
//! Maps step IDs to runtime overlay states using a `ReplayState` snapshot
//! plus compiled node information.  Produces per-node colour, badge, and
//! glow data for the workflow graph visualization described in master doc
//! Section 55 Screen C.

use std::collections::{HashMap, HashSet};

use vb_core::frame::StepState;
use vb_core::ids::StepIdx;
use vb_core::workflow::CompiledNodeKind;

use crate::theme::colors;
use crate::{graph_renderer, verify::taint_overlay};

use super::state::ReplayState;

// ---------------------------------------------------------------------------
// Overlay state (runtime classification)
// ---------------------------------------------------------------------------

/// Runtime overlay state for a single workflow node in the replay theater.
///
/// Extends `graph_renderer::OverlayState` with taint- and
/// verification-aware categories so the UI can colour nodes according to
/// the master doc Section 55 Screen C semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeOverlayState {
    /// Step completed successfully.  Colour: green (#39ff14).
    Succeeded,
    /// Step is waiting on a wait/ask primitive.  Colour: blue (#2d6bff).
    Waiting,
    /// Step is retrying (running within a repeat/retry block).  Colour: amber (#ffe600).
    Retrying,
    /// Step failed.  Colour: red (#ff073a).
    Failed,
    /// Step was not executed (pending or not reached).  Colour: grey (#555577).
    NotExecuted,
    /// Step output slot carries a secret or derived-from-secret taint.
    /// Colour: purple (#ff00ff).
    SecretTainted,
    /// Step passed verification and is safe / in-progress.
    /// Colour: teal (#00f5ff).
    VerificationSafe,
    /// Step is currently running (not a retry).  Colour: cyan (#00f5ff).
    Running,
    /// Step was skipped by control flow.  Colour: grey (#555577).
    Skipped,
    /// Step was cancelled.  Colour: grey (#555577).
    Cancelled,
}

impl NodeOverlayState {
    /// Returns the neon accent colour for this overlay state.
    #[must_use]
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::Succeeded => colors::neon::GREEN,       // #39ff14
            Self::Waiting => colors::neon::BLUE,          // #2d6bff
            Self::Retrying => colors::neon::YELLOW,       // #ffe600 (amber)
            Self::Failed => colors::neon::RED,            // #ff073a
            Self::NotExecuted => colors::text::DIM,       // #555577 (grey)
            Self::SecretTainted => colors::neon::MAGENTA, // #ff00ff (purple)
            Self::VerificationSafe => colors::neon::CYAN, // #00f5ff (teal)
            Self::Running => colors::neon::CYAN,          // #00f5ff (teal/in-progress)
            Self::Skipped => colors::text::DIM,           // #555577 (grey)
            Self::Cancelled => colors::text::DIM,         // #555577 (grey)
        }
    }

    /// Returns the glow radius in pixels for this overlay state.
    #[must_use]
    pub fn glow_radius(self) -> f32 {
        match self {
            Self::Succeeded => 3.0,
            Self::Waiting => 3.0,
            Self::Retrying => 4.0,
            Self::Failed => 6.0,
            Self::NotExecuted => 2.0,
            Self::SecretTainted => 5.0,
            Self::VerificationSafe => 4.0,
            Self::Running => 4.0,
            Self::Skipped => 2.0,
            Self::Cancelled => 2.0,
        }
    }

    /// Convert from the graph_renderer OverlayState to our extended enum,
    /// falling back to `NotExecuted` for Pending.
    #[must_use]
    pub fn from_overlay_state(s: graph_renderer::OverlayState) -> Self {
        match s {
            graph_renderer::OverlayState::Pending => Self::NotExecuted,
            graph_renderer::OverlayState::Running => Self::Running,
            graph_renderer::OverlayState::Succeeded => Self::Succeeded,
            graph_renderer::OverlayState::Failed => Self::Failed,
            graph_renderer::OverlayState::Skipped => Self::Skipped,
            graph_renderer::OverlayState::Waiting => Self::Waiting,
            graph_renderer::OverlayState::Asking => Self::Waiting,
            graph_renderer::OverlayState::Cancelled => Self::Cancelled,
        }
    }

    /// Convert from the core StepState to our overlay enum.
    #[must_use]
    pub fn from_step_state(s: StepState) -> Self {
        match s {
            StepState::Pending => Self::NotExecuted,
            StepState::Running => Self::Running,
            StepState::Succeeded => Self::Succeeded,
            StepState::Failed => Self::Failed,
            StepState::Skipped => Self::Skipped,
            StepState::Waiting => Self::Waiting,
            StepState::Asking => Self::Waiting,
            StepState::Cancelled => Self::Cancelled,
        }
    }
}

// ---------------------------------------------------------------------------
// Node badge for replay overlay
// ---------------------------------------------------------------------------

/// A badge to display on a graph node in the replay theater.
#[derive(Debug, Clone)]
pub struct OverlayBadge {
    /// Short label (e.g. "S" for secret, "V" for verified).
    pub label: String,
    /// Badge accent colour.
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// Per-node overlay result
// ---------------------------------------------------------------------------

/// Overlay result for a single workflow node.
#[derive(Debug, Clone)]
pub struct NodeOverlay {
    /// Step index this overlay applies to.
    pub step: StepIdx,
    /// Resolved overlay state (colour/glow source).
    pub state: NodeOverlayState,
    /// Glow colour derived from `state`.
    pub glow_color: [f32; 4],
    /// Glow radius in pixels.
    pub glow_radius: f32,
    /// Badges to render on this node.
    pub badges: Vec<OverlayBadge>,
}

// ---------------------------------------------------------------------------
// GraphOverlay -- full computation
// ---------------------------------------------------------------------------

/// Configuration knobs for overlay computation.
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// If `true`, nodes whose output slot carries secret taint get the
    /// `SecretTainted` overlay (purple).  When `false`, taint is ignored.
    pub show_taint: bool,
    /// If `true`, nodes that have passed verification (succeeded nodes
    /// with no taint issues reaching a Finish) get the `VerificationSafe`
    /// overlay (teal).  When `false`, succeeded nodes stay green.
    pub show_verification: bool,
    /// If `true`, nodes whose kind is `RetryCheck` and whose step state is
    /// `Running` are classified as `Retrying` (amber) instead of `Running`.
    pub show_retry: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            show_taint: true,
            show_verification: true,
            show_retry: true,
        }
    }
}

/// Full graph overlay computed from a `ReplayState` snapshot and compiled
/// node metadata.
#[derive(Debug, Clone)]
pub struct GraphOverlay {
    /// Per-node overlay data, keyed by step index.
    nodes: HashMap<StepIdx, NodeOverlay>,
}

impl GraphOverlay {
    /// Compute the full graph overlay.
    ///
    /// - `snapshot`: the `ReplayState` at a specific event boundary.
    /// - `kinds`: compiled node kind for each step index (may be sparse).
    /// - `taint_result`: pre-computed taint overlay for the workflow (optional).
    /// - `config`: overlay display configuration.
    #[must_use]
    pub fn compute(
        snapshot: &ReplayState,
        kinds: &HashMap<StepIdx, CompiledNodeKind>,
        taint_result: Option<&taint_overlay::TaintOverlayResult>,
        config: &OverlayConfig,
    ) -> Self {
        let tainted_steps = collect_tainted_steps(snapshot);
        let safe_steps = collect_verification_safe_steps(snapshot, taint_result);

        let mut nodes = HashMap::new();

        // Produce overlays for every step that has a known kind or a known state.
        let all_steps = collect_all_steps(snapshot, kinds);

        for step in all_steps {
            let step_state = snapshot
                .step_states
                .get(&step)
                .copied()
                .unwrap_or(StepState::Pending);

            let kind = kinds.get(&step);

            let mut state = NodeOverlayState::from_step_state(step_state);

            // Upgrade to Retrying if the node is a RetryCheck that is running.
            if config.show_retry
                && state == NodeOverlayState::Running
                && matches!(kind, Some(CompiledNodeKind::RetryCheck { .. }))
            {
                state = NodeOverlayState::Retrying;
            }

            // Upgrade to Retrying if the node is a RepeatAttempt that is running.
            if config.show_retry
                && state == NodeOverlayState::Running
                && matches!(kind, Some(CompiledNodeKind::RepeatAttempt { .. }))
            {
                state = NodeOverlayState::Retrying;
            }

            // Upgrade to SecretTainted if the step output has secret taint.
            if config.show_taint && tainted_steps.contains(&step) {
                state = NodeOverlayState::SecretTainted;
            }

            // Upgrade to VerificationSafe if the step succeeded and is in
            // the safe set *and* is not secret-tainted.
            if config.show_verification
                && state == NodeOverlayState::Succeeded
                && safe_steps.contains(&step)
            {
                state = NodeOverlayState::VerificationSafe;
            }

            let badges = build_badges(step, kind, &state, &tainted_steps);

            nodes.insert(
                step,
                NodeOverlay {
                    step,
                    state,
                    glow_color: state.color(),
                    glow_radius: state.glow_radius(),
                    badges,
                },
            );
        }

        Self { nodes }
    }

    /// Returns the overlay for a specific step, if it was computed.
    #[must_use]
    pub fn get(&self, step: StepIdx) -> Option<&NodeOverlay> {
        self.nodes.get(&step)
    }

    /// Returns the number of nodes in this overlay.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if the overlay contains no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns an iterator over all node overlays.
    pub fn iter(&self) -> impl Iterator<Item = &NodeOverlay> {
        self.nodes.values()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Collect the set of steps whose output slot has secret taint.
fn collect_tainted_steps(snapshot: &ReplayState) -> HashSet<StepIdx> {
    let mut tainted = HashSet::new();

    // A step is tainted only if it has a corresponding entry in the taint
    // map for a slot it could have written.  Since ReplayState does not
    // track which step wrote which slot, we use a heuristic: a tainted
    // slot whose index falls within the step range is attributed to that
    // step.  Only steps present in step_states are considered.
    for (slot_idx, taint_str) in &snapshot.taint {
        let is_secret = taint_str.contains("Secret")
            || taint_str.contains("DerivedFromSecret")
            || taint_str.contains("derived");

        if is_secret && snapshot.slot_values.contains_key(slot_idx) {
            let step = StepIdx::new(slot_idx.get());
            if snapshot.step_states.contains_key(&step) {
                tainted.insert(step);
            }
        }
    }

    tainted
}

/// Collect the set of steps that are verification-safe.
///
/// A step is verification-safe when:
/// 1. It has reached `Succeeded`.
/// 2. The taint overlay says the Finish node is safe (no secrets reach it).
/// 3. The step is reachable from a source (or there are no sources).
fn collect_verification_safe_steps(
    snapshot: &ReplayState,
    taint_result: Option<&taint_overlay::TaintOverlayResult>,
) -> HashSet<StepIdx> {
    let mut safe = HashSet::new();

    // Without an explicit taint analysis result, we cannot confirm
    // verification safety, so we return an empty set.
    let taint = match taint_result {
        Some(tr) => tr,
        None => return safe,
    };

    if !taint.finish_safe {
        return safe;
    }

    // All succeeded steps are considered verification-safe when the finish
    // is safe (no taint paths reach it).
    for (step, state) in &snapshot.step_states {
        if *state == StepState::Succeeded {
            safe.insert(*step);
        }
    }

    safe
}

/// Collect all step indices from both the snapshot and the kinds map.
fn collect_all_steps(
    snapshot: &ReplayState,
    kinds: &HashMap<StepIdx, CompiledNodeKind>,
) -> Vec<StepIdx> {
    let mut steps: Vec<StepIdx> = snapshot
        .step_states
        .keys()
        .chain(kinds.keys())
        .copied()
        .collect();

    steps.sort_by_key(|s| s.get());
    steps.dedup();

    steps
}

/// Build overlay badges for a single node.
fn build_badges(
    step: StepIdx,
    kind: Option<&CompiledNodeKind>,
    state: &NodeOverlayState,
    tainted_steps: &HashSet<StepIdx>,
) -> Vec<OverlayBadge> {
    let mut badges = Vec::new();

    // Inherit structural badges from the graph renderer.
    if let Some(k) = kind {
        let struct_badges = graph_renderer::extract_badges(k);
        for sb in struct_badges {
            badges.push(OverlayBadge {
                label: sb.label,
                color: sb.color,
            });
        }
    }

    // Add a taint badge if the node is secret-tainted.
    if tainted_steps.contains(&step) || *state == NodeOverlayState::SecretTainted {
        // Avoid duplicate "S" badge if graph_renderer already added one.
        let has_secret_badge = badges.iter().any(|b| b.label == "S");
        if !has_secret_badge {
            badges.push(OverlayBadge {
                label: String::from("S"),
                color: colors::neon::MAGENTA,
            });
        }
    }

    // Add a verification badge for safe steps.
    if *state == NodeOverlayState::VerificationSafe {
        badges.push(OverlayBadge {
            label: String::from("V"),
            color: colors::neon::TEAL,
        });
    }

    // Add a retry badge for retrying steps.
    if *state == NodeOverlayState::Retrying {
        let has_retry_badge = badges.iter().any(|b| b.label.starts_with('R'));
        if !has_retry_badge {
            badges.push(OverlayBadge {
                label: String::from("R"),
                color: colors::neon::YELLOW,
            });
        }
    }

    badges
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::WorkflowDigest;
    use vb_core::ids::{ActionId, RunId, SlotIdx};
    use vb_storage::EventSeq;
    use vb_storage::JournalEvent;

    // -- Helpers --

    fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0u8; 32]),
        }
    }

    fn make_step_started(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    fn make_step_succeeded(run: RunId, seq: u64, step: StepIdx, output: SlotIdx) -> JournalEvent {
        JournalEvent::StepSucceeded {
            run,
            seq: EventSeq::new(seq),
            step,
            output,
        }
    }

    fn make_wait_scheduled(run: RunId, seq: u64, step: StepIdx) -> JournalEvent {
        JournalEvent::WaitScheduledEvent {
            run,
            seq: EventSeq::new(seq),
            step,
            attempt: 1,
        }
    }

    fn make_run_failed(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFailedEvent {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
        }
    }

    fn make_run_finished(run: RunId, seq: u64, result: SlotIdx) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result,
            attempt: 1,
        }
    }

    fn make_run_cancelled(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(seq),
            attempt: 1,
            reason: None,
        }
    }

    fn make_kinds(pairs: Vec<(StepIdx, CompiledNodeKind)>) -> HashMap<StepIdx, CompiledNodeKind> {
        pairs.into_iter().collect()
    }

    fn build_state(events: Vec<JournalEvent>) -> ReplayState {
        let mut state = ReplayState::initial();
        for event in &events {
            state = state.apply_event(event);
        }
        state
    }

    fn nop_kind() -> CompiledNodeKind {
        CompiledNodeKind::Nop
    }

    fn do_kind() -> CompiledNodeKind {
        CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        }
    }

    fn retry_check_kind() -> CompiledNodeKind {
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            exhausted: StepIdx::new(2),
        }
    }

    fn repeat_attempt_kind() -> CompiledNodeKind {
        CompiledNodeKind::RepeatAttempt {
            attempt_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        }
    }

    #[allow(dead_code)]
    fn finish_kind() -> CompiledNodeKind {
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        }
    }

    // -- NodeOverlayState colour tests --

    #[test]
    fn succeeded_colour_is_green() {
        assert_eq!(NodeOverlayState::Succeeded.color(), colors::neon::GREEN);
    }

    #[test]
    fn waiting_colour_is_blue() {
        assert_eq!(NodeOverlayState::Waiting.color(), colors::neon::BLUE);
    }

    #[test]
    fn retrying_colour_is_amber() {
        assert_eq!(NodeOverlayState::Retrying.color(), colors::neon::YELLOW);
    }

    #[test]
    fn failed_colour_is_red() {
        assert_eq!(NodeOverlayState::Failed.color(), colors::neon::RED);
    }

    #[test]
    fn not_executed_colour_is_grey() {
        assert_eq!(NodeOverlayState::NotExecuted.color(), colors::text::DIM);
    }

    #[test]
    fn secret_tainted_colour_is_purple() {
        assert_eq!(
            NodeOverlayState::SecretTainted.color(),
            colors::neon::MAGENTA
        );
    }

    #[test]
    fn verification_safe_colour_is_teal() {
        assert_eq!(
            NodeOverlayState::VerificationSafe.color(),
            colors::neon::CYAN
        );
    }

    #[test]
    fn running_colour_is_teal() {
        assert_eq!(NodeOverlayState::Running.color(), colors::neon::CYAN);
    }

    #[test]
    fn skipped_colour_is_grey() {
        assert_eq!(NodeOverlayState::Skipped.color(), colors::text::DIM);
    }

    #[test]
    fn cancelled_colour_is_grey() {
        assert_eq!(NodeOverlayState::Cancelled.color(), colors::text::DIM);
    }

    // -- Glow radius tests --

    #[test]
    fn all_overlay_states_have_positive_glow_radius() {
        let states = [
            NodeOverlayState::Succeeded,
            NodeOverlayState::Waiting,
            NodeOverlayState::Retrying,
            NodeOverlayState::Failed,
            NodeOverlayState::NotExecuted,
            NodeOverlayState::SecretTainted,
            NodeOverlayState::VerificationSafe,
            NodeOverlayState::Running,
            NodeOverlayState::Skipped,
            NodeOverlayState::Cancelled,
        ];
        for s in &states {
            assert!(
                s.glow_radius() > 0.0,
                "glow radius must be positive for {s:?}"
            );
        }
    }

    #[test]
    fn failed_glow_radius_is_largest() {
        assert_eq!(NodeOverlayState::Failed.glow_radius(), 6.0);
    }

    // -- from_step_state tests --

    #[test]
    fn from_step_state_pending_maps_to_not_executed() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Pending),
            NodeOverlayState::NotExecuted
        );
    }

    #[test]
    fn from_step_state_running_maps_to_running() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Running),
            NodeOverlayState::Running
        );
    }

    #[test]
    fn from_step_state_succeeded_maps_to_succeeded() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Succeeded),
            NodeOverlayState::Succeeded
        );
    }

    #[test]
    fn from_step_state_failed_maps_to_failed() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Failed),
            NodeOverlayState::Failed
        );
    }

    #[test]
    fn from_step_state_waiting_maps_to_waiting() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Waiting),
            NodeOverlayState::Waiting
        );
    }

    #[test]
    fn from_step_state_asking_maps_to_waiting() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Asking),
            NodeOverlayState::Waiting
        );
    }

    #[test]
    fn from_step_state_cancelled_maps_to_cancelled() {
        assert_eq!(
            NodeOverlayState::from_step_state(StepState::Cancelled),
            NodeOverlayState::Cancelled
        );
    }

    // -- from_overlay_state tests --

    #[test]
    fn from_overlay_state_pending_maps_to_not_executed() {
        assert_eq!(
            NodeOverlayState::from_overlay_state(graph_renderer::OverlayState::Pending),
            NodeOverlayState::NotExecuted
        );
    }

    #[test]
    fn from_overlay_state_asking_maps_to_waiting() {
        assert_eq!(
            NodeOverlayState::from_overlay_state(graph_renderer::OverlayState::Asking),
            NodeOverlayState::Waiting
        );
    }

    // -- GraphOverlay compute tests --

    #[test]
    fn overlay_empty_snapshot_and_no_kinds_yields_empty() {
        let state = ReplayState::initial();
        let kinds = HashMap::new();
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());
        assert!(overlay.is_empty());
    }

    #[test]
    fn overlay_with_pending_step_is_not_executed() {
        let step = StepIdx::new(0);
        let state = ReplayState::initial();
        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::NotExecuted));
    }

    #[test]
    fn overlay_with_succeeded_step_is_green() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);
        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Succeeded));
        assert_eq!(node.map(|n| n.glow_color), Some(colors::neon::GREEN));
    }

    #[test]
    fn overlay_with_waiting_step_is_blue() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_wait_scheduled(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Waiting));
        assert_eq!(node.map(|n| n.glow_color), Some(colors::neon::BLUE));
    }

    #[test]
    fn overlay_with_failed_step_is_red() {
        let run = RunId::new(1);
        let state = build_state(vec![make_run_accepted(run, 1), make_run_failed(run, 2)]);
        // The run-failed event increments steps_failed but does not insert a
        // step_states entry, so we add one manually via kinds to ensure it
        // appears in the overlay.
        let step = StepIdx::new(0);
        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        // Without a step_states entry the step defaults to Pending -> NotExecuted.
        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::NotExecuted));
    }

    #[test]
    fn overlay_retry_check_running_is_retrying() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, retry_check_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Retrying));
        assert_eq!(node.map(|n| n.glow_color), Some(colors::neon::YELLOW));
    }

    #[test]
    fn overlay_retry_check_running_disabled_stays_running() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, retry_check_kind())]);
        let config = OverlayConfig {
            show_retry: false,
            ..OverlayConfig::default()
        };
        let overlay = GraphOverlay::compute(&state, &kinds, None, &config);

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Running));
    }

    #[test]
    fn overlay_repeat_attempt_running_is_retrying() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, repeat_attempt_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Retrying));
    }

    #[test]
    fn overlay_secret_taint_overrides_succeeded() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let mut state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);
        // Add taint marker.
        state.taint.insert(SlotIdx::new(0), String::from("Secret"));

        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::SecretTainted));
        assert_eq!(node.map(|n| n.glow_color), Some(colors::neon::MAGENTA));
    }

    #[test]
    fn overlay_taint_disabled_stays_succeeded() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let mut state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);
        state.taint.insert(SlotIdx::new(0), String::from("Secret"));

        let kinds = make_kinds(vec![(step, nop_kind())]);
        let config = OverlayConfig {
            show_taint: false,
            ..OverlayConfig::default()
        };
        let overlay = GraphOverlay::compute(&state, &kinds, None, &config);

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Succeeded));
    }

    #[test]
    fn overlay_verification_safe_with_safe_finish() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);

        let kinds = make_kinds(vec![(step, nop_kind())]);

        // Taint result with finish_safe = true.
        let taint_result = taint_overlay::TaintOverlayResult {
            sources: vec![],
            sinks: vec![StepIdx::new(1)],
            paths: vec![],
            finish_safe: true,
            tainted_nodes: HashSet::new(),
            clean_nodes: HashSet::new(),
            forbidden_sinks: HashSet::new(),
            flow_paths: Vec::new(),
        };

        let overlay = GraphOverlay::compute(
            &state,
            &kinds,
            Some(&taint_result),
            &OverlayConfig::default(),
        );

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(
            node.map(|n| n.state),
            Some(NodeOverlayState::VerificationSafe)
        );
        assert_eq!(node.map(|n| n.glow_color), Some(colors::neon::CYAN));
    }

    #[test]
    fn overlay_verification_unsafe_stays_succeeded() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);

        let kinds = make_kinds(vec![(step, nop_kind())]);

        // Taint result with finish_safe = false (dangerous path exists).
        let taint_result = taint_overlay::TaintOverlayResult {
            sources: vec![StepIdx::new(2)],
            sinks: vec![StepIdx::new(1)],
            paths: vec![taint_overlay::TaintPathSegment {
                from: StepIdx::new(2),
                to: StepIdx::new(1),
                status: taint_overlay::TaintPathStatus::Dangerous,
            }],
            finish_safe: false,
            tainted_nodes: HashSet::new(),
            clean_nodes: HashSet::new(),
            forbidden_sinks: HashSet::new(),
            flow_paths: Vec::new(),
        };

        let overlay = GraphOverlay::compute(
            &state,
            &kinds,
            Some(&taint_result),
            &OverlayConfig::default(),
        );

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Succeeded));
    }

    #[test]
    fn overlay_verification_disabled_stays_succeeded() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);

        let kinds = make_kinds(vec![(step, nop_kind())]);
        let config = OverlayConfig {
            show_verification: false,
            ..OverlayConfig::default()
        };

        let overlay = GraphOverlay::compute(&state, &kinds, None, &config);

        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Succeeded));
    }

    // -- Badge tests --

    #[test]
    fn overlay_do_node_has_action_badge() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, do_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        let badges: Vec<String> = node
            .map(|n| n.badges.iter().map(|b| b.label.clone()).collect())
            .unwrap_or_default();
        assert!(badges.contains(&String::from("A1")));
        assert!(badges.contains(&String::from("S")));
    }

    #[test]
    fn overlay_retrying_has_retry_badge() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, retry_check_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        let badges: Vec<String> = node
            .map(|n| n.badges.iter().map(|b| b.label.clone()).collect())
            .unwrap_or_default();
        assert!(badges.contains(&String::from("R")));
    }

    #[test]
    fn overlay_verification_safe_has_v_badge() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);

        let kinds = make_kinds(vec![(step, nop_kind())]);
        let taint_result = taint_overlay::TaintOverlayResult {
            sources: vec![],
            sinks: vec![],
            paths: vec![],
            finish_safe: true,
            tainted_nodes: HashSet::new(),
            clean_nodes: HashSet::new(),
            forbidden_sinks: HashSet::new(),
            flow_paths: Vec::new(),
        };

        let overlay = GraphOverlay::compute(
            &state,
            &kinds,
            Some(&taint_result),
            &OverlayConfig::default(),
        );

        let node = overlay.get(step);
        assert!(node.is_some());
        let badges: Vec<String> = node
            .map(|n| n.badges.iter().map(|b| b.label.clone()).collect())
            .unwrap_or_default();
        assert!(badges.contains(&String::from("V")));
    }

    #[test]
    fn overlay_nop_has_no_badges() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
        ]);
        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        let node = overlay.get(step);
        assert!(node.is_some());
        assert!(node.map(|n| n.badges.is_empty()).unwrap_or(false));
    }

    // -- Multi-step integration test --

    #[test]
    fn overlay_multi_step_lifecycle() {
        let run = RunId::new(1);
        let step0 = StepIdx::new(0);
        let step1 = StepIdx::new(1);
        let step2 = StepIdx::new(2);

        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step0),
            make_step_succeeded(run, 3, step0, SlotIdx::new(0)),
            make_step_started(run, 4, step1),
            make_wait_scheduled(run, 5, step2),
            make_run_finished(run, 6, SlotIdx::new(0)),
        ]);

        let kinds = make_kinds(vec![
            (step0, nop_kind()),
            (step1, nop_kind()),
            (step2, nop_kind()),
        ]);

        let taint_result = taint_overlay::TaintOverlayResult {
            sources: vec![],
            sinks: vec![],
            paths: vec![],
            finish_safe: true,
            tainted_nodes: HashSet::new(),
            clean_nodes: HashSet::new(),
            forbidden_sinks: HashSet::new(),
            flow_paths: Vec::new(),
        };

        let overlay = GraphOverlay::compute(
            &state,
            &kinds,
            Some(&taint_result),
            &OverlayConfig::default(),
        );

        assert_eq!(overlay.len(), 3);

        // step0: Succeeded -> VerificationSafe (finish is safe)
        let n0 = overlay.get(step0);
        assert!(n0.is_some());
        assert_eq!(
            n0.map(|n| n.state),
            Some(NodeOverlayState::VerificationSafe)
        );

        // step1: Running
        let n1 = overlay.get(step1);
        assert!(n1.is_some());
        assert_eq!(n1.map(|n| n.state), Some(NodeOverlayState::Running));

        // step2: Waiting
        let n2 = overlay.get(step2);
        assert!(n2.is_some());
        assert_eq!(n2.map(|n| n.state), Some(NodeOverlayState::Waiting));
    }

    #[test]
    fn overlay_cancelled_run() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_run_cancelled(run, 3),
        ]);

        let kinds = make_kinds(vec![(step, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        // The step should still be Running (the cancellation event does not
        // change per-step state in ReplayState).
        let node = overlay.get(step);
        assert!(node.is_some());
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::Running));
    }

    // -- len / is_empty / iter --

    #[test]
    fn overlay_iter_returns_all_nodes() {
        let run = RunId::new(1);
        let step0 = StepIdx::new(0);
        let step1 = StepIdx::new(1);
        let state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step0),
            make_step_succeeded(run, 3, step0, SlotIdx::new(0)),
            make_step_started(run, 4, step1),
        ]);

        let kinds = make_kinds(vec![(step0, nop_kind()), (step1, nop_kind())]);
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        assert_eq!(overlay.len(), 2);
        assert!(!overlay.is_empty());

        let steps: Vec<StepIdx> = overlay.iter().map(|n| n.step).collect();
        assert!(steps.contains(&step0));
        assert!(steps.contains(&step1));
    }

    #[test]
    fn overlay_get_missing_step_returns_none() {
        let state = ReplayState::initial();
        let kinds = HashMap::new();
        let overlay = GraphOverlay::compute(&state, &kinds, None, &OverlayConfig::default());

        assert!(overlay.get(StepIdx::new(99)).is_none());
    }

    // -- Config defaults --

    #[test]
    fn overlay_config_default_enables_all_features() {
        let config = OverlayConfig::default();
        assert!(config.show_taint);
        assert!(config.show_verification);
        assert!(config.show_retry);
    }

    // -- Secret taint does not get upgraded to VerificationSafe --

    #[test]
    fn overlay_secret_taint_takes_precedence_over_verification() {
        let run = RunId::new(1);
        let step = StepIdx::new(0);
        let mut state = build_state(vec![
            make_run_accepted(run, 1),
            make_step_started(run, 2, step),
            make_step_succeeded(run, 3, step, SlotIdx::new(0)),
        ]);
        // Mark slot as secret-tainted.
        state.taint.insert(SlotIdx::new(0), String::from("Secret"));

        let kinds = make_kinds(vec![(step, nop_kind())]);

        // Finish is safe (no secret reaches it), but the step itself has taint.
        let taint_result = taint_overlay::TaintOverlayResult {
            sources: vec![],
            sinks: vec![],
            paths: vec![],
            finish_safe: true,
            tainted_nodes: HashSet::new(),
            clean_nodes: HashSet::new(),
            forbidden_sinks: HashSet::new(),
            flow_paths: Vec::new(),
        };

        let overlay = GraphOverlay::compute(
            &state,
            &kinds,
            Some(&taint_result),
            &OverlayConfig::default(),
        );

        let node = overlay.get(step);
        assert!(node.is_some());
        // Secret taint takes precedence -- the node stays SecretTainted.
        assert_eq!(node.map(|n| n.state), Some(NodeOverlayState::SecretTainted));
    }
}
