#![forbid(unsafe_code)]
//! Replay Theater screen layout model (Phase 1F).
//!
//! Provides the data skeleton for the Makepad 2.0 Splash DSL layout
//! that renders the Run Replay Theater.  This is a LAYOUT-ONLY
//! implementation -- all data is placeholder; no IPC wiring yet.
//!
//! Layout structure:
//! ```text
//! +---------------------------------------------------------------+
//! | vb -- Replay Theater  [Run: 8172] [Workflow: issue-triage]   |
//! +----------------------------+----------------------------------+
//! |                            |  Detail Inspector                 |
//! |   Workflow Graph           |  +- Step: github.issue.create -+ |
//! |   with Run Overlay         |  | Kind: Do                    | |
//! |   (placeholder canvas)     |  +-----------------------------+ |
//! |                            |  +- Action Ticket #42 ---------+ |
//! |                            |  +-----------------------------+ |
//! |                            |  +- Slot Diffs ----------------+ |
//! |                            |  +-----------------------------+ |
//! +----------------------------+----------------------------------+
//! | [|<] [<] [>] [>|] 1x | [jump: failure] [action] [done]      |
//! | --*--[RunAccepted]--[Step:0]--[ActionScheduled]--[Completed]- |
//! +---------------------------------------------------------------+
//! ```

use crate::replay::slot_panel::SlotDiffPanel;
use crate::replay::timeline::{TimelineChip, TimelineStrip};

// ---------------------------------------------------------------------------
// Color constants -- cyberpunk palette
// ---------------------------------------------------------------------------

/// Panel background: `#12121f`.
pub const PANEL_BG: &str = "#12121f";
/// Card background: `#16162a`.
pub const CARD_BG: &str = "#16162a";
/// Border color: `#2a2a4a`.
pub const BORDER: &str = "#2a2a4a";
/// Primary text: `#e8e8ff`.
pub const TEXT_PRIMARY: &str = "#e8e8ff";
/// Secondary text: `#8888aa`.
pub const TEXT_SECONDARY: &str = "#8888aa";
/// Neon cyan accent: `#00f5ff`.
pub const NEON_CYAN: &str = "#00f5ff";
/// Neon green accent: `#39ff14`.
pub const NEON_GREEN: &str = "#39ff14";
/// Neon red accent: `#ff073a`.
pub const NEON_RED: &str = "#ff073a";
/// Neon orange accent: `#ff6b00`.
pub const NEON_ORANGE: &str = "#ff6b00";
/// Text dim / label color: `#555577`.
pub const TEXT_DIM: &str = "#555577";
/// Canvas background: `#0a0a12`.
pub const CANVAS_BG: &str = "#0a0a12";

// ---------------------------------------------------------------------------
// Detail inspector cards
// ---------------------------------------------------------------------------

/// A single key-value row inside a detail card.
#[derive(Debug, Clone)]
pub struct InspectorField {
    /// Label (left column), e.g. "Step:".
    pub key: String,
    /// Value (right column), e.g. "github.issue.create".
    pub value: String,
    /// Hex color for the value text (Splash DSL `draw_text.color`).
    pub value_color: String,
}

/// One card in the detail inspector panel.
#[derive(Debug, Clone)]
pub struct InspectorCard {
    /// Card title, e.g. "Step Inspector".
    pub title: String,
    /// Title color (hex).
    pub title_color: String,
    /// Key-value fields inside the card.
    pub fields: Vec<InspectorField>,
}

// ---------------------------------------------------------------------------
// Selected Event Panel
// ---------------------------------------------------------------------------

/// Neon purple (#b14dff) -- taint / secret-sensitive.
pub const NEON_PURPLE: &str = "#b14dff";
/// Neon teal (#00e5c7) -- durable / replay-safe.
pub const NEON_TEAL: &str = "#00e5c7";

/// The selected event panel shows details of the currently highlighted journal
/// event in the timeline.
#[derive(Debug, Clone)]
pub struct SelectedEventPanel {
    /// Journal event sequence number.
    pub seq: String,
    /// Event timestamp in microseconds.
    pub timestamp_micros: String,
    /// Shard / replica identifier.
    pub shard_id: String,
    /// Step index this event relates to, if any.
    pub step: String,
    /// Event kind label, e.g. "ActionFailed".
    pub event_kind: String,
    /// Evidence identifier associated with this event.
    pub evidence_id: String,
    /// Short SHA256 digest of the event payload.
    pub digest_summary: String,
    /// Color for the event kind label.
    pub event_kind_color: String,
}

impl SelectedEventPanel {
    /// Returns a placeholder SelectedEventPanel with populated string fields.
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            seq: String::from("3"),
            timestamp_micros: String::from("1715000000000000"),
            shard_id: String::from("shard-0"),
            step: String::from("0"),
            event_kind: String::from("ActionFailed"),
            evidence_id: String::from("ev-0042"),
            digest_summary: String::from("sha256:a3f1…c9d2"),
            event_kind_color: String::from(NEON_RED),
        }
    }
}

// ---------------------------------------------------------------------------
// Slot Diff Table
// ---------------------------------------------------------------------------

/// A single row in the slot diff table.
#[derive(Debug, Clone)]
pub struct SlotDiffRow {
    /// Formatted slot identifier, e.g. "SlotIdx(3)".
    pub slot_id: String,
    /// Formatted previous value, e.g. "I64(0)".
    pub before: String,
    /// Formatted new value, e.g. "I64(42)".
    pub after: String,
    /// Previous taint label, e.g. "Clean".
    pub taint_before: String,
    /// New taint label, e.g. "Secret".
    pub taint_after: String,
}

/// The slot diff table shows computed differences between two replay states.
#[derive(Debug, Clone)]
pub struct SlotDiffTable {
    /// Table header row labels.
    pub headers: Vec<String>,
    /// Data rows.
    pub rows: Vec<SlotDiffRow>,
}

impl SlotDiffTable {
    /// Returns a placeholder SlotDiffTable with two sample rows.
    #[must_use]
    pub fn placeholder() -> Self {
        let headers = vec![
            String::from("slot"),
            String::from("before"),
            String::from("after"),
            String::from("taint before"),
            String::from("taint after"),
        ];
        let rows = vec![
            SlotDiffRow {
                slot_id: String::from("SlotIdx(0)"),
                before: String::from("<empty>"),
                after: String::from("Null"),
                taint_before: String::from("Clean"),
                taint_after: String::from("Clean"),
            },
            SlotDiffRow {
                slot_id: String::from("SlotIdx(3)"),
                before: String::from("I64(0)"),
                after: String::from("I64(42)"),
                taint_before: String::from("Clean"),
                taint_after: String::from("Secret"),
            },
        ];
        Self { headers, rows }
    }
}

// ---------------------------------------------------------------------------
// Recovery Decision Panel
// ---------------------------------------------------------------------------

/// The recovery decision panel shows the active recovery strategy and its
/// parameters when the replay has reached a failure boundary.
#[derive(Debug, Clone)]
pub struct RecoveryDecisionPanel {
    /// Recovery strategy name, e.g. "Retry", "Abort", "Skip".
    pub strategy: String,
    /// Strategy accent color.
    pub strategy_color: String,
    /// Maximum retry attempts before giving up.
    pub max_attempts: String,
    /// Whether idempotent replay is required.
    pub idempotency_required: String,
    /// Human-readable apply/replay action label.
    pub apply_action: String,
    /// Color for the apply action label.
    pub apply_action_color: String,
}

impl RecoveryDecisionPanel {
    /// Returns a placeholder RecoveryDecisionPanel.
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            strategy: String::from("Retry"),
            strategy_color: String::from(NEON_ORANGE),
            max_attempts: String::from("3"),
            idempotency_required: String::from("true"),
            apply_action: String::from("apply: github.issue.create"),
            apply_action_color: String::from(NEON_CYAN),
        }
    }
}

// ---------------------------------------------------------------------------
// Live Mode Toggle
// ---------------------------------------------------------------------------

/// The live/frozen toggle in the transport bar.
#[derive(Debug, Clone)]
pub struct LiveModeToggle {
    /// Toggle label, e.g. "live" or "frozen".
    pub label: String,
    /// `true` = live mode, `false` = frozen mode.
    pub is_live: bool,
    /// Accent color (neon green when live, dim when frozen).
    pub color: String,
}

impl LiveModeToggle {
    /// Returns the live toggle in live mode.
    #[must_use]
    pub fn live() -> Self {
        Self {
            label: String::from("live"),
            is_live: true,
            color: String::from(NEON_GREEN),
        }
    }

    /// Returns the live toggle in frozen mode.
    #[must_use]
    pub fn frozen() -> Self {
        Self {
            label: String::from("frozen"),
            is_live: false,
            color: String::from(TEXT_DIM),
        }
    }
}

// ---------------------------------------------------------------------------
// Transport bar
// ---------------------------------------------------------------------------

/// A button descriptor in the transport control bar.
#[derive(Debug, Clone)]
pub struct TransportButton {
    /// Button label, e.g. "|<", "<", ">", ">|".
    pub label: String,
    /// Whether the button is currently enabled.
    pub enabled: bool,
}

/// A jump-chip in the transport bar (e.g. "jump: failure").
#[derive(Debug, Clone)]
pub struct JumpChip {
    /// Chip label text.
    pub label: String,
    /// Accent color for the chip.
    pub color: String,
}

/// Transport bar state for rendering.
#[derive(Debug, Clone)]
pub struct TransportBar {
    /// Playback transport buttons.
    pub buttons: Vec<TransportButton>,
    /// Current playback speed label, e.g. "1x".
    pub speed_label: String,
    /// Jump chips (quick navigation).
    pub jump_chips: Vec<JumpChip>,
    /// Live/frozen mode toggle.
    pub live_toggle: LiveModeToggle,
}

// ---------------------------------------------------------------------------
// Graph overlay node
// ---------------------------------------------------------------------------

/// A single node in the workflow graph overlay.
#[derive(Debug, Clone)]
pub struct GraphNode {
    /// Display name, e.g. "Do", "SetConst", "Finish".
    pub name: String,
    /// Node kind label, e.g. "github.issue.create".
    pub action_label: Option<String>,
    /// State label, e.g. "succeeded", "running".
    pub state_label: String,
    /// Background color for the node card.
    pub bg_color: String,
    /// Name text color.
    pub name_color: String,
    /// State text color.
    pub state_color: String,
}

// ---------------------------------------------------------------------------
// ReplayTheaterScreen
// ---------------------------------------------------------------------------

/// Top-level data model for the Replay Theater screen layout.
///
/// Contains all the placeholder data needed to render the six
/// panels of the Replay Theater:
///
/// 1. **Top bar** -- run id, workflow name
/// 2. **Left-top** -- runtime graph with node overlay cards
/// 3. **Right-top** -- selected event panel (seq, timestamp, shard, step, kind, evidence, digest)
/// 4. **Left-bottom** -- journal timeline strip with scrubbing cursor
/// 5. **Right-middle** -- slot diff table (slot_id, before, after, taint_before, taint_after)
/// 6. **Right-bottom** -- recovery decision panel (strategy, max_attempts, idempotency)
/// 7. **Bottom-left** -- playback controls + live/frozen toggle
pub struct ReplayTheaterScreen {
    // -- Top bar --
    /// Displayed run id.
    pub run_id: u64,
    /// Displayed workflow name.
    pub workflow_name: String,

    // -- Left-top: runtime graph --
    /// Nodes to render in the graph overlay.
    pub graph_nodes: Vec<GraphNode>,

    // -- Right-top: selected event panel --
    /// Selected event details.
    pub selected_event_panel: SelectedEventPanel,

    // -- Legacy inspector cards (kept for compat) --
    /// Inspector cards (step, ticket, slot diffs).
    pub inspector_cards: Vec<InspectorCard>,

    // -- Left-bottom: journal timeline --
    /// Timeline strip (event markers).
    pub timeline_strip: TimelineStrip,
    /// Pre-built timeline chips for rendering.
    pub timeline_chips: Vec<TimelineChip>,

    // -- Right-middle: slot diff table --
    /// Slot diff table.
    pub slot_diff_table: SlotDiffTable,

    // -- Left-bottom: transport bar --
    /// Transport bar state (buttons, speed, jumps, live toggle).
    pub transport_bar: TransportBar,

    // -- Right-bottom: recovery decision panel --
    /// Recovery decision panel.
    pub recovery_decision_panel: RecoveryDecisionPanel,

    // -- Legacy slot diff panel (kept for existing API compat) --
    /// Slot diff panel (model).
    pub slot_diff_panel: SlotDiffPanel,
}

impl ReplayTheaterScreen {
    /// Create a new screen populated with placeholder data matching the
    /// Phase 1F layout spec.
    #[must_use]
    pub fn new() -> Self {
        let run_id: u64 = 8172;
        let workflow_name = String::from("issue-triage");

        // -- Graph nodes (placeholder) --
        let graph_nodes = vec![
            GraphNode {
                name: String::from("SetConst"),
                action_label: None,
                state_label: String::from("succeeded"),
                bg_color: String::from("#0d1a0d"),
                name_color: String::from(NEON_GREEN),
                state_color: String::from(NEON_GREEN),
            },
            GraphNode {
                name: String::from("Do"),
                action_label: Some(String::from("github.issue.create")),
                state_label: String::from("succeeded"),
                bg_color: String::from("#0d1a0d"),
                name_color: String::from(NEON_ORANGE),
                state_color: String::from(NEON_GREEN),
            },
            GraphNode {
                name: String::from("Choose"),
                action_label: None,
                state_label: String::from("succeeded"),
                bg_color: String::from("#0d1a0d"),
                name_color: String::from("#b14dff"),
                state_color: String::from(NEON_GREEN),
            },
            GraphNode {
                name: String::from("ForEach"),
                action_label: None,
                state_label: String::from("succeeded"),
                bg_color: String::from("#0d1a0d"),
                name_color: String::from("#2d6bff"),
                state_color: String::from(NEON_GREEN),
            },
            GraphNode {
                name: String::from("Do"),
                action_label: Some(String::from("slack.notify")),
                state_label: String::from("succeeded"),
                bg_color: String::from("#1a0d00"),
                name_color: String::from(NEON_ORANGE),
                state_color: String::from(NEON_GREEN),
            },
            GraphNode {
                name: String::from("Finish"),
                action_label: None,
                state_label: String::from("completed"),
                bg_color: String::from("#0d1a0d"),
                name_color: String::from(NEON_GREEN),
                state_color: String::from(NEON_GREEN),
            },
        ];

        // -- Inspector cards --
        let step_card = InspectorCard {
            title: String::from("Step Inspector"),
            title_color: String::from(TEXT_PRIMARY),
            fields: vec![
                InspectorField {
                    key: String::from("Step:"),
                    value: String::from("github.issue.create"),
                    value_color: String::from(TEXT_PRIMARY),
                },
                InspectorField {
                    key: String::from("Kind:"),
                    value: String::from("Do"),
                    value_color: String::from(NEON_ORANGE),
                },
                InspectorField {
                    key: String::from("State:"),
                    value: String::from("Succeeded"),
                    value_color: String::from(NEON_GREEN),
                },
                InspectorField {
                    key: String::from("ActionId:"),
                    value: String::from("17"),
                    value_color: String::from(NEON_CYAN),
                },
            ],
        };

        let ticket_card = InspectorCard {
            title: String::from("Action Ticket"),
            title_color: String::from(TEXT_PRIMARY),
            fields: vec![
                InspectorField {
                    key: String::from("Ticket:"),
                    value: String::from("#42"),
                    value_color: String::from(NEON_CYAN),
                },
                InspectorField {
                    key: String::from("Idempotency:"),
                    value: String::from("verified"),
                    value_color: String::from(NEON_GREEN),
                },
                InspectorField {
                    key: String::from("Side effects:"),
                    value: String::from("none"),
                    value_color: String::from(TEXT_SECONDARY),
                },
                InspectorField {
                    key: String::from("Retries:"),
                    value: String::from("0"),
                    value_color: String::from(TEXT_SECONDARY),
                },
            ],
        };

        let slot_diff_card = InspectorCard {
            title: String::from("Slot Diffs"),
            title_color: String::from(TEXT_PRIMARY),
            fields: vec![
                InspectorField {
                    key: String::from("SlotIdx(0):"),
                    value: String::from("<created> Null"),
                    value_color: String::from(NEON_CYAN),
                },
                InspectorField {
                    key: String::from("SlotIdx(3):"),
                    value: String::from("I64(0) -> I64(42)"),
                    value_color: String::from(NEON_ORANGE),
                },
            ],
        };

        let inspector_cards = vec![step_card, ticket_card, slot_diff_card];

        // -- Transport bar --
        let buttons = vec![
            TransportButton {
                label: String::from("|<"),
                enabled: true,
            },
            TransportButton {
                label: String::from("<"),
                enabled: true,
            },
            TransportButton {
                label: String::from(">"),
                enabled: true,
            },
            TransportButton {
                label: String::from(">|"),
                enabled: true,
            },
        ];

        let jump_chips = vec![
            JumpChip {
                label: String::from("jump: failure"),
                color: String::from(NEON_RED),
            },
            JumpChip {
                label: String::from("action"),
                color: String::from(NEON_ORANGE),
            },
            JumpChip {
                label: String::from("done"),
                color: String::from(NEON_GREEN),
            },
        ];

        let transport_bar = TransportBar {
            buttons,
            speed_label: String::from("1x"),
            jump_chips,
            live_toggle: LiveModeToggle::frozen(),
        };

        // -- Timeline strip (placeholder events) --
        let mut strip = TimelineStrip::new();
        let placeholder_events: &[(&str, u32, Option<u16>)] = &[
            ("RunAccepted", 1, None),
            ("StepStarted", 2, Some(0)),
            ("ActionScheduled", 3, Some(0)),
            ("ActionCompleted", 4, Some(0)),
            ("StepSucceeded", 5, Some(0)),
            ("RunFinished", 6, None),
        ];
        for (kind, seq, step) in placeholder_events {
            strip.extend_from_timeline_events(&[crate::replay::timeline::TimelineEvent {
                seq: *seq,
                event_kind: (*kind).to_owned(),
                step_id: *step,
                timestamp_micros: 0,
                color: TimelineStrip::event_color(kind),
            }]);
        }
        strip.set_cursor(0);
        let timeline_chips = strip.build_chips();

        // -- Selected event panel (placeholder) --
        let selected_event_panel = SelectedEventPanel::placeholder();

        // -- Slot diff table (placeholder) --
        let slot_diff_table = SlotDiffTable::placeholder();

        // -- Recovery decision panel (placeholder) --
        let recovery_decision_panel = RecoveryDecisionPanel::placeholder();

        Self {
            run_id,
            workflow_name,
            graph_nodes,
            inspector_cards,
            slot_diff_panel: SlotDiffPanel::new(),
            transport_bar,
            timeline_strip: strip,
            timeline_chips,
            selected_event_panel,
            slot_diff_table,
            recovery_decision_panel,
        }
    }

    /// Returns the formatted top-bar title string.
    #[must_use]
    pub fn title_text(&self) -> String {
        String::from("vb")
    }

    /// Returns the formatted page title string.
    #[must_use]
    pub fn page_title(&self) -> String {
        String::from("Replay Theater")
    }

    /// Returns the formatted run badge text, e.g. "8172".
    #[must_use]
    pub fn run_id_text(&self) -> String {
        format!("{}", self.run_id)
    }

    /// Returns the formatted workflow badge text.
    #[must_use]
    pub fn workflow_name_text(&self) -> String {
        self.workflow_name.clone()
    }

    /// Returns the graph header label.
    #[must_use]
    pub fn graph_header_text(&self) -> String {
        String::from("WORKFLOW GRAPH")
    }

    /// Returns the graph node count hint text.
    #[must_use]
    pub fn graph_node_count_text(&self) -> String {
        format!("{} nodes", self.graph_nodes.len())
    }

    /// Returns the detail inspector header text.
    #[must_use]
    pub fn inspector_header_text(&self) -> String {
        String::from("DETAIL INSPECTOR")
    }

    /// Returns the transport bar header label.
    #[must_use]
    pub fn transport_header_text(&self) -> String {
        String::from("TRANSPORT")
    }

    /// Returns the timeline header label.
    #[must_use]
    pub fn timeline_header_text(&self) -> String {
        String::from("TIMELINE")
    }

    /// Returns the number of graph nodes.
    #[must_use]
    pub fn graph_node_count(&self) -> usize {
        self.graph_nodes.len()
    }

    /// Returns the number of inspector cards.
    #[must_use]
    pub fn inspector_card_count(&self) -> usize {
        self.inspector_cards.len()
    }

    /// Returns the number of timeline chips.
    #[must_use]
    pub fn chip_count(&self) -> usize {
        self.timeline_chips.len()
    }

    /// Returns a reference to the timeline strip.
    #[must_use]
    pub fn timeline_strip(&self) -> &TimelineStrip {
        &self.timeline_strip
    }

    /// Returns a reference to the slot diff panel.
    #[must_use]
    pub fn slot_diff_panel(&self) -> &SlotDiffPanel {
        &self.slot_diff_panel
    }

    /// Returns a reference to the transport bar.
    #[must_use]
    pub fn transport_bar(&self) -> &TransportBar {
        &self.transport_bar
    }

    /// Returns a reference to the inspector cards.
    #[must_use]
    pub fn inspector_cards(&self) -> &[InspectorCard] {
        &self.inspector_cards
    }

    /// Returns a reference to the graph nodes.
    #[must_use]
    pub fn graph_nodes(&self) -> &[GraphNode] {
        &self.graph_nodes
    }

    /// Returns a reference to the selected event panel.
    #[must_use]
    pub fn selected_event_panel(&self) -> &SelectedEventPanel {
        &self.selected_event_panel
    }

    /// Returns a reference to the slot diff table.
    #[must_use]
    pub fn slot_diff_table(&self) -> &SlotDiffTable {
        &self.slot_diff_table
    }

    /// Returns a reference to the recovery decision panel.
    #[must_use]
    pub fn recovery_decision_panel(&self) -> &RecoveryDecisionPanel {
        &self.recovery_decision_panel
    }

    /// Returns the live/frozen toggle.
    #[must_use]
    pub fn live_toggle(&self) -> &LiveModeToggle {
        &self.transport_bar.live_toggle
    }

    /// Returns the formatted header bar text.
    #[must_use]
    pub fn header_bar_text(&self) -> String {
        String::from("vb -- Replay Theater")
    }

    /// Returns the run id badge text, e.g. "8172".
    #[must_use]
    pub fn run_badge_text(&self) -> String {
        format!("Run: {}", self.run_id)
    }

    /// Returns the workflow badge text, e.g. "Workflow: issue-triage".
    #[must_use]
    pub fn workflow_badge_text(&self) -> String {
        format!("Workflow: {}", self.workflow_name)
    }
}

impl Default for ReplayTheaterScreen {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_screen_has_placeholder_run_id() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.run_id, 8172);
    }

    #[test]
    fn new_screen_has_placeholder_workflow_name() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.workflow_name, "issue-triage");
    }

    #[test]
    fn default_matches_new() {
        let from_new = ReplayTheaterScreen::new();
        let from_default = ReplayTheaterScreen::default();
        assert_eq!(from_new.run_id, from_default.run_id);
        assert_eq!(from_new.workflow_name, from_default.workflow_name);
        assert_eq!(from_new.graph_node_count(), from_default.graph_node_count());
        assert_eq!(
            from_new.inspector_card_count(),
            from_default.inspector_card_count()
        );
    }

    #[test]
    fn title_text_returns_vb() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.title_text(), "vb");
    }

    #[test]
    fn page_title_returns_replay_theater() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.page_title(), "Replay Theater");
    }

    #[test]
    fn run_id_text_formats_u64() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.run_id_text(), "8172");
    }

    #[test]
    fn workflow_name_text_returns_workflow() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.workflow_name_text(), "issue-triage");
    }

    #[test]
    fn graph_header_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.graph_header_text(), "WORKFLOW GRAPH");
    }

    #[test]
    fn graph_node_count_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.graph_node_count_text(), "6 nodes");
    }

    #[test]
    fn inspector_header_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.inspector_header_text(), "DETAIL INSPECTOR");
    }

    #[test]
    fn transport_header_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.transport_header_text(), "TRANSPORT");
    }

    #[test]
    fn timeline_header_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.timeline_header_text(), "TIMELINE");
    }

    // -- Graph nodes --

    #[test]
    fn graph_nodes_has_six_nodes() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.graph_node_count(), 6);
    }

    #[test]
    fn graph_nodes_first_is_setconst() {
        let screen = ReplayTheaterScreen::new();
        let node = screen.graph_nodes().first().expect("first node");
        assert_eq!(node.name, "SetConst");
        assert!(node.action_label.is_none());
        assert_eq!(node.state_label, "succeeded");
    }

    #[test]
    fn graph_nodes_second_is_do_github() {
        let screen = ReplayTheaterScreen::new();
        let node = screen.graph_nodes().get(1).expect("second node");
        assert_eq!(node.name, "Do");
        assert_eq!(node.action_label.as_deref(), Some("github.issue.create"));
    }

    #[test]
    fn graph_nodes_fifth_has_orange_bg() {
        let screen = ReplayTheaterScreen::new();
        let node = screen.graph_nodes().get(4).expect("fifth node");
        assert_eq!(node.name, "Do");
        assert_eq!(node.bg_color, "#1a0d00");
        assert_eq!(node.action_label.as_deref(), Some("slack.notify"));
    }

    #[test]
    fn graph_nodes_last_is_finish() {
        let screen = ReplayTheaterScreen::new();
        let node = screen.graph_nodes().last().expect("last node");
        assert_eq!(node.name, "Finish");
        assert_eq!(node.state_label, "completed");
    }

    // -- Inspector cards --

    #[test]
    fn inspector_cards_has_three_cards() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.inspector_card_count(), 3);
    }

    #[test]
    fn step_card_has_four_fields() {
        let screen = ReplayTheaterScreen::new();
        let card = screen.inspector_cards().first().expect("step card");
        assert_eq!(card.title, "Step Inspector");
        assert_eq!(card.fields.len(), 4);
        assert_eq!(card.fields.first().map(|f| f.key.as_str()), Some("Step:"));
        assert_eq!(
            card.fields.first().map(|f| f.value.as_str()),
            Some("github.issue.create")
        );
    }

    #[test]
    fn ticket_card_has_ticket_42() {
        let screen = ReplayTheaterScreen::new();
        let card = screen.inspector_cards().get(1).expect("ticket card");
        assert_eq!(card.title, "Action Ticket");
        let first = card.fields.first().expect("first field");
        assert_eq!(first.value, "#42");
        assert_eq!(first.value_color, NEON_CYAN);
    }

    #[test]
    fn slot_diff_card_has_two_entries() {
        let screen = ReplayTheaterScreen::new();
        let card = screen.inspector_cards().get(2).expect("slot diff card");
        assert_eq!(card.title, "Slot Diffs");
        assert_eq!(card.fields.len(), 2);
    }

    // -- Transport bar --

    #[test]
    fn transport_bar_has_four_buttons() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.transport_bar().buttons.len(), 4);
    }

    #[test]
    fn transport_bar_buttons_labels() {
        let screen = ReplayTheaterScreen::new();
        let labels: Vec<&str> = screen
            .transport_bar()
            .buttons
            .iter()
            .map(|b| b.label.as_str())
            .collect();
        assert_eq!(labels, vec!["|<", "<", ">", ">|"]);
    }

    #[test]
    fn transport_bar_speed_is_1x() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.transport_bar().speed_label, "1x");
    }

    #[test]
    fn transport_bar_has_three_jump_chips() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.transport_bar().jump_chips.len(), 3);
    }

    #[test]
    fn jump_chip_failure_is_red() {
        let screen = ReplayTheaterScreen::new();
        let chip = screen
            .transport_bar()
            .jump_chips
            .first()
            .expect("failure chip");
        assert_eq!(chip.label, "jump: failure");
        assert_eq!(chip.color, NEON_RED);
    }

    #[test]
    fn jump_chip_action_is_orange() {
        let screen = ReplayTheaterScreen::new();
        let chip = screen
            .transport_bar()
            .jump_chips
            .get(1)
            .expect("action chip");
        assert_eq!(chip.label, "action");
        assert_eq!(chip.color, NEON_ORANGE);
    }

    #[test]
    fn jump_chip_done_is_green() {
        let screen = ReplayTheaterScreen::new();
        let chip = screen.transport_bar().jump_chips.get(2).expect("done chip");
        assert_eq!(chip.label, "done");
        assert_eq!(chip.color, NEON_GREEN);
    }

    // -- Timeline --

    #[test]
    fn timeline_has_six_chips() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.chip_count(), 6);
    }

    #[test]
    fn timeline_first_chip_is_run_accepted() {
        let screen = ReplayTheaterScreen::new();
        let chip = screen.timeline_chips.first().expect("first chip");
        assert_eq!(chip.label, "RunAccepted");
        assert_eq!(chip.seq, 1);
        assert!(chip.is_cursor);
    }

    #[test]
    fn timeline_second_chip_is_step_started() {
        let screen = ReplayTheaterScreen::new();
        let chip = screen.timeline_chips.get(1).expect("second chip");
        assert_eq!(chip.label, "StepStarted");
        assert_eq!(chip.step, Some(0));
        assert!(!chip.is_cursor);
    }

    #[test]
    fn timeline_last_chip_is_run_finished() {
        let screen = ReplayTheaterScreen::new();
        let chip = screen.timeline_chips.last().expect("last chip");
        assert_eq!(chip.label, "RunFinished");
        assert_eq!(chip.seq, 6);
    }

    #[test]
    fn only_first_chip_is_cursor() {
        let screen = ReplayTheaterScreen::new();
        let cursor_count = screen.timeline_chips.iter().filter(|c| c.is_cursor).count();
        assert_eq!(cursor_count, 1);
    }

    // -- Slot diff panel --

    #[test]
    fn slot_diff_panel_is_empty_for_placeholder() {
        let screen = ReplayTheaterScreen::new();
        assert!(!screen.slot_diff_panel().has_changes());
        assert!(screen.slot_diff_panel().entries().is_empty());
    }

    // -- Color constants --

    #[test]
    fn color_constants_match_spec() {
        assert_eq!(PANEL_BG, "#12121f");
        assert_eq!(CARD_BG, "#16162a");
        assert_eq!(BORDER, "#2a2a4a");
        assert_eq!(TEXT_PRIMARY, "#e8e8ff");
        assert_eq!(TEXT_SECONDARY, "#8888aa");
        assert_eq!(NEON_CYAN, "#00f5ff");
        assert_eq!(NEON_GREEN, "#39ff14");
    }

    // -- Graph node color assignments --

    #[test]
    fn graph_nodes_all_have_nonempty_colors() {
        let screen = ReplayTheaterScreen::new();
        for (i, node) in screen.graph_nodes().iter().enumerate() {
            assert!(!node.bg_color.is_empty(), "empty bg_color at index {i}");
            assert!(!node.name_color.is_empty(), "empty name_color at index {i}");
            assert!(
                !node.state_color.is_empty(),
                "empty state_color at index {i}"
            );
        }
    }

    // -- Inspector card color assignments --

    #[test]
    fn inspector_card_fields_have_nonempty_colors() {
        let screen = ReplayTheaterScreen::new();
        for (i, card) in screen.inspector_cards().iter().enumerate() {
            assert!(
                !card.title_color.is_empty(),
                "empty title_color for card {i}"
            );
            for (j, field) in card.fields.iter().enumerate() {
                assert!(
                    !field.value_color.is_empty(),
                    "empty value_color for card {i}, field {j}"
                );
            }
        }
    }

    // -- Transport buttons all enabled --

    #[test]
    fn all_transport_buttons_enabled() {
        let screen = ReplayTheaterScreen::new();
        for (i, btn) in screen.transport_bar().buttons.iter().enumerate() {
            assert!(btn.enabled, "button {i} should be enabled");
        }
    }

    // -- Timeline strip accessor --

    #[test]
    fn timeline_strip_events_count_matches_chips() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.timeline_strip().events().len(), screen.chip_count());
    }

    // -- InspectorField clone --

    #[test]
    fn inspector_field_clone_roundtrip() {
        let field = InspectorField {
            key: String::from("Test:"),
            value: String::from("value"),
            value_color: String::from("#ffffff"),
        };
        let cloned = field.clone();
        assert_eq!(cloned.key, field.key);
        assert_eq!(cloned.value, field.value);
        assert_eq!(cloned.value_color, field.value_color);
    }

    // -- InspectorCard clone --

    #[test]
    fn inspector_card_clone_roundtrip() {
        let card = InspectorCard {
            title: String::from("Test Card"),
            title_color: String::from("#ffffff"),
            fields: vec![InspectorField {
                key: String::from("K:"),
                value: String::from("V"),
                value_color: String::from("#000000"),
            }],
        };
        let cloned = card.clone();
        assert_eq!(cloned.title, card.title);
        assert_eq!(cloned.fields.len(), 1);
    }

    // -- GraphNode clone --

    #[test]
    fn graph_node_clone_roundtrip() {
        let node = GraphNode {
            name: String::from("Test"),
            action_label: Some(String::from("action")),
            state_label: String::from("running"),
            bg_color: String::from("#000000"),
            name_color: String::from("#ffffff"),
            state_color: String::from("#00ff00"),
        };
        let cloned = node.clone();
        assert_eq!(cloned.name, node.name);
        assert_eq!(cloned.action_label, node.action_label);
    }

    // -- TransportButton clone --

    #[test]
    fn transport_button_clone_roundtrip() {
        let btn = TransportButton {
            label: String::from(">"),
            enabled: true,
        };
        let cloned = btn.clone();
        assert_eq!(cloned.label, btn.label);
        assert_eq!(cloned.enabled, btn.enabled);
    }

    // -- JumpChip clone --

    #[test]
    fn jump_chip_clone_roundtrip() {
        let chip = JumpChip {
            label: String::from("test"),
            color: String::from("#ff0000"),
        };
        let cloned = chip.clone();
        assert_eq!(cloned.label, chip.label);
        assert_eq!(cloned.color, chip.color);
    }

    // -- TransportBar clone --

    #[test]
    fn transport_bar_clone_roundtrip() {
        let screen = ReplayTheaterScreen::new();
        let cloned = screen.transport_bar.clone();
        assert_eq!(cloned.buttons.len(), screen.transport_bar.buttons.len());
        assert_eq!(cloned.speed_label, screen.transport_bar.speed_label);
        assert_eq!(
            cloned.jump_chips.len(),
            screen.transport_bar.jump_chips.len()
        );
        assert_eq!(
            cloned.live_toggle.label,
            screen.transport_bar.live_toggle.label
        );
    }

    // -- SelectedEventPanel --

    #[test]
    fn selected_event_panel_placeholder_has_all_fields() {
        let panel = SelectedEventPanel::placeholder();
        assert!(!panel.seq.is_empty());
        assert!(!panel.timestamp_micros.is_empty());
        assert!(!panel.shard_id.is_empty());
        assert!(!panel.step.is_empty());
        assert!(!panel.event_kind.is_empty());
        assert!(!panel.evidence_id.is_empty());
        assert!(!panel.digest_summary.is_empty());
        assert_eq!(panel.event_kind, "ActionFailed");
        assert_eq!(panel.event_kind_color, NEON_RED);
    }

    #[test]
    fn selected_event_panel_clone_roundtrip() {
        let panel = SelectedEventPanel::placeholder();
        let cloned = panel.clone();
        assert_eq!(cloned.seq, panel.seq);
        assert_eq!(cloned.event_kind, panel.event_kind);
        assert_eq!(cloned.event_kind_color, panel.event_kind_color);
    }

    #[test]
    fn screen_selected_event_panel_is_accessible() {
        let screen = ReplayTheaterScreen::new();
        let panel = screen.selected_event_panel();
        assert_eq!(panel.event_kind, "ActionFailed");
    }

    // -- SlotDiffTable --

    #[test]
    fn slot_diff_table_placeholder_has_headers_and_rows() {
        let table = SlotDiffTable::placeholder();
        assert_eq!(table.headers.len(), 5);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.headers[0], "slot");
        assert_eq!(table.headers[1], "before");
        assert_eq!(table.headers[2], "after");
        assert_eq!(table.headers[3], "taint before");
        assert_eq!(table.headers[4], "taint after");
    }

    #[test]
    fn slot_diff_table_placeholder_row_fields_are_nonempty() {
        let table = SlotDiffTable::placeholder();
        for (i, row) in table.rows.iter().enumerate() {
            assert!(!row.slot_id.is_empty(), "row {i} slot_id empty");
            assert!(!row.before.is_empty(), "row {i} before empty");
            assert!(!row.after.is_empty(), "row {i} after empty");
            assert!(!row.taint_before.is_empty(), "row {i} taint_before empty");
            assert!(!row.taint_after.is_empty(), "row {i} taint_after empty");
        }
    }

    #[test]
    fn slot_diff_table_clone_roundtrip() {
        let table = SlotDiffTable::placeholder();
        let cloned = table.clone();
        assert_eq!(cloned.headers.len(), table.headers.len());
        assert_eq!(cloned.rows.len(), table.rows.len());
    }

    #[test]
    fn screen_slot_diff_table_is_accessible() {
        let screen = ReplayTheaterScreen::new();
        let table = screen.slot_diff_table();
        assert_eq!(table.rows.len(), 2);
    }

    // -- RecoveryDecisionPanel --

    #[test]
    fn recovery_decision_panel_placeholder_fields() {
        let panel = RecoveryDecisionPanel::placeholder();
        assert_eq!(panel.strategy, "Retry");
        assert_eq!(panel.strategy_color, NEON_ORANGE);
        assert_eq!(panel.max_attempts, "3");
        assert_eq!(panel.idempotency_required, "true");
        assert!(!panel.apply_action.is_empty());
        assert_eq!(panel.apply_action_color, NEON_CYAN);
    }

    #[test]
    fn recovery_decision_panel_clone_roundtrip() {
        let panel = RecoveryDecisionPanel::placeholder();
        let cloned = panel.clone();
        assert_eq!(cloned.strategy, panel.strategy);
        assert_eq!(cloned.max_attempts, panel.max_attempts);
        assert_eq!(cloned.idempotency_required, panel.idempotency_required);
    }

    #[test]
    fn screen_recovery_decision_panel_is_accessible() {
        let screen = ReplayTheaterScreen::new();
        let panel = screen.recovery_decision_panel();
        assert_eq!(panel.strategy, "Retry");
        assert_eq!(panel.max_attempts, "3");
    }

    // -- LiveModeToggle --

    #[test]
    fn live_mode_toggle_live() {
        let toggle = LiveModeToggle::live();
        assert_eq!(toggle.label, "live");
        assert!(toggle.is_live);
        assert_eq!(toggle.color, NEON_GREEN);
    }

    #[test]
    fn live_mode_toggle_frozen() {
        let toggle = LiveModeToggle::frozen();
        assert_eq!(toggle.label, "frozen");
        assert!(!toggle.is_live);
        assert_eq!(toggle.color, TEXT_DIM);
    }

    #[test]
    fn live_mode_toggle_clone_roundtrip() {
        let toggle = LiveModeToggle::live();
        let cloned = toggle.clone();
        assert_eq!(cloned.label, toggle.label);
        assert_eq!(cloned.is_live, toggle.is_live);
        assert_eq!(cloned.color, toggle.color);
    }

    #[test]
    fn transport_bar_includes_live_toggle() {
        let screen = ReplayTheaterScreen::new();
        let toggle = screen.live_toggle();
        assert_eq!(toggle.label, "frozen");
        assert!(!toggle.is_live);
    }

    // -- Screen-level integration --

    #[test]
    fn screen_new_builds_all_six_panels() {
        let screen = ReplayTheaterScreen::new();
        // Graph nodes (left-top)
        assert_eq!(screen.graph_node_count(), 6);
        // Selected event panel (right-top)
        assert_eq!(screen.selected_event_panel().event_kind, "ActionFailed");
        // Timeline strip (left-bottom)
        assert_eq!(screen.timeline_strip().events().len(), 6);
        // Slot diff table (right-middle)
        assert_eq!(screen.slot_diff_table().rows.len(), 2);
        // Transport bar (left-bottom)
        assert_eq!(screen.transport_bar().buttons.len(), 4);
        // Recovery decision panel (right-bottom)
        assert_eq!(screen.recovery_decision_panel().strategy, "Retry");
    }

    #[test]
    fn screen_header_bar_text_formats_correctly() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.header_bar_text(), "vb -- Replay Theater");
    }

    #[test]
    fn screen_run_badge_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.run_badge_text(), "Run: 8172");
    }

    #[test]
    fn screen_workflow_badge_text() {
        let screen = ReplayTheaterScreen::new();
        assert_eq!(screen.workflow_badge_text(), "Workflow: issue-triage");
    }

    // -- SlotDiffRow clone --

    #[test]
    fn slot_diff_row_clone_roundtrip() {
        let row = SlotDiffRow {
            slot_id: String::from("SlotIdx(1)"),
            before: String::from("I64(0)"),
            after: String::from("I64(99)"),
            taint_before: String::from("Clean"),
            taint_after: String::from("Secret"),
        };
        let cloned = row.clone();
        assert_eq!(cloned.slot_id, row.slot_id);
        assert_eq!(cloned.before, row.before);
        assert_eq!(cloned.after, row.after);
        assert_eq!(cloned.taint_before, row.taint_before);
        assert_eq!(cloned.taint_after, row.taint_after);
    }

    // -- NEON_PURPLE and NEON_TEAL constants --

    #[test]
    fn neon_purple_is_b14dff() {
        assert_eq!(NEON_PURPLE, "#b14dff");
    }

    #[test]
    fn neon_teal_is_00e5c7() {
        assert_eq!(NEON_TEAL, "#00e5c7");
    }

    // -- TransportBar includes live_toggle in clone --

    #[test]
    fn transport_bar_clone_includes_live_toggle() {
        let screen = ReplayTheaterScreen::new();
        let cloned = screen.transport_bar.clone();
        assert_eq!(cloned.live_toggle.label, "frozen");
    }
}
