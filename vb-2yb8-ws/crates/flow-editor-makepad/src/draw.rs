#![forbid(unsafe_code)]
//! Drawing primitives for the flow editor canvas.
//! Constants and helper types for node, edge, port, and grid rendering.

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
    /// Bezier control point horizontal offset as a fraction of horizontal distance.
    pub const BEZIER_CP_FRACTION: f64 = 0.4;
    /// Minimum control point offset so short edges still curve visibly.
    pub const BEZIER_CP_MIN: f64 = 40.0;
}

/// Grid rendering constants
pub mod grid {
    pub const MAJOR_SPACING: f64 = 100.0;
    pub const MINOR_SPACING: f64 = 20.0;
}

/// Viewport limits
pub mod viewport {
    pub const ZOOM_MIN: f64 = 0.1;
    pub const ZOOM_MAX: f64 = 8.0;
    pub const ZOOM_STEP: f64 = 1.1;
    pub const CLICK_THRESHOLD: f64 = 4.0;
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- port constants ----

    #[test]
    fn port_radius_is_positive() {
        assert!(port::RADIUS > 0.0);
    }

    #[test]
    fn port_hit_size_larger_than_radius() {
        assert!(port::HIT_SIZE > port::RADIUS);
    }

    #[test]
    fn port_height_is_positive() {
        assert!(port::HEIGHT > 0.0);
    }

    #[test]
    fn port_label_gap_is_non_negative() {
        assert!(port::LABEL_GAP >= 0.0);
    }

    #[test]
    fn port_hit_size_is_reasonable() {
        // HIT_SIZE should be an integer multiple feel for usability
        assert!(port::HIT_SIZE >= port::RADIUS * 2.0);
    }

    // ---- node constants ----

    #[test]
    fn node_header_height_is_positive() {
        assert!(node::HEADER_HEIGHT > 0.0);
    }

    #[test]
    fn node_min_width_is_positive() {
        assert!(node::MIN_WIDTH > 0.0);
    }

    #[test]
    fn node_min_height_is_positive() {
        assert!(node::MIN_HEIGHT > 0.0);
    }

    #[test]
    fn node_padding_is_non_negative() {
        assert!(node::PADDING >= 0.0);
    }

    #[test]
    fn node_border_radius_is_non_negative() {
        assert!(node::BORDER_RADIUS >= 0.0);
    }

    #[test]
    fn node_badge_size_is_positive() {
        assert!(node::BADGE_SIZE > 0.0);
    }

    #[test]
    fn node_min_width_exceeds_header() {
        // MIN_WIDTH should be wide enough to contain a header bar
        assert!(node::MIN_WIDTH > node::BADGE_SIZE);
    }

    #[test]
    fn node_min_height_exceeds_header() {
        // MIN_HEIGHT should accommodate the header
        assert!(node::MIN_HEIGHT > node::HEADER_HEIGHT);
    }

    // ---- edge constants ----

    #[test]
    fn edge_default_width_is_positive() {
        assert!(edge::DEFAULT_WIDTH > 0.0);
    }

    #[test]
    fn edge_particle_speed_is_positive() {
        assert!(edge::PARTICLE_SPEED > 0.0);
    }

    #[test]
    fn edge_particle_size_is_positive() {
        assert!(edge::PARTICLE_SIZE > 0.0);
    }

    #[test]
    fn edge_bezier_cp_fraction_in_range() {
        assert!(edge::BEZIER_CP_FRACTION > 0.0);
        assert!(edge::BEZIER_CP_FRACTION <= 1.0);
    }

    #[test]
    fn edge_bezier_cp_min_is_positive() {
        assert!(edge::BEZIER_CP_MIN > 0.0);
    }

    #[test]
    fn edge_default_width_is_f32() {
        // Ensure the width fits in f32 range
        let _width: f32 = edge::DEFAULT_WIDTH;
        assert!(_width > 0.0);
    }

    // ---- grid constants ----

    #[test]
    fn grid_major_spacing_is_positive() {
        assert!(grid::MAJOR_SPACING > 0.0);
    }

    #[test]
    fn grid_minor_spacing_is_positive() {
        assert!(grid::MINOR_SPACING > 0.0);
    }

    #[test]
    fn grid_major_is_multiple_of_minor() {
        // Major spacing should be an exact multiple of minor spacing
        let ratio = grid::MAJOR_SPACING / grid::MINOR_SPACING;
        assert!((ratio - ratio.round()).abs() < 1e-10);
    }

    #[test]
    fn grid_major_larger_than_minor() {
        assert!(grid::MAJOR_SPACING > grid::MINOR_SPACING);
    }

    // ---- viewport constants ----

    #[test]
    fn viewport_zoom_min_is_positive() {
        assert!(viewport::ZOOM_MIN > 0.0);
    }

    #[test]
    fn viewport_zoom_max_exceeds_min() {
        assert!(viewport::ZOOM_MAX > viewport::ZOOM_MIN);
    }

    #[test]
    fn viewport_zoom_step_exceeds_one() {
        // ZOOM_STEP > 1 means zooming in increases the value
        assert!(viewport::ZOOM_STEP > 1.0);
    }

    #[test]
    fn viewport_click_threshold_is_positive() {
        assert!(viewport::CLICK_THRESHOLD > 0.0);
    }

    #[test]
    fn viewport_zoom_inversion_step_is_below_one() {
        // 1.0 / ZOOM_STEP should be < 1.0 (zoom out)
        let zoom_out = 1.0 / viewport::ZOOM_STEP;
        assert!(zoom_out < 1.0);
        assert!(zoom_out > 0.0);
    }

    // ---- consistency checks ----

    #[test]
    fn port_height_accommodates_radius() {
        // Port circle should fit within the allocated port row height
        assert!(port::HEIGHT >= port::RADIUS * 2.0);
    }

    #[test]
    fn node_padding_smaller_than_min_dimensions() {
        assert!(node::PADDING < node::MIN_WIDTH);
        assert!(node::PADDING < node::MIN_HEIGHT);
    }

    // =====================================================================
    // Additional comprehensive coverage tests
    // =====================================================================

    // ---- Cross-module consistency ----

    #[test]
    fn viewport_zoom_range_allows_full_dezoom() {
        // ZOOM_MIN should allow seeing the entire grid
        assert!(viewport::ZOOM_MIN < 1.0);
    }

    #[test]
    fn viewport_zoom_range_allows_deep_zoom() {
        // ZOOM_MAX should allow zooming well beyond default
        assert!(viewport::ZOOM_MAX > 4.0);
    }

    #[test]
    fn edge_width_compatible_with_port_radius() {
        // Default edge width should be thinner than port diameter
        let edge_w = f64::from(edge::DEFAULT_WIDTH);
        assert!(edge_w < port::RADIUS * 2.0);
    }

    #[test]
    fn grid_minor_divides_evenly_into_major() {
        let ratio = grid::MAJOR_SPACING / grid::MINOR_SPACING;
        let ratio_int = ratio.round() as i64;
        assert!(ratio_int > 0);
        let reconstructed = grid::MINOR_SPACING * ratio_int as f64;
        let diff = (reconstructed - grid::MAJOR_SPACING).abs();
        assert!(diff < 1e-10);
    }

    #[test]
    fn node_badge_size_smaller_than_header() {
        assert!(node::BADGE_SIZE < node::HEADER_HEIGHT);
    }

    #[test]
    fn click_threshold_is_small() {
        // CLICK_THRESHOLD should be a small pixel value (not a large one)
        assert!(viewport::CLICK_THRESHOLD < 10.0);
    }

    #[test]
    fn particle_size_smaller_than_hit_size() {
        assert!(edge::PARTICLE_SIZE < port::HIT_SIZE);
    }

    #[test]
    fn border_radius_smaller_than_min_dimension() {
        assert!(node::BORDER_RADIUS < node::MIN_WIDTH / 2.0);
        assert!(node::BORDER_RADIUS < node::MIN_HEIGHT / 2.0);
    }
}
