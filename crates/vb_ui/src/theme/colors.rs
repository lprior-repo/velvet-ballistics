/// Background layers
pub mod bg {
    pub const CANVAS: [f32; 4] = [0.039, 0.039, 0.071, 1.0]; // #0a0a12
    pub const PANEL: [f32; 4] = [0.071, 0.071, 0.122, 1.0]; // #12121f
    pub const PANEL_ALT: [f32; 4] = [0.102, 0.102, 0.180, 1.0]; // #1a1a2e
    pub const CARD: [f32; 4] = [0.086, 0.086, 0.165, 1.0]; // #16162a
    pub const CARD_HOVER: [f32; 4] = [0.118, 0.118, 0.220, 1.0]; // #1e1e38
    pub const BORDER: [f32; 4] = [0.165, 0.165, 0.290, 1.0]; // #2a2a4a
    pub const BORDER_BRIGHT: [f32; 4] = [0.247, 0.247, 0.420, 1.0]; // #3f3f6b
    pub const GRID: [f32; 4] = [0.118, 0.118, 0.227, 1.0]; // #1e1e3a
}

/// Neon accent colors
pub mod neon {
    pub const CYAN: [f32; 4] = [0.000, 0.961, 1.000, 1.0]; // #00f5ff
    pub const CYAN_DIM: [f32; 4] = [0.000, 0.482, 0.502, 1.0]; // #007b80
    pub const MAGENTA: [f32; 4] = [1.000, 0.000, 1.000, 1.0]; // #ff00ff
    pub const YELLOW: [f32; 4] = [1.000, 0.902, 0.000, 1.0]; // #ffe600
    pub const GREEN: [f32; 4] = [0.224, 1.000, 0.078, 1.0]; // #39ff14
    pub const GREEN_DIM: [f32; 4] = [0.112, 0.502, 0.039, 1.0]; // #1d800a
    pub const RED: [f32; 4] = [1.000, 0.027, 0.227, 1.0]; // #ff073a
    pub const RED_DIM: [f32; 4] = [0.502, 0.014, 0.114, 1.0]; // #80041d
    pub const PURPLE: [f32; 4] = [0.694, 0.302, 1.000, 1.0]; // #b14dff
    pub const ORANGE: [f32; 4] = [1.000, 0.420, 0.000, 1.0]; // #ff6b00
    pub const TEAL: [f32; 4] = [0.000, 0.898, 0.780, 1.0]; // #00e5c7
    pub const PINK: [f32; 4] = [1.000, 0.176, 0.482, 1.0]; // #ff2d7b
    pub const BLUE: [f32; 4] = [0.176, 0.420, 1.000, 1.0]; // #2d6bff
    pub const BLUE_DIM: [f32; 4] = [0.088, 0.212, 0.502, 1.0]; // #163680
}

/// Text colors
pub mod text {
    use super::neon;

    pub const PRIMARY: [f32; 4] = [0.910, 0.910, 1.000, 1.0]; // #e8e8ff
    pub const SECONDARY: [f32; 4] = [0.533, 0.533, 0.667, 1.0]; // #8888aa
    pub const DIM: [f32; 4] = [0.333, 0.333, 0.467, 1.0]; // #555577
    pub const ACCENT: [f32; 4] = neon::CYAN;
    pub const SUCCESS: [f32; 4] = neon::GREEN;
    pub const ERROR: [f32; 4] = neon::RED;
    pub const WARNING: [f32; 4] = neon::YELLOW;
}

/// State-specific colors for step states
pub mod state {
    use super::{neon, text};

    pub const PENDING: [f32; 4] = [0.165, 0.165, 0.290, 1.0]; // #2a2a4a
    pub const RUNNING: [f32; 4] = neon::CYAN;
    pub const SUCCEEDED: [f32; 4] = neon::GREEN;
    pub const FAILED: [f32; 4] = neon::RED;
    pub const SKIPPED: [f32; 4] = text::DIM;
    pub const WAITING: [f32; 4] = neon::BLUE;
    pub const ASKING: [f32; 4] = neon::YELLOW;
    pub const CANCELLED: [f32; 4] = text::DIM;
    pub const SECRET: [f32; 4] = neon::MAGENTA;
}

/// Node category colors (body fill, slightly muted from neon)
pub mod node_category {
    pub const DATA: [f32; 4] = [0.133, 0.133, 0.200, 1.0]; // muted gray-blue
    pub const EXTERNAL: [f32; 4] = [0.200, 0.118, 0.039, 1.0]; // muted orange
    pub const BRANCH: [f32; 4] = [0.180, 0.098, 0.251, 1.0]; // muted purple
    pub const LOOP: [f32; 4] = [0.078, 0.157, 0.251, 1.0]; // muted blue
    pub const PARALLEL: [f32; 4] = [0.078, 0.157, 0.251, 1.0]; // muted blue
    pub const COLLECT: [f32; 4] = [0.078, 0.157, 0.251, 1.0]; // muted blue
    pub const REDUCE: [f32; 4] = [0.078, 0.157, 0.251, 1.0]; // muted blue
    pub const SUSPEND: [f32; 4] = [0.078, 0.200, 0.098, 1.0]; // muted green
    pub const ERROR: [f32; 4] = [0.251, 0.078, 0.098, 1.0]; // muted red
    pub const TERMINAL: [f32; 4] = [0.039, 0.200, 0.180, 1.0]; // muted teal
    pub const CONTROL: [f32; 4] = [0.133, 0.133, 0.200, 1.0]; // muted gray
}

/// Node header colors (darker than body, for DoubleRoundedRect style)
pub mod node_header {
    pub const DATA: [f32; 4] = [0.098, 0.098, 0.157, 1.0];
    pub const EXTERNAL: [f32; 4] = [0.157, 0.086, 0.027, 1.0];
    pub const BRANCH: [f32; 4] = [0.133, 0.071, 0.196, 1.0];
    pub const LOOP: [f32; 4] = [0.055, 0.118, 0.196, 1.0];
    pub const PARALLEL: [f32; 4] = [0.055, 0.118, 0.196, 1.0];
    pub const COLLECT: [f32; 4] = [0.055, 0.118, 0.196, 1.0];
    pub const REDUCE: [f32; 4] = [0.055, 0.118, 0.196, 1.0];
    pub const SUSPEND: [f32; 4] = [0.055, 0.157, 0.071, 1.0];
    pub const ERROR: [f32; 4] = [0.196, 0.055, 0.071, 1.0];
    pub const TERMINAL: [f32; 4] = [0.027, 0.157, 0.133, 1.0];
    pub const CONTROL: [f32; 4] = [0.098, 0.098, 0.157, 1.0];
}

/// Queue pressure gradient (low -> medium -> high -> critical)
pub mod pressure {
    use super::neon;

    pub const LOW: [f32; 4] = neon::CYAN;
    pub const MEDIUM: [f32; 4] = neon::YELLOW;
    pub const HIGH: [f32; 4] = neon::ORANGE;
    pub const CRITICAL: [f32; 4] = neon::RED;
}

/// Hex string versions for Makepad color literals
pub mod hex {
    pub const CANVAS_BG: &str = "#0a0a12";
    pub const PANEL_BG: &str = "#12121f";
    pub const CARD_BG: &str = "#16162a";
    pub const BORDER: &str = "#2a2a4a";
    pub const NEON_CYAN: &str = "#00f5ff";
    pub const NEON_MAGENTA: &str = "#ff00ff";
    pub const NEON_YELLOW: &str = "#ffe600";
    pub const NEON_GREEN: &str = "#39ff14";
    pub const NEON_RED: &str = "#ff073a";
    pub const NEON_PURPLE: &str = "#b14dff";
    pub const NEON_ORANGE: &str = "#ff6b00";
    pub const NEON_TEAL: &str = "#00e5c7";
    pub const NEON_BLUE: &str = "#2d6bff";
    pub const TEXT_PRIMARY: &str = "#e8e8ff";
    pub const TEXT_SECONDARY: &str = "#8888aa";
    pub const TEXT_DIM: &str = "#555577";
}
