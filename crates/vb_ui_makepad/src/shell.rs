#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShellNav {
    Overview,
    WorkflowGraph,
    Executions,
    Verification,
    Replay,
    Incidents,
    Actions,
    Storage,
}

impl ShellNav {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::WorkflowGraph => "Workflow Graph",
            Self::Executions => "Executions",
            Self::Verification => "Verification",
            Self::Replay => "Replay",
            Self::Incidents => "Incidents",
            Self::Actions => "Actions",
            Self::Storage => "Storage / AI",
        }
    }

    pub const fn nav_color(&self) -> [f32; 4] {
        match self {
            Self::Overview => [0.145, 0.388, 0.922, 1.0],
            Self::WorkflowGraph => [0.431, 0.321, 0.898, 1.0],
            Self::Executions => [0.145, 0.388, 0.922, 1.0],
            Self::Verification => [0.086, 0.651, 0.416, 1.0],
            Self::Replay => [0.169, 0.424, 1.0, 1.0],
            Self::Incidents => [0.898, 0.282, 0.302, 1.0],
            Self::Actions => [0.773, 0.357, 0.083, 1.0],
            Self::Storage => [0.078, 0.722, 0.651, 1.0],
        }
    }

    pub const fn screen(&self) -> Screen {
        match self {
            Self::Overview => Screen::ExecutionOverview,
            Self::WorkflowGraph => Screen::WorkflowGraphAuthoring,
            Self::Executions => Screen::ExecutionDetailsGraph,
            Self::Verification => Screen::VerificationCertificate,
            Self::Replay => Screen::ReplayTheater,
            Self::Incidents => Screen::IncidentFailureConsole,
            Self::Actions => Screen::ActionRegistry,
            Self::Storage => Screen::StorageDoctorAiContext,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShellAction {
    Verify,
    Simulate,
    Submit,
}

#[derive(Debug, Clone)]
pub struct ShellStatusChip {
    pub label: String,
    pub color: [f32; 4],
}

impl ShellStatusChip {
    pub fn new(label: &str, color: [f32; 4]) -> Self {
        Self {
            label: String::from(label),
            color,
        }
    }
}

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

    pub const fn is_shell_screen(self) -> bool {
        true
    }
}

pub struct AppShell {
    pub active_nav: ShellNav,
    pub status_chips: Vec<ShellStatusChip>,
}

impl AppShell {
    pub fn new() -> Result<Self, crate::Error> {
        Ok(Self {
            active_nav: ShellNav::Overview,
            status_chips: Vec::new(),
        })
    }

    pub fn set_active_nav(&mut self, nav: ShellNav) {
        self.active_nav = nav;
    }

    pub fn active_nav(&self) -> ShellNav {
        self.active_nav
    }

    pub fn nav_item_rect(&self, index: usize) -> Rect {
        let sidebar_width = crate::layout::SIDEBAR_WIDTH;
        let item_height = crate::layout::NAV_ITEM_HEIGHT;
        // Safe: index is bounded by nav item count, cast is lossless for our range
        let y = u32::try_from(index).map_or(0.0, f64::from) * item_height;
        Rect {
            x: 0.0,
            y,
            width: sidebar_width,
            height: item_height,
        }
    }

    pub fn topbar_rect(&self) -> Rect {
        Rect {
            x: crate::layout::SIDEBAR_WIDTH,
            y: 0.0,
            width: crate::layout::TOP_BAR_WIDTH,
            height: crate::layout::TOP_BAR_HEIGHT,
        }
    }

    pub fn content_rect(&self) -> Rect {
        Rect {
            x: crate::layout::SIDEBAR_WIDTH,
            y: crate::layout::TOP_BAR_HEIGHT,
            width: crate::layout::CONTENT_WIDTH,
            height: crate::layout::CONTENT_HEIGHT,
        }
    }
}

pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
