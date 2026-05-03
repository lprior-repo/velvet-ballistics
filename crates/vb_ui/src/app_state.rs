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
    pub selected_workflow_digest: Option<[u8; 32]>,
    pub replay: ReplayData,
    pub system: SystemData,
    pub incident: IncidentData,
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
    pub selected_incident: Option<u64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_screen: Screen::RunReplay,
            connected: false,
            selected_run_id: None,
            selected_workflow_digest: None,
            replay: ReplayData::new(),
            system: SystemData::new(),
            incident: IncidentData::new(),
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
}

impl IncidentData {
    fn new() -> Self {
        Self {
            active_incidents: 0,
            critical_count: 0,
            selected_incident: None,
        }
    }
}
