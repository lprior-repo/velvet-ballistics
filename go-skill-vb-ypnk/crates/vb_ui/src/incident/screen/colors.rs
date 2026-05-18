#![forbid(unsafe_code)]
//! Cyberpunk color constants for incident screen.

pub const CANVAS_BG: &str = "#0a0a12";
pub const PANEL_BG: &str = "#12121f";
pub const PANEL_BG_ALT: &str = "#1a1a2e";
pub const CARD_BG: &str = "#16162a";
pub const BORDER: &str = "#2a2a4a";
pub const GRID_LINE: &str = "#1e1e3a";

pub const NEON_CYAN: &str = "#00f5ff";
pub const NEON_MAGENTA: &str = "#ff00ff";
pub const NEON_YELLOW: &str = "#ffe600";
pub const NEON_GREEN: &str = "#39ff14";
pub const NEON_RED: &str = "#ff073a";
pub const NEON_PURPLE: &str = "#b14dff";
pub const NEON_ORANGE: &str = "#ff6b00";
pub const NEON_TEAL: &str = "#00e5c7";
pub const NEON_PINK: &str = "#ff2d7b";
pub const NEON_BLUE: &str = "#2d6bff";

pub const TEXT_PRIMARY: &str = "#e8e8ff";
pub const TEXT_SECONDARY: &str = "#8888aa";
pub const TEXT_DIM: &str = "#555577";
pub const TEXT_ACCENT: &str = "#00f5ff";

pub const STATE_SUCCEEDED: &str = "#39ff14";
pub const STATE_RUNNING: &str = "#00f5ff";
pub const STATE_FAILED: &str = "#ff073a";
pub const STATE_WAITING: &str = "#2d6bff";
pub const STATE_RETRYING: &str = "#ff6b00";
pub const STATE_CANCELLED: &str = "#555577";
pub const STATE_SECRET_TAINTED: &str = "#ff00ff";
