/// Cyberpunk mission control color palette.
pub mod colors {
    // Background layers
    pub const CANVAS_BG: [f32; 4] = [0.04, 0.04, 0.07, 1.0]; // #0a0a12
    pub const PANEL_BG: [f32; 4] = [0.07, 0.07, 0.12, 1.0]; // #12121f
    pub const PANEL_BG_ALT: [f32; 4] = [0.10, 0.10, 0.18, 1.0]; // #1a1a2e
    pub const CARD_BG: [f32; 4] = [0.09, 0.09, 0.16, 1.0]; // #16162a
    pub const BORDER: [f32; 4] = [0.16, 0.16, 0.29, 1.0]; // #2a2a4a
    pub const GRID_LINE: [f32; 4] = [0.12, 0.12, 0.23, 1.0]; // #1e1e3a

    // Neon accents
    pub const NEON_CYAN: [f32; 4] = [0.0, 0.96, 1.0, 1.0]; // #00f5ff
    pub const NEON_MAGENTA: [f32; 4] = [1.0, 0.0, 1.0, 1.0]; // #ff00ff
    pub const NEON_YELLOW: [f32; 4] = [1.0, 0.90, 0.0, 1.0]; // #ffe600
    pub const NEON_GREEN: [f32; 4] = [0.22, 1.0, 0.08, 1.0]; // #39ff14
    pub const NEON_RED: [f32; 4] = [1.0, 0.03, 0.23, 1.0]; // #ff073a
    pub const NEON_PURPLE: [f32; 4] = [0.69, 0.30, 1.0, 1.0]; // #b14dff
    pub const NEON_ORANGE: [f32; 4] = [1.0, 0.42, 0.0, 1.0]; // #ff6b00
    pub const NEON_TEAL: [f32; 4] = [0.0, 0.90, 0.78, 1.0]; // #00e5c7
    pub const NEON_PINK: [f32; 4] = [1.0, 0.18, 0.48, 1.0]; // #ff2d7b
    pub const NEON_BLUE: [f32; 4] = [0.18, 0.42, 1.0, 1.0]; // #2d6bff

    // Text
    pub const TEXT_PRIMARY: [f32; 4] = [0.91, 0.91, 1.0, 1.0]; // #e8e8ff
    pub const TEXT_SECONDARY: [f32; 4] = [0.53, 0.53, 0.67, 1.0]; // #8888aa
    pub const TEXT_DIM: [f32; 4] = [0.33, 0.33, 0.47, 1.0]; // #555577

    // State colors
    pub const STATE_SUCCEEDED: [f32; 4] = NEON_GREEN;
    pub const STATE_RUNNING: [f32; 4] = NEON_CYAN;
    pub const STATE_FAILED: [f32; 4] = NEON_RED;
    pub const STATE_WAITING: [f32; 4] = NEON_BLUE;
    pub const STATE_ASKING: [f32; 4] = NEON_YELLOW;
    pub const STATE_PENDING: [f32; 4] = BORDER;
    pub const STATE_CANCELLED: [f32; 4] = TEXT_DIM;
    pub const STATE_SECRET: [f32; 4] = NEON_MAGENTA;
}

/// Node category color mapping for different node kinds.
pub mod node_colors {
    use super::colors;

    pub const DATA: [f32; 4] = colors::TEXT_SECONDARY; // gray for data ops
    pub const EXTERNAL: [f32; 4] = colors::NEON_ORANGE; // Do nodes
    pub const BRANCH: [f32; 4] = colors::NEON_PURPLE; // Choose nodes
    pub const LOOP: [f32; 4] = colors::NEON_BLUE; // ForEach/Collect/Reduce
    pub const PARALLEL: [f32; 4] = colors::NEON_BLUE; // Together
    pub const SUSPEND: [f32; 4] = colors::NEON_GREEN; // Wait/Ask
    pub const ERROR: [f32; 4] = colors::NEON_RED; // ErrorHandler
    pub const TERMINAL: [f32; 4] = colors::NEON_TEAL; // Finish
    pub const CONTROL: [f32; 4] = colors::TEXT_SECONDARY; // Jump/Nop
}
