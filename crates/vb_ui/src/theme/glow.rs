/// Glow parameters for a node overlay.
#[derive(Debug, Clone, Copy)]
pub struct GlowParams {
    /// Glow color (RGBA).
    pub color: [f32; 4],
    /// Glow radius in pixels.
    pub radius: f64,
    /// Animation cycle duration in seconds (0.0 = no animation).
    pub pulse_period: f64,
    /// Minimum opacity during pulse (0.0-1.0).
    pub pulse_min: f32,
    /// Maximum opacity during pulse (0.0-1.0).
    pub pulse_max: f32,
}

impl GlowParams {
    pub const fn steady(color: [f32; 4], radius: f64) -> Self {
        Self {
            color,
            radius,
            pulse_period: 0.0,
            pulse_min: 1.0,
            pulse_max: 1.0,
        }
    }

    pub const fn pulsing(color: [f32; 4], radius: f64, period: f64) -> Self {
        Self {
            color,
            radius,
            pulse_period: period,
            pulse_min: 0.3,
            pulse_max: 1.0,
        }
    }
}

/// Predefined glow parameters by state.
pub mod state_glow {
    use super::GlowParams;
    use crate::theme::colors::state;

    pub const RUNNING: GlowParams = GlowParams::pulsing(state::RUNNING, 4.0, 1.5);
    pub const SUCCEEDED: GlowParams = GlowParams::steady(state::SUCCEEDED, 3.0);
    pub const FAILED: GlowParams = GlowParams::pulsing(state::FAILED, 6.0, 0.8);
    pub const WAITING: GlowParams = GlowParams::pulsing(state::WAITING, 2.0, 3.0);
    pub const ASKING: GlowParams = GlowParams::pulsing(state::ASKING, 3.0, 2.0);
    pub const SECRET: GlowParams = GlowParams::steady(state::SECRET, 3.0);
}
