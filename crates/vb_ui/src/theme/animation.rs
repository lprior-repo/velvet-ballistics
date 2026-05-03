/// Animation durations in seconds.
pub mod duration {
    pub const STATE_TRANSITION: f64 = 0.15;
    pub const CAMERA_PAN: f64 = 0.3;
    pub const CAMERA_ZOOM: f64 = 0.2;
    pub const GLOW_PULSE_SLOW: f64 = 3.0;
    pub const GLOW_PULSE_NORMAL: f64 = 1.5;
    pub const GLOW_PULSE_FAST: f64 = 0.8;
    pub const EVENT_PARTICLE: f64 = 0.5;
    pub const TOOLTIP_FADE: f64 = 0.1;
}

/// Easing functions (stored as Bezier control points for Makepad animator).
pub mod easing {
    pub const EASE_OUT: [f64; 4] = [0.0, 0.0, 0.2, 1.0];
    pub const EASE_IN_OUT: [f64; 4] = [0.4, 0.0, 0.2, 1.0];
    pub const SPRING: [f64; 4] = [0.175, 0.885, 0.32, 1.275];
}
