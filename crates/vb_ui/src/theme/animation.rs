#![forbid(unsafe_code)]
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

/// Supported easing curve types for UI state transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum EasingFunction {
    /// Linear interpolation: constant speed.
    Linear,
    /// Ease-in: slow start, fast finish (quadratic).
    EaseIn,
    /// Ease-out: fast start, slow finish (quadratic).
    EaseOut,
    /// Ease-in-out: slow start and end, fast middle (quadratic).
    EaseInOut,
}

impl EasingFunction {
    /// Evaluate the easing function at time `t` (must be in 0.0..=1.0).
    ///
    /// Returns a value in 0.0..=1.0 representing the eased progress.
    /// Values of `t` outside 0.0..=1.0 are clamped.
    pub fn evaluate(&self, t: f32) -> f32 {
        let tc = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => tc,
            Self::EaseIn => tc * tc,
            Self::EaseOut => {
                let one_minus = 1.0_f32 - tc;
                1.0_f32 - one_minus * one_minus
            }
            Self::EaseInOut => {
                if tc < 0.5 {
                    2.0_f32.mul_add(tc * tc, 0.0)
                } else {
                    let one_minus = 1.0_f32 - tc;
                    1.0_f32 - 2.0 * one_minus * one_minus
                }
            }
        }
    }
}

/// Configuration for a neon pulse (oscillating alpha) effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseConfig {
    /// Oscillation frequency in hertz.
    pub frequency_hz: f32,
    /// Minimum alpha value during the pulse cycle.
    pub min_alpha: f32,
    /// Maximum alpha value during the pulse cycle.
    pub max_alpha: f32,
}

impl PulseConfig {
    /// Evaluate the pulse alpha at time `t_secs`.
    ///
    /// Uses a sine wave: the result oscillates between `min_alpha` and `max_alpha`
    /// at the configured `frequency_hz`.
    pub fn evaluate(&self, t_secs: f32) -> f32 {
        // sin oscillates in [-1, 1]; remap to [0, 1] then scale to [min, max].
        let sine = (2.0 * std::f32::consts::PI * self.frequency_hz * t_secs).sin();
        // Map [-1, 1] -> [0, 1]
        let normalized = (sine + 1.0) * 0.5;
        // Map [0, 1] -> [min_alpha, max_alpha]
        self.min_alpha + (self.max_alpha - self.min_alpha) * normalized
    }
}

/// Configuration for a neon glow effect (bloom/blur overlay).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlowConfig {
    /// Glow spread radius in pixels.
    pub radius: f32,
    /// Glow intensity multiplier (0.0 = invisible, 1.0 = full).
    pub intensity: f32,
    /// Glow color as RGBA (each channel 0.0..=1.0).
    pub color: [f32; 4],
}

/// Configuration for a UI state transition (e.g. panel slide, fade).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionConfig {
    /// Total duration of the transition in seconds.
    pub duration_secs: f32,
    /// Easing curve applied to the transition.
    pub easing: EasingFunction,
}

impl TransitionConfig {
    /// Compute the eased progress fraction at elapsed time `t_secs`.
    ///
    /// Returns a value in 0.0..=1.0. Values beyond `duration_secs` are clamped to 1.0.
    /// Returns 0.0 for zero or negative elapsed time.
    /// Returns 1.0 for zero duration transitions.
    pub fn progress_at(&self, t_secs: f32) -> f32 {
        if self.duration_secs <= 0.0 {
            return if t_secs <= 0.0 { 0.0 } else { 1.0 };
        }
        let raw = t_secs / self.duration_secs;
        self.easing.evaluate(raw)
    }
}

/// Predefined pulse configs matching the cyberpunk palette.
pub mod pulse {
    use super::PulseConfig;

    /// Neon cyan pulse for running state (2.0 Hz, 0.3-1.0 alpha).
    pub const RUNNING: PulseConfig = PulseConfig {
        frequency_hz: 2.0,
        min_alpha: 0.3,
        max_alpha: 1.0,
    };

    /// Neon blue slow pulse for waiting state (0.5 Hz, 0.2-0.8 alpha).
    pub const WAITING: PulseConfig = PulseConfig {
        frequency_hz: 0.5,
        min_alpha: 0.2,
        max_alpha: 0.8,
    };

    /// Neon orange glow cycle for retried state (1.5 Hz, 0.4-1.0 alpha).
    pub const RETRIED: PulseConfig = PulseConfig {
        frequency_hz: 1.5,
        min_alpha: 0.4,
        max_alpha: 1.0,
    };
}

/// Predefined glow configs matching the cyberpunk palette.
pub mod glow {
    use super::GlowConfig;
    use crate::theme::colors::neon;

    /// Neon red glow for failed state (radius 4.0, intensity 0.8).
    pub const FAILED: GlowConfig = GlowConfig {
        radius: 4.0,
        intensity: 0.8,
        color: neon::RED,
    };
}

/// Predefined transition configs.
pub mod transition {
    use super::{EasingFunction, TransitionConfig};

    /// Default UI state transition (0.3s, ease-in-out).
    pub const DEFAULT: TransitionConfig = TransitionConfig {
        duration_secs: 0.3,
        easing: EasingFunction::EaseInOut,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f32 = 0.001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < TOLERANCE
    }

    // --- EasingFunction tests ---

    #[test]
    fn easing_linear_returns_identity() {
        assert!(approx_eq(EasingFunction::Linear.evaluate(0.0), 0.0));
        assert!(approx_eq(EasingFunction::Linear.evaluate(0.5), 0.5));
        assert!(approx_eq(EasingFunction::Linear.evaluate(1.0), 1.0));
    }

    #[test]
    fn easing_ease_in_is_slow_then_fast() {
        let e = EasingFunction::EaseIn;
        // Quadratic: t^2
        assert!(approx_eq(e.evaluate(0.0), 0.0));
        assert!(approx_eq(e.evaluate(0.5), 0.25));
        assert!(approx_eq(e.evaluate(1.0), 1.0));
        // At t=0.25 the eased value (0.0625) should be less than linear (0.25)
        assert!(e.evaluate(0.25) < 0.25);
    }

    #[test]
    fn easing_ease_out_is_fast_then_slow() {
        let e = EasingFunction::EaseOut;
        assert!(approx_eq(e.evaluate(0.0), 0.0));
        assert!(approx_eq(e.evaluate(1.0), 1.0));
        // At t=0.75 the eased value should be greater than linear (0.75)
        assert!(e.evaluate(0.75) > 0.75);
        // Exact: 1 - (1-0.5)^2 = 1 - 0.25 = 0.75
        assert!(approx_eq(e.evaluate(0.5), 0.75));
    }

    #[test]
    fn easing_ease_in_out_has_inflection_at_half() {
        let e = EasingFunction::EaseInOut;
        assert!(approx_eq(e.evaluate(0.0), 0.0));
        assert!(approx_eq(e.evaluate(0.5), 0.5));
        assert!(approx_eq(e.evaluate(1.0), 1.0));
        // First half: 2*t^2 => at t=0.25 => 2*0.0625 = 0.125
        assert!(approx_eq(e.evaluate(0.25), 0.125));
        // Second half: 1 - 2*(1-t)^2 => at t=0.75 => 1 - 2*0.0625 = 0.875
        assert!(approx_eq(e.evaluate(0.75), 0.875));
    }

    #[test]
    fn easing_clamps_out_of_range() {
        let e = EasingFunction::Linear;
        assert!(approx_eq(e.evaluate(-0.5), 0.0));
        assert!(approx_eq(e.evaluate(1.5), 1.0));
    }

    // --- PulseConfig tests ---

    #[test]
    fn pulse_at_time_zero_is_midpoint() {
        // sin(0) = 0 => normalized = 0.5 => midpoint of [min, max]
        let cfg = PulseConfig {
            frequency_hz: 1.0,
            min_alpha: 0.0,
            max_alpha: 1.0,
        };
        let alpha = cfg.evaluate(0.0);
        assert!(approx_eq(alpha, 0.5), "expected 0.5, got {alpha}");
    }

    #[test]
    fn pulse_running_at_peak() {
        // For 2.0 Hz, sin peak at t where 2*pi*f*t = pi/2 => t = 1/(4*f)
        // sin(pi/2) = 1 => normalized = 1.0 => max_alpha
        let cfg = pulse::RUNNING;
        let quarter_period = 1.0_f32 / (4.0_f32 * cfg.frequency_hz);
        let alpha = cfg.evaluate(quarter_period);
        assert!(
            approx_eq(alpha, cfg.max_alpha),
            "expected {}, got {alpha}",
            cfg.max_alpha,
        );
    }

    #[test]
    fn pulse_waiting_is_slow() {
        // 0.5 Hz => period = 2s. At t=1s we should be at half period => sin(pi) = 0 => normalized = 0.5
        let cfg = pulse::WAITING;
        let period = 1.0_f32 / cfg.frequency_hz;
        let alpha = cfg.evaluate(period * 0.5);
        // sin(pi) is approximately 0
        let mid = cfg.min_alpha + (cfg.max_alpha - cfg.min_alpha) * 0.5;
        assert!((alpha - mid).abs() < 0.01, "expected ~{mid}, got {alpha}",);
    }

    #[test]
    fn pulse_stays_within_bounds() {
        let cfg = pulse::RETRIED;
        // Sample many time points to verify bounds.
        let mut i: u32 = 0;
        while i < 1000 {
            // i < 1000, safe for f32 precision.
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            let t = i as f32 * 0.01;
            let alpha = cfg.evaluate(t);
            assert!(
                alpha >= cfg.min_alpha - TOLERANCE && alpha <= cfg.max_alpha + TOLERANCE,
                "alpha {alpha} out of [{}, {}] at t={t}",
                cfg.min_alpha,
                cfg.max_alpha,
            );
            i += 1;
        }
    }

    // --- GlowConfig tests ---

    #[test]
    fn glow_failed_uses_neon_red() {
        let g = glow::FAILED;
        assert_eq!(g.color, crate::theme::colors::neon::RED);
        assert!(approx_eq(g.radius, 4.0));
        assert!(approx_eq(g.intensity, 0.8));
    }

    // --- TransitionConfig tests ---

    #[test]
    fn transition_progress_starts_at_zero() {
        let tc = transition::DEFAULT;
        assert!(approx_eq(tc.progress_at(0.0), 0.0));
    }

    #[test]
    fn transition_progress_ends_at_one() {
        let tc = transition::DEFAULT;
        assert!(approx_eq(tc.progress_at(tc.duration_secs), 1.0));
    }

    #[test]
    fn transition_progress_clamps_beyond_duration() {
        let tc = transition::DEFAULT;
        assert!(approx_eq(tc.progress_at(10.0), 1.0));
    }

    #[test]
    fn transition_zero_duration_returns_one_for_positive_time() {
        let tc = TransitionConfig {
            duration_secs: 0.0,
            easing: EasingFunction::Linear,
        };
        assert!(approx_eq(tc.progress_at(0.0), 0.0));
        assert!(approx_eq(tc.progress_at(0.001), 1.0));
    }

    #[test]
    fn transition_uses_easing_curve() {
        let tc = TransitionConfig {
            duration_secs: 1.0,
            easing: EasingFunction::EaseIn,
        };
        // At t=0.5 with EaseIn => 0.25
        assert!(approx_eq(tc.progress_at(0.5), 0.25));
    }
}
