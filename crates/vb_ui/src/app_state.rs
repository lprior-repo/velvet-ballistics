#![forbid(unsafe_code)]
//! Screen navigation and global app state for the mission control UI.
//!
//! Manages which of the five primary screens is active and holds the
//! per-screen data payloads that the UI reads during rendering.

use crate::replay::timeline::TimelineStrip;
use crate::system::screen::SystemScreen;

/// The 5 primary screens of the mission control UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    RunReplay,
    Verification,
    SystemOverview,
    WorkflowGraph,
    IncidentConsole,
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
}

/// Replay Theater screen data.
pub struct ReplayData {
    pub playback_position: u32,
    pub total_events: u32,
    pub is_playing: bool,
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

    /// Returns the neon color hex string for the badge based on status.
    /// neon_green (#39ff14) for PASS, neon_yellow (#ffe600) for WARN,
    /// neon_red (#ff073a) for FAIL.
    pub fn badge_color(&self) -> &'static str {
        match self.badge_text.as_str() {
            "PASS" => "#39ff14",
            "WARN" => "#ffe600",
            "FAIL" => "#ff073a",
            _ => "#555577",
        }
    }

    /// Returns the neon color hex string for field values based on status.
    pub fn field_color(&self) -> &'static str {
        match self.badge_text.as_str() {
            "PASS" => "#39ff14",
            "WARN" => "#ffe600",
            "FAIL" => "#ff073a",
            _ => "#555577",
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
            current_screen: Screen::RunReplay,
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
        }
    }

    pub fn switch_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
    }

    pub fn current_screen(&self) -> Screen {
        self.current_screen
    }

    pub fn screen_title(&self) -> &'static str {
        match self.current_screen {
            Screen::RunReplay => "Replay Theater",
            Screen::Verification => "Verification",
            Screen::SystemOverview => "System Overview",
            Screen::WorkflowGraph => "Workflow Graph",
            Screen::IncidentConsole => "Incident Console",
        }
    }

    /// Returns an RGBA color (each channel 0.0–1.0) used for the active nav tab accent.
    pub fn screen_nav_color(&self) -> [f32; 4] {
        match self.current_screen {
            // Neon cyan: #00f5ff → (0, 0.96, 1.0, 1)
            Screen::RunReplay => [0.0, 0.96, 1.0, 1.0],
            // Neon green: #39ff14 → (0.22, 1.0, 0.08, 1)
            Screen::Verification => [0.22, 1.0, 0.08, 1.0],
            // Neon blue: #2d6bff → (0.18, 0.42, 1.0, 1)
            Screen::SystemOverview => [0.18, 0.42, 1.0, 1.0],
            // Neon purple: #b14dff → (0.69, 0.30, 1.0, 1)
            Screen::WorkflowGraph => [0.69, 0.30, 1.0, 1.0],
            // Neon red: #ff073a → (1.0, 0.03, 0.23, 1)
            Screen::IncidentConsole => [1.0, 0.03, 0.23, 1.0],
        }
    }

    /// Re-derive the lightweight `SystemData` summary fields from the rich
    /// `SystemScreen` model. Call this after updating `system_screen` with
    /// fresh metrics so that the summary struct stays consistent.
    pub fn sync_system_from_screen(&mut self) {
        let metrics = self.system_screen.metrics();
        self.system.shard_count =
            u32::try_from(metrics.shards.len()).unwrap_or(u32::MAX);
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
            is_playing: false,
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
    /// - Action:     gate_14
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
        self.cert_action = build_card(certs, &["gate_14"]);
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
        self.all_clean = fail == 0 && pass > 0;
    }

    /// Returns a human-readable summary string for the verification badge.
    pub fn status_badge_text(&self) -> String {
        if self.all_clean {
            String::from("PASS (all panels clean)")
        } else if self.fail_count > 0 {
            let total = self.total_checks;
            let clean = total.saturating_sub(self.fail_count).saturating_sub(self.warn_count);
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
    fn app_state_new_defaults_to_run_replay_screen() {
        let state = AppState::new();
        assert_eq!(state.current_screen, Screen::RunReplay);
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
            Screen::RunReplay,
            Screen::Verification,
            Screen::SystemOverview,
            Screen::WorkflowGraph,
            Screen::IncidentConsole,
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
        assert_eq!(state.current_screen(), Screen::RunReplay);

        state.switch_screen(Screen::Verification);
        assert_eq!(state.current_screen(), Screen::Verification);

        state.switch_screen(Screen::SystemOverview);
        assert_eq!(state.current_screen(), Screen::SystemOverview);

        state.switch_screen(Screen::WorkflowGraph);
        assert_eq!(state.current_screen(), Screen::WorkflowGraph);

        state.switch_screen(Screen::IncidentConsole);
        assert_eq!(state.current_screen(), Screen::IncidentConsole);
    }

    #[test]
    fn screen_title_returns_correct_labels() {
        let mut state = AppState::new();

        state.switch_screen(Screen::RunReplay);
        assert_eq!(state.screen_title(), "Replay Theater");

        state.switch_screen(Screen::Verification);
        assert_eq!(state.screen_title(), "Verification");

        state.switch_screen(Screen::SystemOverview);
        assert_eq!(state.screen_title(), "System Overview");

        state.switch_screen(Screen::WorkflowGraph);
        assert_eq!(state.screen_title(), "Workflow Graph");

        state.switch_screen(Screen::IncidentConsole);
        assert_eq!(state.screen_title(), "Incident Console");
    }

    // -----------------------------------------------------------------------
    // screen_nav_color returns unique RGBA for each screen
    // -----------------------------------------------------------------------

    #[test]
    fn screen_nav_color_cyan_for_run_replay() {
        let mut state = AppState::new();
        state.switch_screen(Screen::RunReplay);
        let [r, g, b, a] = state.screen_nav_color();
        assert_eq!(r, 0.0);
        assert!((g - 0.96).abs() < 0.01);
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn screen_nav_color_green_for_verification() {
        let mut state = AppState::new();
        state.switch_screen(Screen::Verification);
        let [r, g, b, a] = state.screen_nav_color();
        assert!((r - 0.22).abs() < 0.01);
        assert_eq!(g, 1.0);
        assert!((b - 0.08).abs() < 0.01);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn screen_nav_color_blue_for_system_overview() {
        let mut state = AppState::new();
        state.switch_screen(Screen::SystemOverview);
        let [r, g, b, a] = state.screen_nav_color();
        assert!((r - 0.18).abs() < 0.01);
        assert!((g - 0.42).abs() < 0.01);
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn screen_nav_color_purple_for_workflow_graph() {
        let mut state = AppState::new();
        state.switch_screen(Screen::WorkflowGraph);
        let [r, g, b, a] = state.screen_nav_color();
        assert!((r - 0.69).abs() < 0.01);
        assert!((g - 0.30).abs() < 0.01);
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn screen_nav_color_red_for_incident_console() {
        let mut state = AppState::new();
        state.switch_screen(Screen::IncidentConsole);
        let [r, g, b, a] = state.screen_nav_color();
        assert_eq!(r, 1.0);
        assert!((g - 0.03).abs() < 0.01);
        assert!((b - 0.23).abs() < 0.01);
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
        assert!(!replay.is_playing);
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
        assert_eq!(ReplayData::run_id_text(Some(u64::MAX)), u64::MAX.to_string());
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
}
