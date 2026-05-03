//! Drawing primitives for the flow editor canvas.
//! Stubs for now — actual Makepad GPU draw calls to be implemented.

/// Port rendering constants
pub mod port {
    pub const RADIUS: f64 = 5.0;
    pub const HIT_SIZE: f64 = 18.0;
    pub const HEIGHT: f64 = 20.0;
    pub const LABEL_GAP: f64 = 4.0;
}

/// Node rendering constants
pub mod node {
    pub const HEADER_HEIGHT: f64 = 32.0;
    pub const MIN_WIDTH: f64 = 160.0;
    pub const MIN_HEIGHT: f64 = 60.0;
    pub const PADDING: f64 = 12.0;
    pub const BORDER_RADIUS: f64 = 6.0;
    pub const BADGE_SIZE: f64 = 16.0;
}

/// Edge rendering constants
pub mod edge {
    pub const DEFAULT_WIDTH: f32 = 2.0;
    pub const PARTICLE_SPEED: f64 = 50.0;
    pub const PARTICLE_SIZE: f64 = 3.0;
}

/// Grid rendering constants
pub mod grid {
    pub const MAJOR_SPACING: f64 = 100.0;
    pub const MINOR_SPACING: f64 = 20.0;
}
