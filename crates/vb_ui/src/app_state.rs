#![forbid(unsafe_code)]
//! Screen navigation and global app state for the Makepad UI.
//!
//! Manages the eight canonical screens from the 11:51 Figma bundle while
//! retaining the IPC-backed data payloads used by the renderer.

use crate::replay::timeline::TimelineStrip;
use crate::replay::transport::TransportState;
use crate::system::screen::SystemScreen;

/// The eight canonical screens of the white Makepad UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Screen {
    ExecutionOverview,
    WorkflowGraphAuthoring,
    ExecutionDetailsGraph,
    VerificationCertificate,
    ReplayTheater,
    IncidentFailureConsole,
    ActionRegistry,
    StorageDoctorAiContext,
}

impl Screen {
    pub const fn splash_name(self) -> &'static str {
        match self {
            Self::ExecutionOverview => "ExecutionOverview",
            Self::WorkflowGraphAuthoring => "WorkflowGraphAuthoring",
            Self::ExecutionDetailsGraph => "ExecutionDetailsGraph",
            Self::VerificationCertificate => "VerificationCertificate",
            Self::ReplayTheater => "ReplayTheater",
            Self::IncidentFailureConsole => "IncidentFailureConsole",
            Self::ActionRegistry => "ActionRegistry",
            Self::StorageDoctorAiContext => "StorageDoctorAiContext",
        }
    }

    pub const fn nav_label(self) -> &'static str {
        match self {
            Self::ExecutionOverview => "Overview",
            Self::WorkflowGraphAuthoring => "Workflow Graph",
            Self::ExecutionDetailsGraph => "Executions",
            Self::VerificationCertificate => "Verification",
            Self::ReplayTheater => "Replay",
            Self::IncidentFailureConsole => "Incidents",
            Self::ActionRegistry => "Actions",
            Self::StorageDoctorAiContext => "Storage / AI",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::ExecutionOverview => "Execution Observatory Overview",
            Self::WorkflowGraphAuthoring => "Workflow Graph Authoring",
            Self::ExecutionDetailsGraph => "Execution Details Graph View",
            Self::VerificationCertificate => "Verification Certificate View",
            Self::ReplayTheater => "Replay Theater",
            Self::IncidentFailureConsole => "Incident Failure Console",
            Self::ActionRegistry => "Action Registry / Contract Inspector",
            Self::StorageDoctorAiContext => "Storage / Journal Doctor + AI Context",
        }
    }

    pub const fn subtitle(self) -> &'static str {
        match self {
            Self::ExecutionOverview => "Black-box recorder · local-first workflow health",
            Self::WorkflowGraphAuthoring => "YAML source projected as verified numeric IR",
            Self::ExecutionDetailsGraph => {
                "Single run inspection with graph, events, and step details"
            }
            Self::VerificationCertificate => "Accepted artifact proof before runtime admission",
            Self::ReplayTheater => "A cinematic flight recorder for durable workflow executions",
            Self::IncidentFailureConsole => "Minimal red, maximum evidence and recovery clarity",
            Self::ActionRegistry => {
                "Numeric ActionId contracts, side-effect policy, capabilities, and schemas"
            }
            Self::StorageDoctorAiContext => {
                "Fjall keyspaces, Postcard envelopes, replay health, and AI-safe packet"
            }
        }
    }

    pub const fn status_chips(self) -> (&'static str, &'static str) {
        match self {
            Self::ExecutionOverview => ("makepad · prod", "Healthy"),
            Self::WorkflowGraphAuthoring => ("Verified", "Draft"),
            Self::ExecutionDetailsGraph => ("Running", "Replay safe"),
            Self::VerificationCertificate => ("Verified", "Strict"),
            Self::ReplayTheater => ("Replay safe", "Journaled"),
            Self::IncidentFailureConsole => ("Needs operator", "Replay safe"),
            Self::ActionRegistry => ("Contracts valid", "ABI stable"),
            Self::StorageDoctorAiContext => ("DB healthy", "Replay lab ready"),
        }
    }

    /// RGBA semantic accent color for the active nav row and screen highlights.
    pub const fn nav_color(self) -> [f32; 4] {
        match self {
            Self::ExecutionOverview => [0.145, 0.388, 0.922, 1.0],
            Self::WorkflowGraphAuthoring => [0.431, 0.321, 0.898, 1.0],
            Self::ExecutionDetailsGraph => [0.145, 0.388, 0.922, 1.0],
            Self::VerificationCertificate => [0.086, 0.651, 0.416, 1.0],
            Self::ReplayTheater => [0.169, 0.424, 1.0, 1.0],
            Self::IncidentFailureConsole => [0.898, 0.282, 0.302, 1.0],
            Self::ActionRegistry => [0.773, 0.357, 0.083, 1.0],
            Self::StorageDoctorAiContext => [0.431, 0.321, 0.898, 1.0],
        }
    }
}

/// Global app state shared across screens.
pub struct AppState {
    pub current_screen: Screen,
    pub connected: bool,
    pub selected_run_id: Option<u64>,
    pub selected_workflow_name: Option<String>,
    pub selected_workflow_digest: Option<[u8; 32]>,
    pub replay: ReplayData,
    pub system: SystemData,
    pub incident: IncidentData,
    pub verification: VerificationData,
    pub workflow: WorkflowData,
    /// Rich system screen model (topology, metrics, alerts, ticker, queues).
    /// Used by the renderer to produce `SystemFrame` data for the Makepad UI.
    pub system_screen: SystemScreen,
    /// Last IPC wiring error, if any. Surfaces connection failures and IPC
    /// errors in the System Overview screen so they are not silently swallowed.
    pub last_ipc_error: Option<String>,
    /// Whether to show the shortcuts help overlay.
    pub show_shortcuts: bool,
}

/// Replay Theater screen data.
pub struct ReplayData {
    pub playback_position: u32,
    pub total_events: u32,
    pub transport_state: TransportState,
    pub playback_speed: f64,
    pub current_step: Option<u16>,
    pub step_state: Option<String>,
    /// Timeline strip built from journal events. Holds event markers with
    /// labels, colors, and sequence info for chip rendering.
    pub timeline_strip: TimelineStrip,
}

/// System Overview screen data.
pub struct SystemData {
    pub shard_count: u32,
    pub total_active_runs: u32,
    pub total_queue_depth: u32,
    pub overall_health: HealthLevel,
}

/// Overall system health indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Critical,
}

/// Incident Console screen data.
pub struct IncidentData {
    pub active_incidents: u32,
    pub critical_count: u32,
    pub warning_count: u32,
    pub selected_incident: Option<u64>,
}

/// Per-certificate-card status for a verification panel.
#[derive(Debug, Clone)]
pub struct CertCardStatus {
    /// Badge text: "PASS", "WARN", or "FAIL".
    pub badge_text: String,
    /// Detail line 1 value text.
    pub field1: String,
    /// Detail line 2 value text.
    pub field2: String,
    /// Detail line 3 value text.
    pub field3: String,
    /// Detail line 4 value text.
    pub field4: String,
}

impl CertCardStatus {
    pub fn empty() -> Self {
        Self {
            badge_text: String::from("--"),
            field1: String::from("--"),
            field2: String::from("--"),
            field3: String::from("--"),
            field4: String::from("--"),
        }
    }

    /// Returns the semantic light-theme color hex string for the badge.
    pub fn badge_color(&self) -> &'static str {
        match self.badge_text.as_str() {
            "PASS" => "#16a66a",
            "WARN" => "#f59e0b",
            "FAIL" => "#e5484d",
            _ => "#98a2b3",
        }
    }

    /// Returns the semantic light-theme color hex string for field values.
    pub fn field_color(&self) -> &'static str {
        match self.badge_text.as_str() {
            "PASS" => "#16a66a",
            "WARN" => "#f59e0b",
            "FAIL" => "#e5484d",
            _ => "#98a2b3",
        }
    }
}

/// Verification screen data.
pub struct VerificationData {
    pub total_checks: u32,
    pub pass_count: u32,
    pub warn_count: u32,
    pub fail_count: u32,
    /// True when all checks pass (no warnings or failures).
    pub all_clean: bool,
    /// Per-certificate card detail status for each verification panel.
    pub cert_structure: CertCardStatus,
    pub cert_bounded: CertCardStatus,
    pub cert_resources: CertCardStatus,
    pub cert_taint: CertCardStatus,
    pub cert_action: CertCardStatus,
    pub cert_durability: CertCardStatus,
}

/// Workflow Graph screen data.
pub struct WorkflowData {
    pub name: Option<String>,
    pub node_count: u32,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::ExecutionOverview,
            connected: false,
            selected_run_id: None,
            selected_workflow_name: None,
            selected_workflow_digest: None,
            replay: ReplayData::new(),
            system: SystemData::new(),
            incident: IncidentData::new(),
            verification: VerificationData::new(),
            workflow: WorkflowData::new(),
            system_screen: SystemScreen::new(),
            last_ipc_error: None,
            show_shortcuts: false,
        }
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
    }

    pub fn current_screen(&self) -> Screen {
        self.current_screen
    }

    pub fn screen_title(&self) -> &'static str {
        self.current_screen.title()
    }

    pub fn screen_subtitle(&self) -> &'static str {
        self.current_screen.subtitle()
    }

    pub fn screen_status_chips(&self) -> (&'static str, &'static str) {
        self.current_screen.status_chips()
    }

    /// Returns an RGBA color (each channel 0.0-1.0) used for the active nav row accent.
    pub fn screen_nav_color(&self) -> [f32; 4] {
        self.current_screen.nav_color()
    }

    /// Re-derive the lightweight `SystemData` summary fields from the rich
    /// `SystemScreen` model. Call this after updating `system_screen` with
    /// fresh metrics so that the summary struct stays consistent.
    pub fn sync_system_from_screen(&mut self) {
        let metrics = self.system_screen.metrics();
        self.system.shard_count = u32::try_from(metrics.shards.len()).unwrap_or(u32::MAX);
        self.system.total_active_runs = metrics.total_active_runs;
        self.system.total_queue_depth = metrics
            .total_ready_queue_depth
            .saturating_add(metrics.total_action_queue_depth);
        self.system.overall_health = match metrics.overall_health {
            crate::system::metrics::HealthStatus::Healthy => HealthLevel::Healthy,
            crate::system::metrics::HealthStatus::Degraded => HealthLevel::Degraded,
            crate::system::metrics::HealthStatus::Critical => HealthLevel::Critical,
        };
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplayData {
    fn new() -> Self {
        Self {
            playback_position: 0,
            total_events: 0,
            transport_state: TransportState::Idle,
            playback_speed: 1.0,
            current_step: None,
            step_state: None,
            timeline_strip: TimelineStrip::new(),
        }
    }

    /// Returns "N events" for the event count label.
    pub fn event_count_text(&self) -> String {
        format!("{} events", self.total_events)
    }

    /// Returns the speed as a human string (e.g. "1.0x").
    pub fn speed_text(&self) -> String {
        if self.playback_speed < 10.0 {
            format!("{:.1}x", self.playback_speed)
        } else {
            format!("{:.0}x", self.playback_speed)
        }
    }

    /// Returns the run ID display string or "--".
    pub fn run_id_text(run_id: Option<u64>) -> String {
        match run_id {
            Some(id) => id.to_string(),
            None => String::from("--"),
        }
    }
}

impl SystemData {
    fn new() -> Self {
        Self {
            shard_count: 0,
            total_active_runs: 0,
            total_queue_depth: 0,
            overall_health: HealthLevel::Healthy,
        }
    }

    /// Returns "N active runs across M shards" for the lanes hint.
    pub fn lanes_hint_text(&self) -> String {
        format!(
            "{} active runs across {} shards",
            self.total_active_runs, self.shard_count
        )
    }

    /// Returns health as a display string.
    pub fn health_text(&self) -> &'static str {
        match self.overall_health {
            HealthLevel::Healthy => "HEALTHY",
            HealthLevel::Degraded => "DEGRADED",
            HealthLevel::Critical => "CRITICAL",
        }
    }
}

impl IncidentData {
    fn new() -> Self {
        Self {
            active_incidents: 0,
            critical_count: 0,
            warning_count: 0,
            selected_incident: None,
        }
    }
}

impl VerificationData {
    fn new() -> Self {
        Self {
            total_checks: 0,
            pass_count: 0,
            warn_count: 0,
            fail_count: 0,
            all_clean: true,
            cert_structure: CertCardStatus::empty(),
            cert_bounded: CertCardStatus::empty(),
            cert_resources: CertCardStatus::empty(),
            cert_taint: CertCardStatus::empty(),
            cert_action: CertCardStatus::empty(),
            cert_durability: CertCardStatus::empty(),
        }
    }

    /// Populate all six cert card panels from a slice of `CertificateWire`
    /// results returned over IPC.
    ///
    /// Gate-to-panel mapping:
    /// - Structure: gate_09, gate_10
    /// - Bounded:   gate_07
    /// - Resources:  gate_08
    /// - Taint:      gate_13
    /// - Action:     gate_12, gate_14
    /// - Durability: gate_11, gate_15
    pub fn populate_cert_cards(&mut self, certs: &[vb_ipc::CertificateWire]) {
        let total_count = u32::try_from(certs.len()).unwrap_or(u32::MAX);

        // Helper: given a list of gate name prefixes, build a CertCardStatus.
        fn build_card(certs: &[vb_ipc::CertificateWire], prefixes: &[&str]) -> CertCardStatus {
            let mut pass_count: u32 = 0;
            let mut fail_count: u32 = 0;
            let mut total_in_panel: u32 = 0;

            for cert in certs {
                let matches = prefixes.iter().any(|prefix| cert.kind.starts_with(prefix));
                if matches {
                    total_in_panel = total_in_panel.saturating_add(1);
                    if cert.status == "Pass" {
                        pass_count = pass_count.saturating_add(1);
                    } else {
                        fail_count = fail_count.saturating_add(1);
                    }
                }
            }

            let badge_text = if fail_count == 0 && pass_count > 0 {
                "PASS"
            } else if fail_count > 0 {
                "FAIL"
            } else {
                "--"
            };

            CertCardStatus {
                badge_text: String::from(badge_text),
                field1: format!("total: {total_in_panel}"),
                field2: format!("pass: {pass_count}"),
                field3: format!("fail: {fail_count}"),
                field4: String::from("--"),
            }
        }

        self.cert_structure = build_card(certs, &["gate_09", "gate_10"]);
        self.cert_bounded = build_card(certs, &["gate_07"]);
        self.cert_resources = build_card(certs, &["gate_08"]);
        self.cert_taint = build_card(certs, &["gate_13"]);
        self.cert_action = build_card(certs, &["gate_12", "gate_14"]);
        self.cert_durability = build_card(certs, &["gate_11", "gate_15"]);

        // Derive aggregate counters from the full certificate list.
        let mut pass: u32 = 0;
        let mut fail: u32 = 0;
        for cert in certs {
            if cert.status == "Pass" {
                pass = pass.saturating_add(1);
            } else {
                fail = fail.saturating_add(1);
            }
        }
        self.total_checks = total_count;
        self.pass_count = pass;
        self.fail_count = fail;
        self.warn_count = 0;
        self.all_clean = fail == 0 && (pass > 0 || total_count == 0);
    }

    /// Returns a human-readable summary string for the verification badge.
    pub fn status_badge_text(&self) -> String {
        if self.all_clean {
            String::from("PASS (all panels clean)")
        } else if self.fail_count > 0 {
            let total = self.total_checks;
            let clean = total
                .saturating_sub(self.fail_count)
                .saturating_sub(self.warn_count);
            format!("FAIL ({clean}/{total} panels clean)")
        } else {
            let total = self.total_checks;
            let clean = total.saturating_sub(self.warn_count);
            format!("PASS ({clean}/{total} panels clean)")
        }
    }

    /// Returns the worst risk level as a human-readable string.
    pub fn worst_risk_text(&self) -> &'static str {
        if self.fail_count > 0 {
            "HIGH RISK"
        } else if self.warn_count > 0 {
            "WARNING"
        } else {
            "CLEAN"
        }
    }
}

impl WorkflowData {
    fn new() -> Self {
        Self {
            name: None,
            node_count: 0,
        }
    }

    /// Returns the workflow name or "unknown".
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("unknown")
    }

    /// Returns "N nodes" string.
    pub fn node_hint(&self) -> String {
        format!("{} nodes", self.node_count)
    }
}

impl Default for VerificationData {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WorkflowData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // AppState::new() default values
    // -----------------------------------------------------------------------

    #[test]
    fn app_state_new_defaults_to_execution_overview_screen() {
        let state = AppState::new();
        assert_eq!(state.current_screen, Screen::ExecutionOverview);
    }

    #[test]
    fn app_state_new_defaults_to_disconnected() {
        let state = AppState::new();
        assert!(!state.connected);
    }

    #[test]
    fn app_state_new_has_no_selected_run_id() {
        let state = AppState::new();
        assert!(state.selected_run_id.is_none());
    }

    #[test]
    fn app_state_new_has_no_selected_workflow_name() {
        let state = AppState::new();
        assert!(state.selected_workflow_name.is_none());
    }

    #[test]
    fn app_state_new_has_no_selected_workflow_digest() {
        let state = AppState::new();
        assert!(state.selected_workflow_digest.is_none());
    }

    #[test]
    fn app_state_default_matches_new() {
        let from_new = AppState::new();
        let from_default = AppState::default();
        assert_eq!(from_new.current_screen, from_default.current_screen);
        assert_eq!(from_new.connected, from_default.connected);
        assert_eq!(from_new.selected_run_id, from_default.selected_run_id);
    }

    // -----------------------------------------------------------------------
    // Screen enum variants
    // -----------------------------------------------------------------------

    #[test]
    fn screen_variants_are_distinct() {
        let variants = [
            Screen::ExecutionOverview,
            Screen::WorkflowGraphAuthoring,
            Screen::ExecutionDetailsGraph,
            Screen::VerificationCertificate,
            Screen::ReplayTheater,
            Screen::IncidentFailureConsole,
            Screen::ActionRegistry,
            Screen::StorageDoctorAiContext,
        ];
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                assert_eq!(i == j, a == b, "Screen variant mismatch at indices {i},{j}");
            }
        }
    }

    // -----------------------------------------------------------------------
    // switch_screen / current_screen / screen_title
    // -----------------------------------------------------------------------

    #[test]
    fn switch_screen_updates_current_screen() {
        let mut state = AppState::new();
        assert_eq!(state.current_screen(), Screen::ExecutionOverview);

        state.switch_screen(Screen::WorkflowGraphAuthoring);
        assert_eq!(state.current_screen(), Screen::WorkflowGraphAuthoring);

        state.switch_screen(Screen::ExecutionDetailsGraph);
        assert_eq!(state.current_screen(), Screen::ExecutionDetailsGraph);

        state.switch_screen(Screen::VerificationCertificate);
        assert_eq!(state.current_screen(), Screen::VerificationCertificate);

        state.switch_screen(Screen::ReplayTheater);
        assert_eq!(state.current_screen(), Screen::ReplayTheater);

        state.switch_screen(Screen::IncidentFailureConsole);
        assert_eq!(state.current_screen(), Screen::IncidentFailureConsole);

        state.switch_screen(Screen::ActionRegistry);
        assert_eq!(state.current_screen(), Screen::ActionRegistry);

        state.switch_screen(Screen::StorageDoctorAiContext);
        assert_eq!(state.current_screen(), Screen::StorageDoctorAiContext);
    }

    #[test]
    fn screen_title_returns_correct_labels() {
        let mut state = AppState::new();

        state.switch_screen(Screen::ExecutionOverview);
        assert_eq!(state.screen_title(), "Execution Observatory Overview");

        state.switch_screen(Screen::WorkflowGraphAuthoring);
        assert_eq!(state.screen_title(), "Workflow Graph Authoring");

        state.switch_screen(Screen::ExecutionDetailsGraph);
        assert_eq!(state.screen_title(), "Execution Details Graph View");

        state.switch_screen(Screen::VerificationCertificate);
        assert_eq!(state.screen_title(), "Verification Certificate View");

        state.switch_screen(Screen::ReplayTheater);
        assert_eq!(state.screen_title(), "Replay Theater");

        state.switch_screen(Screen::IncidentFailureConsole);
        assert_eq!(state.screen_title(), "Incident Failure Console");

        state.switch_screen(Screen::ActionRegistry);
        assert_eq!(state.screen_title(), "Action Registry / Contract Inspector");

        state.switch_screen(Screen::StorageDoctorAiContext);
        assert_eq!(
            state.screen_title(),
            "Storage / Journal Doctor + AI Context"
        );
    }

    // -----------------------------------------------------------------------
    // screen metadata returns canonical Figma labels
    // -----------------------------------------------------------------------

    #[test]
    fn screen_metadata_matches_figma_bundle() {
        let mut state = AppState::new();
        state.switch_screen(Screen::ReplayTheater);
        assert_eq!(
            state.screen_subtitle(),
            "A cinematic flight recorder for durable workflow executions"
        );
        assert_eq!(state.screen_status_chips(), ("Replay safe", "Journaled"));
    }

    #[test]
    fn screen_nav_color_uses_light_semantic_blue_for_overview() {
        let state = AppState::new();
        let [r, g, b, a] = state.screen_nav_color();
        assert!((r - 0.145).abs() < 0.01);
        assert!((g - 0.388).abs() < 0.01);
        assert!((b - 0.922).abs() < 0.01);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn screen_nav_color_uses_light_semantic_red_for_incidents() {
        let mut state = AppState::new();
        state.switch_screen(Screen::IncidentFailureConsole);
        let [r, g, b, a] = state.screen_nav_color();
        assert!((r - 0.898).abs() < 0.01);
        assert!((g - 0.282).abs() < 0.01);
        assert!((b - 0.302).abs() < 0.01);
        assert_eq!(a, 1.0);
    }

    // -----------------------------------------------------------------------
    // ReplayData
    // -----------------------------------------------------------------------

    #[test]
    fn replay_data_new_defaults() {
        let state = AppState::new();
        let replay = &state.replay;
        assert_eq!(replay.playback_position, 0);
        assert_eq!(replay.total_events, 0);
        assert!(replay.transport_state.is_idle());
        assert!((replay.playback_speed - 1.0).abs() < f64::EPSILON);
        assert!(replay.current_step.is_none());
        assert!(replay.step_state.is_none());
    }

    #[test]
    fn replay_data_event_count_text() {
        let mut state = AppState::new();
        assert_eq!(state.replay.event_count_text(), "0 events");
        state.replay.total_events = 42;
        assert_eq!(state.replay.event_count_text(), "42 events");
    }

    #[test]
    fn replay_data_speed_text_slow() {
        let mut state = AppState::new();
        state.replay.playback_speed = 2.5;
        assert_eq!(state.replay.speed_text(), "2.5x");
    }

    #[test]
    fn replay_data_speed_text_fast() {
        let mut state = AppState::new();
        state.replay.playback_speed = 15.0;
        assert_eq!(state.replay.speed_text(), "15x");
    }

    #[test]
    fn replay_data_speed_text_boundary_below_ten() {
        let mut state = AppState::new();
        state.replay.playback_speed = 9.9;
        assert_eq!(state.replay.speed_text(), "9.9x");
    }

    #[test]
    fn replay_data_speed_text_boundary_at_ten() {
        let mut state = AppState::new();
        state.replay.playback_speed = 10.0;
        assert_eq!(state.replay.speed_text(), "10x");
    }

    #[test]
    fn replay_data_run_id_text_some() {
        assert_eq!(ReplayData::run_id_text(Some(12345)), "12345");
    }

    #[test]
    fn replay_data_run_id_text_none() {
        assert_eq!(ReplayData::run_id_text(None), "--");
    }

    #[test]
    fn replay_data_run_id_text_zero() {
        assert_eq!(ReplayData::run_id_text(Some(0)), "0");
    }

    #[test]
    fn replay_data_run_id_text_large_value() {
        assert_eq!(
            ReplayData::run_id_text(Some(u64::MAX)),
            u64::MAX.to_string()
        );
    }

    // -----------------------------------------------------------------------
    // SystemData
    // -----------------------------------------------------------------------

    #[test]
    fn system_data_new_defaults() {
        let state = AppState::new();
        let sys = &state.system;
        assert_eq!(sys.shard_count, 0);
        assert_eq!(sys.total_active_runs, 0);
        assert_eq!(sys.total_queue_depth, 0);
        assert_eq!(sys.overall_health, HealthLevel::Healthy);
    }

    #[test]
    fn system_data_lanes_hint_text_zero_shards() {
        let state = AppState::new();
        assert_eq!(
            state.system.lanes_hint_text(),
            "0 active runs across 0 shards"
        );
    }

    #[test]
    fn system_data_lanes_hint_text_with_data() {
        let mut state = AppState::new();
        state.system.total_active_runs = 7;
        state.system.shard_count = 3;
        assert_eq!(
            state.system.lanes_hint_text(),
            "7 active runs across 3 shards"
        );
    }

    #[test]
    fn system_data_health_text_healthy() {
        let state = AppState::new();
        assert_eq!(state.system.health_text(), "HEALTHY");
    }

    #[test]
    fn system_data_health_text_degraded() {
        let mut state = AppState::new();
        state.system.overall_health = HealthLevel::Degraded;
        assert_eq!(state.system.health_text(), "DEGRADED");
    }

    #[test]
    fn system_data_health_text_critical() {
        let mut state = AppState::new();
        state.system.overall_health = HealthLevel::Critical;
        assert_eq!(state.system.health_text(), "CRITICAL");
    }

    // -----------------------------------------------------------------------
    // HealthLevel enum
    // -----------------------------------------------------------------------

    #[test]
    fn health_level_equality() {
        assert_eq!(HealthLevel::Healthy, HealthLevel::Healthy);
        assert_eq!(HealthLevel::Degraded, HealthLevel::Degraded);
        assert_eq!(HealthLevel::Critical, HealthLevel::Critical);
        assert_ne!(HealthLevel::Healthy, HealthLevel::Degraded);
        assert_ne!(HealthLevel::Degraded, HealthLevel::Critical);
        assert_ne!(HealthLevel::Critical, HealthLevel::Healthy);
    }

    // -----------------------------------------------------------------------
    // IncidentData::new() defaults
    // -----------------------------------------------------------------------

    #[test]
    fn incident_data_new_defaults() {
        let state = AppState::new();
        let inc = &state.incident;
        assert_eq!(inc.active_incidents, 0);
        assert_eq!(inc.critical_count, 0);
        assert_eq!(inc.warning_count, 0);
        assert!(inc.selected_incident.is_none());
    }

    // -----------------------------------------------------------------------
    // VerificationData::new() defaults
    // -----------------------------------------------------------------------

    #[test]
    fn verification_data_new_defaults() {
        let state = AppState::new();
        let v = &state.verification;
        assert_eq!(v.total_checks, 0);
        assert_eq!(v.pass_count, 0);
        assert_eq!(v.warn_count, 0);
        assert_eq!(v.fail_count, 0);
        assert!(v.all_clean);
    }

    #[test]
    fn verification_data_default_matches_new() {
        let from_new = VerificationData::new();
        let from_default = VerificationData::default();
        assert_eq!(from_new.total_checks, from_default.total_checks);
        assert_eq!(from_new.all_clean, from_default.all_clean);
    }

    // -----------------------------------------------------------------------
    // VerificationData::populate_cert_cards — empty certs edge case
    // -----------------------------------------------------------------------

    #[test]
    fn populate_cert_cards_empty_certs_is_all_clean() {
        // Black hat fix #6: empty certs must not flip all_clean to false.
        // Nothing failed, so the state should remain clean.
        let mut vd = VerificationData::new();
        vd.populate_cert_cards(&[]);
        assert!(vd.all_clean, "empty certs should produce all_clean=true");
        assert_eq!(vd.total_checks, 0);
        assert_eq!(vd.pass_count, 0);
        assert_eq!(vd.fail_count, 0);
    }

    // -----------------------------------------------------------------------
    // VerificationData::status_badge_text
    // -----------------------------------------------------------------------

    #[test]
    fn verification_status_badge_all_clean() {
        let mut state = AppState::new();
        state.verification.all_clean = true;
        assert_eq!(
            state.verification.status_badge_text(),
            "PASS (all panels clean)"
        );
    }

    #[test]
    fn verification_status_badge_with_failures() {
        let mut state = AppState::new();
        state.verification.all_clean = false;
        state.verification.total_checks = 10;
        state.verification.fail_count = 2;
        state.verification.warn_count = 3;
        // clean = 10 - 2 - 3 = 5
        assert_eq!(
            state.verification.status_badge_text(),
            "FAIL (5/10 panels clean)"
        );
    }

    #[test]
    fn verification_status_badge_with_warnings_only() {
        let mut state = AppState::new();
        state.verification.all_clean = false;
        state.verification.total_checks = 8;
        state.verification.warn_count = 2;
        // clean = 8 - 2 = 6
        assert_eq!(
            state.verification.status_badge_text(),
            "PASS (6/8 panels clean)"
        );
    }

    #[test]
    fn verification_status_badge_saturating_sub_no_panic() {
        let mut state = AppState::new();
        state.verification.all_clean = false;
        state.verification.total_checks = 1;
        state.verification.fail_count = 5;
        state.verification.warn_count = 5;
        // clean = saturating_sub => 0
        let text = state.verification.status_badge_text();
        assert!(text.contains("FAIL"));
        assert!(text.contains("0/1 panels clean"));
    }

    // -----------------------------------------------------------------------
    // VerificationData::worst_risk_text
    // -----------------------------------------------------------------------

    #[test]
    fn verification_worst_risk_clean() {
        let state = AppState::new();
        assert_eq!(state.verification.worst_risk_text(), "CLEAN");
    }

    #[test]
    fn verification_worst_risk_warning() {
        let mut state = AppState::new();
        state.verification.warn_count = 1;
        assert_eq!(state.verification.worst_risk_text(), "WARNING");
    }

    #[test]
    fn verification_worst_risk_high() {
        let mut state = AppState::new();
        state.verification.fail_count = 1;
        // Fail takes precedence over warn
        state.verification.warn_count = 5;
        assert_eq!(state.verification.worst_risk_text(), "HIGH RISK");
    }

    // -----------------------------------------------------------------------
    // WorkflowData
    // -----------------------------------------------------------------------

    #[test]
    fn workflow_data_new_defaults() {
        let state = AppState::new();
        let wf = &state.workflow;
        assert!(wf.name.is_none());
        assert_eq!(wf.node_count, 0);
    }

    #[test]
    fn workflow_data_default_matches_new() {
        let from_new = WorkflowData::new();
        let from_default = WorkflowData::default();
        assert_eq!(from_new.name, from_default.name);
        assert_eq!(from_new.node_count, from_default.node_count);
    }

    #[test]
    fn workflow_data_display_name_when_set() {
        let mut state = AppState::new();
        state.workflow.name = Some("deploy_pipeline".to_string());
        assert_eq!(state.workflow.display_name(), "deploy_pipeline");
    }

    #[test]
    fn workflow_data_display_name_when_none() {
        let state = AppState::new();
        assert_eq!(state.workflow.display_name(), "unknown");
    }

    #[test]
    fn workflow_data_node_hint_zero() {
        let state = AppState::new();
        assert_eq!(state.workflow.node_hint(), "0 nodes");
    }

    #[test]
    fn workflow_data_node_hint_with_nodes() {
        let mut state = AppState::new();
        state.workflow.node_count = 12;
        assert_eq!(state.workflow.node_hint(), "12 nodes");
    }

    // -----------------------------------------------------------------------
    // sync_system_from_screen
    // -----------------------------------------------------------------------

    #[test]
    fn sync_system_from_screen_empty_starts_healthy() {
        let mut state = AppState::new();
        state.sync_system_from_screen();
        assert_eq!(state.system.shard_count, 0);
        assert_eq!(state.system.total_active_runs, 0);
        assert_eq!(state.system.total_queue_depth, 0);
        assert_eq!(state.system.overall_health, HealthLevel::Healthy);
    }

    #[test]
    fn sync_system_from_screen_propagates_metrics() {
        let mut state = AppState::new();

        // Use the public update_from_metrics path to inject a healthy shard.
        let ipc_shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 5,
            ready_queue_depth: 10,
            action_queue_depth: 3,
            timer_count: 1,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 15.0,
            steps_total: 0,
            actions_total: 0,
        };
        state.system_screen.update_from_metrics(&ipc_shard);

        state.sync_system_from_screen();
        assert_eq!(state.system.shard_count, 1);
        assert_eq!(state.system.total_active_runs, 5);
        assert_eq!(state.system.total_queue_depth, 13); // 10 + 3
        assert_eq!(state.system.overall_health, HealthLevel::Healthy);
    }

    #[test]
    fn sync_system_from_screen_maps_degraded_health() {
        let mut state = AppState::new();

        // Degraded shard: trace ring 75%
        let ipc_shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 60,
            frame_pool_total: 100,
            trace_ring_fill_pct: 75.0,
            steps_total: 0,
            actions_total: 0,
        };
        state.system_screen.update_from_metrics(&ipc_shard);
        state.sync_system_from_screen();
        assert_eq!(state.system.overall_health, HealthLevel::Degraded);
    }

    #[test]
    fn sync_system_from_screen_maps_critical_health() {
        let mut state = AppState::new();

        // Critical shard: pool used 95/100 = 95%
        let ipc_shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 0,
            action_queue_depth: 0,
            timer_count: 0,
            frame_pool_free: 5,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        state.system_screen.update_from_metrics(&ipc_shard);
        state.sync_system_from_screen();
        assert_eq!(state.system.overall_health, HealthLevel::Critical);
    }

    #[test]
    fn sync_system_from_screen_queue_depth_saturating_add() {
        let mut state = AppState::new();

        // We verify that total_queue_depth = ready + action via saturating_add.
        // Inject two shards to check aggregation.
        let shard_a = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 1,
            ready_queue_depth: 100,
            action_queue_depth: 50,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        let shard_b = vb_ipc::ShardMetrics {
            shard_id: 1,
            active_runs: 2,
            ready_queue_depth: 200,
            action_queue_depth: 75,
            timer_count: 0,
            frame_pool_free: 80,
            frame_pool_total: 100,
            trace_ring_fill_pct: 20.0,
            steps_total: 0,
            actions_total: 0,
        };
        state.system_screen.update_from_metrics(&shard_a);
        state.system_screen.update_from_metrics(&shard_b);
        state.sync_system_from_screen();

        // ready total = 100 + 200 = 300, action total = 50 + 75 = 125
        // queue_depth = 300 + 125 = 425
        assert_eq!(state.system.total_queue_depth, 425);
    }

    // ===================================================================
    // Additional comprehensive tests
    // ===================================================================

    // -----------------------------------------------------------------------
    // populate_cert_cards with various certificate results
    // -----------------------------------------------------------------------

    #[test]
    fn populate_cert_cards_all_pass() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_structure".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_10_structure_alloc".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_07_expression_stack_depth".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_08_resources".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_13_taint".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_12_action".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_14_action_policy".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_11_durability".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_15_durability_replay".into(),
                status: "Pass".into(),
                details: String::new(),
            },
        ];
        vd.populate_cert_cards(&certs);
        assert!(vd.all_clean);
        assert_eq!(vd.total_checks, 9);
        assert_eq!(vd.pass_count, 9);
        assert_eq!(vd.fail_count, 0);
        assert_eq!(vd.cert_structure.badge_text, "PASS");
        assert_eq!(vd.cert_bounded.badge_text, "PASS");
        assert_eq!(vd.cert_resources.badge_text, "PASS");
        assert_eq!(vd.cert_taint.badge_text, "PASS");
        assert_eq!(vd.cert_action.badge_text, "PASS");
        assert_eq!(vd.cert_durability.badge_text, "PASS");
    }

    #[test]
    fn populate_cert_cards_mixed_pass_fail() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_structure".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_07_expression_stack_depth".into(),
                status: "Fail".into(),
                details: "stack overflow".into(),
            },
        ];
        vd.populate_cert_cards(&certs);
        assert!(!vd.all_clean);
        assert_eq!(vd.total_checks, 2);
        assert_eq!(vd.pass_count, 1);
        assert_eq!(vd.fail_count, 1);
        assert_eq!(vd.cert_structure.badge_text, "PASS");
        assert_eq!(vd.cert_bounded.badge_text, "FAIL");
    }

    #[test]
    fn populate_cert_cards_all_fail() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_structure".into(),
                status: "Fail".into(),
                details: "bad structure".into(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_13_taint".into(),
                status: "Fail".into(),
                details: "taint leak".into(),
            },
        ];
        vd.populate_cert_cards(&certs);
        assert!(!vd.all_clean);
        assert_eq!(vd.pass_count, 0);
        assert_eq!(vd.fail_count, 2);
        assert_eq!(vd.cert_structure.badge_text, "FAIL");
        assert_eq!(vd.cert_taint.badge_text, "FAIL");
        // Panels with no matching certs should show "--"
        assert_eq!(vd.cert_bounded.badge_text, "--");
        assert_eq!(vd.cert_resources.badge_text, "--");
    }

    #[test]
    fn populate_cert_cards_unrelated_gates_ignored() {
        let mut vd = VerificationData::new();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_99_unknown".into(),
            status: "Pass".into(),
            details: String::new(),
        }];
        vd.populate_cert_cards(&certs);
        // Unknown gate does not match any panel prefix
        assert_eq!(vd.cert_structure.badge_text, "--");
        assert_eq!(vd.cert_bounded.badge_text, "--");
        assert_eq!(vd.cert_resources.badge_text, "--");
        assert_eq!(vd.cert_taint.badge_text, "--");
        assert_eq!(vd.cert_action.badge_text, "--");
        assert_eq!(vd.cert_durability.badge_text, "--");
        // But it still counts as a check (non-"Pass" counts toward fail)
        assert_eq!(vd.total_checks, 1);
    }

    #[test]
    fn populate_cert_cards_panel_field_text() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_structure_a".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_10_structure_b".into(),
                status: "Fail".into(),
                details: "broken".into(),
            },
        ];
        vd.populate_cert_cards(&certs);
        let card = &vd.cert_structure;
        assert_eq!(card.badge_text, "FAIL");
        assert_eq!(card.field1, "total: 2");
        assert_eq!(card.field2, "pass: 1");
        assert_eq!(card.field3, "fail: 1");
        assert_eq!(card.field4, "--");
    }

    // -----------------------------------------------------------------------
    // VerificationState methods
    // -----------------------------------------------------------------------

    #[test]
    fn verification_data_status_badge_all_clean_from_certs() {
        let mut vd = VerificationData::new();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_09_structure".into(),
            status: "Pass".into(),
            details: String::new(),
        }];
        vd.populate_cert_cards(&certs);
        assert_eq!(vd.status_badge_text(), "PASS (all panels clean)");
    }

    #[test]
    fn verification_data_status_badge_with_failures_from_certs() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_07_expr".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_13_taint".into(),
                status: "Fail".into(),
                details: "leak".into(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_08_res".into(),
                status: "Fail".into(),
                details: "overflow".into(),
            },
        ];
        vd.populate_cert_cards(&certs);
        assert!(!vd.all_clean);
        let text = vd.status_badge_text();
        // 3 total, 2 fail -> clean = 3 - 2 - 0 = 1
        assert!(text.contains("FAIL"));
        assert!(text.contains("1/3 panels clean"));
    }

    #[test]
    fn verification_data_worst_risk_from_certs() {
        let mut vd = VerificationData::new();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_09_check".into(),
            status: "Fail".into(),
            details: "bad".into(),
        }];
        vd.populate_cert_cards(&certs);
        assert_eq!(vd.worst_risk_text(), "HIGH RISK");
    }

    #[test]
    fn verification_data_worst_risk_clean_from_certs() {
        let mut vd = VerificationData::new();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_09_check".into(),
            status: "Pass".into(),
            details: String::new(),
        }];
        vd.populate_cert_cards(&certs);
        assert_eq!(vd.worst_risk_text(), "CLEAN");
    }

    // -----------------------------------------------------------------------
    // CertCardStatus methods
    // -----------------------------------------------------------------------

    #[test]
    fn cert_card_status_empty_values() {
        let card = CertCardStatus::empty();
        assert_eq!(card.badge_text, "--");
        assert_eq!(card.field1, "--");
        assert_eq!(card.field2, "--");
        assert_eq!(card.field3, "--");
        assert_eq!(card.field4, "--");
    }

    #[test]
    fn cert_card_status_badge_color_pass() {
        let card = CertCardStatus {
            badge_text: "PASS".into(),
            field1: String::new(),
            field2: String::new(),
            field3: String::new(),
            field4: String::new(),
        };
        assert_eq!(card.badge_color(), "#16a66a");
        assert_eq!(card.field_color(), "#16a66a");
    }

    #[test]
    fn cert_card_status_badge_color_warn() {
        let card = CertCardStatus {
            badge_text: "WARN".into(),
            field1: String::new(),
            field2: String::new(),
            field3: String::new(),
            field4: String::new(),
        };
        assert_eq!(card.badge_color(), "#f59e0b");
        assert_eq!(card.field_color(), "#f59e0b");
    }

    #[test]
    fn cert_card_status_badge_color_fail() {
        let card = CertCardStatus {
            badge_text: "FAIL".into(),
            field1: String::new(),
            field2: String::new(),
            field3: String::new(),
            field4: String::new(),
        };
        assert_eq!(card.badge_color(), "#e5484d");
        assert_eq!(card.field_color(), "#e5484d");
    }

    #[test]
    fn cert_card_status_badge_color_unknown() {
        let card = CertCardStatus {
            badge_text: "--".into(),
            field1: String::new(),
            field2: String::new(),
            field3: String::new(),
            field4: String::new(),
        };
        assert_eq!(card.badge_color(), "#98a2b3");
        assert_eq!(card.field_color(), "#98a2b3");
    }

    // -----------------------------------------------------------------------
    // SystemState health transitions
    // -----------------------------------------------------------------------

    #[test]
    fn system_state_health_transition_healthy_to_degraded() {
        let mut state = AppState::new();
        assert_eq!(state.system.overall_health, HealthLevel::Healthy);
        state.system.overall_health = HealthLevel::Degraded;
        assert_eq!(state.system.overall_health, HealthLevel::Degraded);
        assert_eq!(state.system.health_text(), "DEGRADED");
    }

    #[test]
    fn system_state_health_transition_degraded_to_critical() {
        let mut state = AppState::new();
        state.system.overall_health = HealthLevel::Degraded;
        state.system.overall_health = HealthLevel::Critical;
        assert_eq!(state.system.overall_health, HealthLevel::Critical);
        assert_eq!(state.system.health_text(), "CRITICAL");
    }

    #[test]
    fn system_state_health_transition_critical_back_to_healthy() {
        let mut state = AppState::new();
        state.system.overall_health = HealthLevel::Critical;
        state.system.overall_health = HealthLevel::Healthy;
        assert_eq!(state.system.overall_health, HealthLevel::Healthy);
        assert_eq!(state.system.health_text(), "HEALTHY");
    }

    #[test]
    fn system_state_lanes_hint_updates_with_new_values() {
        let mut state = AppState::new();
        assert_eq!(
            state.system.lanes_hint_text(),
            "0 active runs across 0 shards"
        );
        state.system.total_active_runs = 15;
        state.system.shard_count = 4;
        assert_eq!(
            state.system.lanes_hint_text(),
            "15 active runs across 4 shards"
        );
    }

    // -----------------------------------------------------------------------
    // AppState::new() default values — additional coverage
    // -----------------------------------------------------------------------

    #[test]
    fn app_state_new_last_ipc_error_is_none() {
        let state = AppState::new();
        assert!(state.last_ipc_error.is_none());
    }

    #[test]
    fn app_state_new_replay_defaults() {
        let state = AppState::new();
        assert_eq!(state.replay.playback_position, 0);
        assert_eq!(state.replay.total_events, 0);
        assert!(state.replay.transport_state.is_idle());
        assert!((state.replay.playback_speed - 1.0).abs() < f64::EPSILON);
        assert!(state.replay.current_step.is_none());
        assert!(state.replay.step_state.is_none());
        assert_eq!(state.replay.event_count_text(), "0 events");
        assert_eq!(state.replay.speed_text(), "1.0x");
    }

    #[test]
    fn app_state_new_incident_defaults() {
        let state = AppState::new();
        assert_eq!(state.incident.active_incidents, 0);
        assert_eq!(state.incident.critical_count, 0);
        assert_eq!(state.incident.warning_count, 0);
        assert!(state.incident.selected_incident.is_none());
    }

    #[test]
    fn app_state_new_workflow_defaults() {
        let state = AppState::new();
        assert!(state.workflow.name.is_none());
        assert_eq!(state.workflow.node_count, 0);
        assert_eq!(state.workflow.display_name(), "unknown");
        assert_eq!(state.workflow.node_hint(), "0 nodes");
    }

    #[test]
    fn app_state_new_verification_cert_cards_are_empty() {
        let state = AppState::new();
        assert_eq!(state.verification.cert_structure.badge_text, "--");
        assert_eq!(state.verification.cert_bounded.badge_text, "--");
        assert_eq!(state.verification.cert_resources.badge_text, "--");
        assert_eq!(state.verification.cert_taint.badge_text, "--");
        assert_eq!(state.verification.cert_action.badge_text, "--");
        assert_eq!(state.verification.cert_durability.badge_text, "--");
    }

    // -----------------------------------------------------------------------
    // BLACKHAT security and correctness review tests
    // -----------------------------------------------------------------------

    /// HIGH: warn_count is hardcoded to 0 in populate_cert_cards. The
    /// aggregate counters never populate warn_count regardless of cert
    /// status. This means worst_risk_text() can never return "WARNING"
    /// after populate_cert_cards runs, and the warn_count field is
    /// permanently zero. The status_badge_text() path that handles
    /// warn-only (line 399-402) is unreachable via populate_cert_cards.
    #[test]
    fn blackhat_populate_cert_cards_warn_count_always_zero() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_check".into(),
                status: "Warn".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_13_taint".into(),
                status: "Warn".into(),
                details: String::new(),
            },
        ];
        vd.populate_cert_cards(&certs);
        assert_eq!(
            vd.warn_count, 0,
            "warn_count should be 0 even with Warn certs -- hardcoded to 0"
        );
        // All "Warn" certs count as fail (not "Pass"), so fail_count = 2.
        assert_eq!(vd.fail_count, 2);
        assert!(!vd.all_clean);
    }

    /// MEDIUM: populate_cert_cards treats any non-"Pass" status as a failure.
    /// "Warn" status, unknown statuses like "Skip", and empty strings all
    /// count as failures. This makes the fail_count inflated and the
    /// all_clean flag too conservative.
    #[test]
    fn blackhat_populate_cert_cards_non_pass_treated_as_fail() {
        let mut vd = VerificationData::new();
        let certs = vec![
            vb_ipc::CertificateWire {
                kind: "gate_09_check".into(),
                status: "Pass".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_13_taint".into(),
                status: "Skip".into(),
                details: String::new(),
            },
            vb_ipc::CertificateWire {
                kind: "gate_08_res".into(),
                status: String::new(),
                details: String::new(),
            },
        ];
        vd.populate_cert_cards(&certs);
        // Only gate_09 is "Pass". "Skip" and "" count as fail.
        assert_eq!(vd.pass_count, 1);
        assert_eq!(vd.fail_count, 2);
        assert!(!vd.all_clean);
    }

    /// MEDIUM: build_card inside populate_cert_cards also treats any
    /// non-"Pass" status as fail for per-panel badges. A panel with
    /// only "Warn" certs gets a FAIL badge, not a WARN badge.
    #[test]
    fn blackhat_build_card_treats_warn_as_fail_in_panel() {
        let mut vd = VerificationData::new();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_09_warn".into(),
            status: "Warn".into(),
            details: String::new(),
        }];
        vd.populate_cert_cards(&certs);
        // cert_structure matches gate_09 prefix. The "Warn" status is
        // not "Pass", so the panel badge is FAIL.
        assert_eq!(
            vd.cert_structure.badge_text, "FAIL",
            "Warn status should produce FAIL badge in panel -- no WARN category exists"
        );
    }

    /// MEDIUM: populate_cert_cards with only unrelated gates (gate_99)
    /// produces all_clean=false when pass=0 and fail>0. This means
    /// non-matching gates that are not "Pass" inflate the fail counter
    /// and taint the overall all_clean flag, even though no recognized
    /// panel has any failures.
    #[test]
    fn blackhat_unrelated_gates_taint_all_clean_via_fail_counter() {
        let mut vd = VerificationData::new();
        let certs = vec![vb_ipc::CertificateWire {
            kind: "gate_99_unknown".into(),
            status: "Fail".into(),
            details: "unknown".into(),
        }];
        vd.populate_cert_cards(&certs);
        // All panel badges are "--" (no matching prefixes), but fail_count=1.
        assert_eq!(vd.cert_structure.badge_text, "--");
        assert_eq!(vd.cert_bounded.badge_text, "--");
        assert!(!vd.all_clean, "unrelated gate failure taints all_clean");
        assert_eq!(vd.fail_count, 1);
    }

    /// LOW: VerificationData status_badge_text saturating_sub correctness.
    /// When warn_count > 0 and fail_count == 0, clean = total - warn.
    /// But since populate_cert_cards hardcodes warn_count=0, this path
    /// is only reachable via direct field mutation.
    #[test]
    fn blackhat_status_badge_warn_only_path_reachable_via_direct_mutation() {
        let mut vd = VerificationData::new();
        vd.all_clean = false;
        vd.total_checks = 5;
        vd.warn_count = 2;
        vd.fail_count = 0;
        let text = vd.status_badge_text();
        // clean = 5 - 0 - 2 = 3 (using saturating_sub chain)
        assert!(text.contains("PASS"));
        assert!(text.contains("3/5 panels clean"));
        assert_eq!(vd.worst_risk_text(), "WARNING");
    }

    /// LOW: ReplayData speed_text boundary exactly at 9.95. The boundary
    /// check is `playback_speed < 10.0`, so 9.95 uses the {:.1} format.
    /// format!("{:.1}", 9.95) produces "9.9" (banker's rounding), not "10.0".
    #[test]
    fn blackhat_speed_text_rounding_boundary() {
        let mut state = AppState::new();
        state.replay.playback_speed = 9.95;
        let text = state.replay.speed_text();
        assert_eq!(
            text, "9.9x",
            "9.95 rounds down in .1f format (bankers rounding)"
        );
    }

    /// LOW: Negative playback_speed produces "-1.0x". No validation
    /// prevents negative speeds from being displayed.
    #[test]
    fn blackhat_negative_playback_speed_displays_as_negative() {
        let mut state = AppState::new();
        state.replay.playback_speed = -2.5;
        let text = state.replay.speed_text();
        assert_eq!(text, "-2.5x");
    }

    /// LOW: Zero playback_speed produces "0.0x". Not a valid speed but
    /// the display code handles it without panic.
    #[test]
    fn blackhat_zero_playback_speed_displays_zero() {
        let mut state = AppState::new();
        state.replay.playback_speed = 0.0;
        let text = state.replay.speed_text();
        assert_eq!(text, "0.0x");
    }

    /// LOW: IncidentData critical_count + warning_count can exceed
    /// active_incidents. No invariant enforcement between these fields.
    #[test]
    fn blackhat_incident_counts_can_exceed_active_incidents() {
        let mut state = AppState::new();
        state.incident.active_incidents = 1;
        state.incident.critical_count = 5;
        state.incident.warning_count = 10;
        // No invariant check -- counts can exceed active_incidents.
        assert_eq!(state.incident.active_incidents, 1);
        assert_eq!(state.incident.critical_count, 5);
        assert_eq!(state.incident.warning_count, 10);
    }

    /// LOW: VerificationData all_clean can be true while fail_count > 0
    /// via direct mutation. No invariant enforcement. status_badge_text()
    /// checks all_clean first, so the badge is inconsistent with worst_risk.
    #[test]
    fn blackhat_all_clean_invariant_violation_via_direct_mutation() {
        let mut vd = VerificationData::new();
        vd.all_clean = true;
        vd.fail_count = 3;
        // Inconsistent state: all_clean=true but fail_count > 0.
        assert!(vd.all_clean);
        assert_eq!(vd.fail_count, 3);
        // worst_risk_text checks fail_count first, so it says HIGH RISK.
        assert_eq!(vd.worst_risk_text(), "HIGH RISK");
        // But status_badge_text checks all_clean first, returning PASS.
        // This demonstrates the inconsistency: badge says PASS but risk says HIGH.
        assert!(
            vd.status_badge_text().contains("PASS"),
            "status_badge_text should return PASS because all_clean=true, \
             even though fail_count=3 -- this documents the invariant violation"
        );
    }

    /// LOW: CertCardStatus badge_color returns fallback "#98a2b3" for
    /// any unrecognized badge_text, including empty string.
    #[test]
    fn blackhat_cert_card_badge_color_fallback_for_empty_string() {
        let card = CertCardStatus {
            badge_text: String::new(),
            field1: String::new(),
            field2: String::new(),
            field3: String::new(),
            field4: String::new(),
        };
        assert_eq!(card.badge_color(), "#98a2b3");
        assert_eq!(card.field_color(), "#98a2b3");
    }

    /// LOW: Sync system from screen with zero total_active_runs and
    /// non-zero queue depths. The metrics propagation should reflect
    /// queue depth correctly.
    #[test]
    fn blackhat_sync_system_zero_active_runs_with_queue_depth() {
        let mut state = AppState::new();
        let shard = vb_ipc::ShardMetrics {
            shard_id: 0,
            active_runs: 0,
            ready_queue_depth: 50,
            action_queue_depth: 30,
            timer_count: 0,
            frame_pool_free: 90,
            frame_pool_total: 100,
            trace_ring_fill_pct: 10.0,
            steps_total: 0,
            actions_total: 0,
        };
        state.system_screen.update_from_metrics(&shard);
        state.sync_system_from_screen();
        assert_eq!(state.system.total_active_runs, 0);
        assert_eq!(state.system.total_queue_depth, 80);
    }
}
