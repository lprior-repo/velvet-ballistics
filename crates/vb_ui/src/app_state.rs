#![forbid(unsafe_code)]
//! Screen navigation and global app state for the mission control UI.
//!
//! Manages which of the five primary screens is active and holds the
//! per-screen data payloads that the UI reads during rendering.

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
}

/// Replay Theater screen data.
pub struct ReplayData {
    pub playback_position: u32,
    pub total_events: u32,
    pub is_playing: bool,
    pub playback_speed: f64,
    pub current_step: Option<u16>,
    pub step_state: Option<String>,
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

/// Verification screen data.
pub struct VerificationData {
    pub total_checks: u32,
    pub pass_count: u32,
    pub warn_count: u32,
    pub fail_count: u32,
    /// True when all checks pass (no warnings or failures).
    pub all_clean: bool,
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
        }
    }

    /// Returns a human-readable summary string for the verification badge.
    pub fn status_badge_text(&self) -> String {
        if self.all_clean {
            String::from("PASS (all panels clean)")
        } else {
            let total = self.total_checks;
            let clean = self
                .total_checks
                .saturating_sub(self.fail_count)
                .saturating_sub(self.warn_count);
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
