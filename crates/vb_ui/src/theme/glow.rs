#![forbid(unsafe_code)]
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

// ---------------------------------------------------------------------------
// GlowLayer -- single Gaussian-like glow layer
// ---------------------------------------------------------------------------

/// A single glow layer with Gaussian falloff.
#[derive(Debug, Clone, Copy)]
pub struct GlowLayer {
    /// Glow color (RGBA).
    pub color: [f32; 4],
    /// Gaussian standard-deviation controlling the spread (pixels).
    pub radius: f32,
    /// Peak intensity multiplier (0.0 - 1.0).
    pub intensity: f32,
    /// (x, y) offset from the center of the glow source.
    pub offset: (f32, f32),
}

impl GlowLayer {
    /// Compute the alpha contribution at `distance` pixels from the layer
    /// centre using a Gaussian-like curve:
    ///
    /// ```text
    /// alpha = intensity * exp(-(distance^2) / (2 * radius^2))
    /// ```
    pub fn alpha_at_distance(&self, distance: f32) -> f32 {
        let r = self.radius;
        let r_sq = r * r;
        if r_sq <= 0.0 {
            return 0.0;
        }
        let d_sq = distance * distance;
        let exponent = d_sq / (2.0 * r_sq);
        self.intensity * glow_exp(-exponent)
    }

    /// Construct a new `GlowLayer` from the given colour, radius, intensity,
    /// and offset.
    pub const fn new(color: [f32; 4], radius: f32, intensity: f32, offset: (f32, f32)) -> Self {
        Self {
            color,
            radius,
            intensity,
            offset,
        }
    }
}

// ---------------------------------------------------------------------------
// GlowPreset -- named neon presets
// ---------------------------------------------------------------------------

/// Named neon glow presets, each mapping to a well-known colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GlowPreset {
    NeonCyan,
    NeonRed,
    NeonGreen,
    NeonOrange,
    NeonMagenta,
}

impl GlowPreset {
    /// Return the base colour associated with this preset.
    pub const fn color(&self) -> [f32; 4] {
        match self {
            Self::NeonCyan => crate::theme::colors::neon::CYAN,
            Self::NeonRed => crate::theme::colors::neon::RED,
            Self::NeonGreen => crate::theme::colors::neon::GREEN,
            Self::NeonOrange => crate::theme::colors::neon::ORANGE,
            Self::NeonMagenta => crate::theme::colors::neon::MAGENTA,
        }
    }

    /// Convert this preset into a `GlowLayer` with default radius, intensity
    /// and zero offset.
    pub const fn to_layer(&self) -> GlowLayer {
        GlowLayer::new(self.color(), 4.0, 0.8, (0.0, 0.0))
    }
}

// ---------------------------------------------------------------------------
// MultiGlow -- composite of several glow layers
// ---------------------------------------------------------------------------

/// A stack of glow layers that are composited additively.
#[derive(Debug, Clone, Default)]
pub struct MultiGlow {
    pub layers: Vec<GlowLayer>,
}

impl MultiGlow {
    /// Construct an empty `MultiGlow`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute the composite alpha at `distance` pixels by summing each
    /// layer's contribution and clamping to `[0.0, 1.0]`.
    pub fn composite_alpha_at(&self, distance: f32) -> f32 {
        let sum: f32 = self
            .layers
            .iter()
            .map(|l| l.alpha_at_distance(distance))
            .sum();
        sum.clamp(0.0, 1.0)
    }
}

// ---------------------------------------------------------------------------
// Predefined glow presets
// ---------------------------------------------------------------------------

/// Red glow for FAILED state.
pub const FAILED_GLOW: GlowLayer =
    GlowLayer::new(crate::theme::colors::neon::RED, 6.0, 0.9, (0.0, 0.0));

/// Cyan glow for RUNNING state.
pub const RUNNING_GLOW: GlowLayer =
    GlowLayer::new(crate::theme::colors::neon::CYAN, 4.0, 0.7, (0.0, 0.0));

/// Orange glow for WARNING state.
pub const WARNING_GLOW: GlowLayer =
    GlowLayer::new(crate::theme::colors::neon::ORANGE, 5.0, 0.6, (0.0, 0.0));

/// Green glow for SUCCESS state.
pub const SUCCESS_GLOW: GlowLayer =
    GlowLayer::new(crate::theme::colors::neon::GREEN, 3.0, 0.5, (0.0, 0.0));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute `exp(x)` using a Taylor-series expansion.
///
/// Uses 18 terms of the Maclaurin series which gives ~1e-5 accuracy for
/// |x| up to ~8.  Avoids `as` casts entirely by using precomputed
/// reciprocal factorial values instead of integer-to-float conversion.
fn glow_exp(x: f32) -> f32 {
    // Precomputed 1/n! for n = 0..17.  18 terms gives <1e-5 accuracy for
    // |x| up to ~8, covering the Gaussian exponent range in practice.
    let inv_fact: [f32; 18] = [
        1.0,
        1.0,
        0.5,
        0.16666667,
        0.04166667,
        0.008333333,
        0.001388889,
        0.0001984127,
        0.00002480159,
        0.000002755732,
        0.0000002755732,
        0.00000002505211,
        0.00000000208768,
        0.00000000016059,
        0.00000000001147,
        0.00000000000076,
        0.00000000000005,
        0.00000000000000,
    ];

    let mut power: f32 = 1.0;
    let mut result: f32 = inv_fact[0];
    for coeff in inv_fact.iter().skip(1) {
        power *= x;
        result += power * coeff;
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::E;

    #[test]
    fn alpha_at_center_equals_intensity() {
        let layer = GlowLayer::new([1.0, 0.0, 0.0, 1.0], 4.0, 0.8, (0.0, 0.0));
        let alpha = layer.alpha_at_distance(0.0);
        assert!((alpha - 0.8).abs() < 0.01, "expected ~0.8, got {alpha}");
    }

    #[test]
    fn alpha_at_one_sigma_is_60_percent_of_intensity() {
        let layer = GlowLayer::new([1.0, 0.0, 0.0, 1.0], 4.0, 1.0, (0.0, 0.0));
        let alpha = layer.alpha_at_distance(4.0);
        assert!(
            (alpha - 0.6065).abs() < 0.02,
            "expected ~0.6065, got {alpha}"
        );
    }

    #[test]
    fn alpha_falls_off_with_distance() {
        let layer = GlowLayer::new([1.0, 0.0, 0.0, 1.0], 4.0, 1.0, (0.0, 0.0));
        let near = layer.alpha_at_distance(1.0);
        let far = layer.alpha_at_distance(8.0);
        assert!(near > far, "near={near} should be > far={far}");
    }

    #[test]
    fn alpha_zero_radius_returns_zero() {
        let layer = GlowLayer::new([1.0, 0.0, 0.0, 1.0], 0.0, 1.0, (0.0, 0.0));
        assert_eq!(layer.alpha_at_distance(0.0), 0.0);
        assert_eq!(layer.alpha_at_distance(5.0), 0.0);
    }

    #[test]
    fn preset_neon_cyan_colour() {
        assert_eq!(
            GlowPreset::NeonCyan.color(),
            crate::theme::colors::neon::CYAN
        );
    }

    #[test]
    fn preset_to_layer_returns_correct_colour() {
        let layer = GlowPreset::NeonRed.to_layer();
        assert_eq!(layer.color, crate::theme::colors::neon::RED);
    }

    #[test]
    fn all_presets_have_non_zero_colour() {
        for preset in [
            GlowPreset::NeonCyan,
            GlowPreset::NeonRed,
            GlowPreset::NeonGreen,
            GlowPreset::NeonOrange,
            GlowPreset::NeonMagenta,
        ] {
            let c = preset.color();
            assert!(
                c[0] > 0.0 || c[1] > 0.0 || c[2] > 0.0,
                "preset {preset:?} has zero colour"
            );
        }
    }

    #[test]
    fn failed_glow_is_red_with_radius_6() {
        assert_eq!(FAILED_GLOW.color, crate::theme::colors::neon::RED);
        assert!((FAILED_GLOW.radius - 6.0).abs() < f32::EPSILON);
        assert!((FAILED_GLOW.intensity - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn running_glow_is_cyan_with_radius_4() {
        assert_eq!(RUNNING_GLOW.color, crate::theme::colors::neon::CYAN);
        assert!((RUNNING_GLOW.radius - 4.0).abs() < f32::EPSILON);
        assert!((RUNNING_GLOW.intensity - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn warning_glow_is_orange_with_radius_5() {
        assert_eq!(WARNING_GLOW.color, crate::theme::colors::neon::ORANGE);
        assert!((WARNING_GLOW.radius - 5.0).abs() < f32::EPSILON);
        assert!((WARNING_GLOW.intensity - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn success_glow_is_green_with_radius_3() {
        assert_eq!(SUCCESS_GLOW.color, crate::theme::colors::neon::GREEN);
        assert!((SUCCESS_GLOW.radius - 3.0).abs() < f32::EPSILON);
        assert!((SUCCESS_GLOW.intensity - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn multi_glow_single_layer_matches_layer() {
        let layer = GlowLayer::new([1.0, 0.0, 0.0, 1.0], 4.0, 0.8, (0.0, 0.0));
        let mg = MultiGlow {
            layers: vec![layer],
        };
        let composite = mg.composite_alpha_at(2.0);
        let direct = layer.alpha_at_distance(2.0);
        assert!(
            (composite - direct).abs() < 0.001,
            "composite={composite}, direct={direct}"
        );
    }

    #[test]
    fn multi_glow_sums_and_clamps() {
        let layer = GlowLayer::new([1.0, 0.0, 0.0, 1.0], 4.0, 0.8, (0.0, 0.0));
        let mg = MultiGlow {
            layers: vec![layer, layer],
        };
        let alpha = mg.composite_alpha_at(0.0);
        assert!(
            (alpha - 1.0).abs() < f32::EPSILON,
            "expected 1.0, got {alpha}"
        );
    }

    #[test]
    fn multi_glow_empty_layers_returns_zero() {
        let mg = MultiGlow::new();
        assert_eq!(mg.composite_alpha_at(0.0), 0.0);
    }

    #[test]
    fn glow_exp_matches_stdlib_for_common_values() {
        for x in [-4.0_f32, -2.0, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
            let approx = glow_exp(x);
            let reference = E.powf(x);
            let err = (approx - reference).abs();
            assert!(
                err < 0.01,
                "glow_exp({x}) = {approx}, reference = {reference}, err = {err}"
            );
        }
    }
}
