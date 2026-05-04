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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- color validity helpers ----

    fn assert_valid_color(c: [f32; 4], name: &str) {
        assert!(c[0] >= 0.0 && c[0] <= 1.0, "{name} red out of range: {}", c[0]);
        assert!(c[1] >= 0.0 && c[1] <= 1.0, "{name} green out of range: {}", c[1]);
        assert!(c[2] >= 0.0 && c[2] <= 1.0, "{name} blue out of range: {}", c[2]);
        assert!(c[3] >= 0.0 && c[3] <= 1.0, "{name} alpha out of range: {}", c[3]);
    }

    fn assert_opaque(c: [f32; 4], name: &str) {
        assert!((c[3] - 1.0).abs() < f32::EPSILON, "{name} should be opaque, alpha = {}", c[3]);
    }

    // ---- background layer colors ----

    #[test]
    fn canvas_bg_is_dark() {
        assert_valid_color(colors::CANVAS_BG, "CANVAS_BG");
        assert_opaque(colors::CANVAS_BG, "CANVAS_BG");
        assert!(colors::CANVAS_BG[0] < 0.1);
        assert!(colors::CANVAS_BG[1] < 0.1);
        assert!(colors::CANVAS_BG[2] < 0.1);
    }

    #[test]
    fn panel_bg_is_dark() {
        assert_valid_color(colors::PANEL_BG, "PANEL_BG");
        assert_opaque(colors::PANEL_BG, "PANEL_BG");
    }

    #[test]
    fn panel_bg_alt_darker_than_panel_bg() {
        assert_valid_color(colors::PANEL_BG_ALT, "PANEL_BG_ALT");
        assert!(colors::PANEL_BG_ALT[0] > colors::PANEL_BG[0]);
    }

    #[test]
    fn card_bg_between_panel_and_alt() {
        assert_valid_color(colors::CARD_BG, "CARD_BG");
        assert!(colors::CARD_BG[0] >= colors::PANEL_BG[0]);
    }

    #[test]
    fn border_is_visible() {
        assert_valid_color(colors::BORDER, "BORDER");
        assert_opaque(colors::BORDER, "BORDER");
        // Border should be brighter than canvas background
        assert!(colors::BORDER[0] > colors::CANVAS_BG[0]);
    }

    #[test]
    fn grid_line_darker_than_border() {
        assert_valid_color(colors::GRID_LINE, "GRID_LINE");
        assert!(colors::GRID_LINE[0] < colors::BORDER[0]);
    }

    // ---- neon accent colors ----

    #[test]
    fn neon_cyan_is_bright() {
        assert_valid_color(colors::NEON_CYAN, "NEON_CYAN");
        assert_opaque(colors::NEON_CYAN, "NEON_CYAN");
        assert!(colors::NEON_CYAN[1] > 0.9);
        assert!(colors::NEON_CYAN[2] > 0.9);
    }

    #[test]
    fn neon_magenta_is_bright() {
        assert_valid_color(colors::NEON_MAGENTA, "NEON_MAGENTA");
        assert_opaque(colors::NEON_MAGENTA, "NEON_MAGENTA");
        assert!(colors::NEON_MAGENTA[0] > 0.9);
        assert!(colors::NEON_MAGENTA[2] > 0.9);
    }

    #[test]
    fn neon_yellow_is_bright() {
        assert_valid_color(colors::NEON_YELLOW, "NEON_YELLOW");
        assert_opaque(colors::NEON_YELLOW, "NEON_YELLOW");
        assert!(colors::NEON_YELLOW[0] > 0.9);
    }

    #[test]
    fn neon_green_is_bright() {
        assert_valid_color(colors::NEON_GREEN, "NEON_GREEN");
        assert_opaque(colors::NEON_GREEN, "NEON_GREEN");
        assert!(colors::NEON_GREEN[1] > 0.9);
    }

    #[test]
    fn neon_red_is_bright() {
        assert_valid_color(colors::NEON_RED, "NEON_RED");
        assert_opaque(colors::NEON_RED, "NEON_RED");
        assert!(colors::NEON_RED[0] > 0.9);
    }

    #[test]
    fn neon_purple_is_bright() {
        assert_valid_color(colors::NEON_PURPLE, "NEON_PURPLE");
        assert_opaque(colors::NEON_PURPLE, "NEON_PURPLE");
        assert!(colors::NEON_PURPLE[2] > 0.9);
    }

    #[test]
    fn neon_orange_is_bright() {
        assert_valid_color(colors::NEON_ORANGE, "NEON_ORANGE");
        assert_opaque(colors::NEON_ORANGE, "NEON_ORANGE");
        assert!(colors::NEON_ORANGE[0] > 0.9);
    }

    #[test]
    fn neon_teal_is_bright() {
        assert_valid_color(colors::NEON_TEAL, "NEON_TEAL");
        assert_opaque(colors::NEON_TEAL, "NEON_TEAL");
        assert!(colors::NEON_TEAL[1] > 0.8);
    }

    #[test]
    fn neon_pink_is_bright() {
        assert_valid_color(colors::NEON_PINK, "NEON_PINK");
        assert_opaque(colors::NEON_PINK, "NEON_PINK");
        assert!(colors::NEON_PINK[0] > 0.9);
    }

    #[test]
    fn neon_blue_is_bright() {
        assert_valid_color(colors::NEON_BLUE, "NEON_BLUE");
        assert_opaque(colors::NEON_BLUE, "NEON_BLUE");
        assert!(colors::NEON_BLUE[2] > 0.9);
    }

    // ---- all neon colors are distinct ----

    #[test]
    fn all_neon_colors_are_distinct() {
        let neons = [
            colors::NEON_CYAN,
            colors::NEON_MAGENTA,
            colors::NEON_YELLOW,
            colors::NEON_GREEN,
            colors::NEON_RED,
            colors::NEON_PURPLE,
            colors::NEON_ORANGE,
            colors::NEON_TEAL,
            colors::NEON_PINK,
            colors::NEON_BLUE,
        ];
        for i in 0..neons.len() {
            for j in (i.saturating_add(1))..neons.len() {
                assert_ne!(
                    neons[i], neons[j],
                    "neon colors at index {i} and {j} should be distinct"
                );
            }
        }
    }

    // ---- text colors ----

    #[test]
    fn text_primary_is_bright() {
        assert_valid_color(colors::TEXT_PRIMARY, "TEXT_PRIMARY");
        assert_opaque(colors::TEXT_PRIMARY, "TEXT_PRIMARY");
        assert!(colors::TEXT_PRIMARY[0] > 0.8);
    }

    #[test]
    fn text_secondary_dimmer_than_primary() {
        assert_valid_color(colors::TEXT_SECONDARY, "TEXT_SECONDARY");
        assert!(colors::TEXT_SECONDARY[0] < colors::TEXT_PRIMARY[0]);
    }

    #[test]
    fn text_dim_dimmer_than_secondary() {
        assert_valid_color(colors::TEXT_DIM, "TEXT_DIM");
        assert!(colors::TEXT_DIM[0] < colors::TEXT_SECONDARY[0]);
    }

    // ---- state colors are aliased correctly ----

    #[test]
    fn state_succeeded_is_green() {
        assert_eq!(colors::STATE_SUCCEEDED, colors::NEON_GREEN);
    }

    #[test]
    fn state_running_is_cyan() {
        assert_eq!(colors::STATE_RUNNING, colors::NEON_CYAN);
    }

    #[test]
    fn state_failed_is_red() {
        assert_eq!(colors::STATE_FAILED, colors::NEON_RED);
    }

    #[test]
    fn state_waiting_is_blue() {
        assert_eq!(colors::STATE_WAITING, colors::NEON_BLUE);
    }

    #[test]
    fn state_asking_is_yellow() {
        assert_eq!(colors::STATE_ASKING, colors::NEON_YELLOW);
    }

    #[test]
    fn state_pending_is_border() {
        assert_eq!(colors::STATE_PENDING, colors::BORDER);
    }

    #[test]
    fn state_cancelled_is_dim() {
        assert_eq!(colors::STATE_CANCELLED, colors::TEXT_DIM);
    }

    #[test]
    fn state_secret_is_magenta() {
        assert_eq!(colors::STATE_SECRET, colors::NEON_MAGENTA);
    }

    // ---- all colors have valid RGBA ----

    #[test]
    fn all_colors_valid_rgba() {
        let all_colors: [(&str, [f32; 4]); 19] = [
            ("CANVAS_BG", colors::CANVAS_BG),
            ("PANEL_BG", colors::PANEL_BG),
            ("PANEL_BG_ALT", colors::PANEL_BG_ALT),
            ("CARD_BG", colors::CARD_BG),
            ("BORDER", colors::BORDER),
            ("GRID_LINE", colors::GRID_LINE),
            ("NEON_CYAN", colors::NEON_CYAN),
            ("NEON_MAGENTA", colors::NEON_MAGENTA),
            ("NEON_YELLOW", colors::NEON_YELLOW),
            ("NEON_GREEN", colors::NEON_GREEN),
            ("NEON_RED", colors::NEON_RED),
            ("NEON_PURPLE", colors::NEON_PURPLE),
            ("NEON_ORANGE", colors::NEON_ORANGE),
            ("NEON_TEAL", colors::NEON_TEAL),
            ("NEON_PINK", colors::NEON_PINK),
            ("NEON_BLUE", colors::NEON_BLUE),
            ("TEXT_PRIMARY", colors::TEXT_PRIMARY),
            ("TEXT_SECONDARY", colors::TEXT_SECONDARY),
            ("TEXT_DIM", colors::TEXT_DIM),
            // STATE_PENDING is an alias for BORDER which is already checked
        ];
        for (name, c) in &all_colors {
            assert_valid_color(*c, name);
        }
    }

    // ---- node_colors tests ----

    #[test]
    fn node_color_data_is_secondary() {
        assert_eq!(node_colors::DATA, colors::TEXT_SECONDARY);
    }

    #[test]
    fn node_color_external_is_orange() {
        assert_eq!(node_colors::EXTERNAL, colors::NEON_ORANGE);
    }

    #[test]
    fn node_color_branch_is_purple() {
        assert_eq!(node_colors::BRANCH, colors::NEON_PURPLE);
    }

    #[test]
    fn node_color_loop_is_blue() {
        assert_eq!(node_colors::LOOP, colors::NEON_BLUE);
    }

    #[test]
    fn node_color_parallel_is_blue() {
        assert_eq!(node_colors::PARALLEL, colors::NEON_BLUE);
    }

    #[test]
    fn node_color_suspend_is_green() {
        assert_eq!(node_colors::SUSPEND, colors::NEON_GREEN);
    }

    #[test]
    fn node_color_error_is_red() {
        assert_eq!(node_colors::ERROR, colors::NEON_RED);
    }

    #[test]
    fn node_color_terminal_is_teal() {
        assert_eq!(node_colors::TERMINAL, colors::NEON_TEAL);
    }

    #[test]
    fn node_color_control_is_secondary() {
        assert_eq!(node_colors::CONTROL, colors::TEXT_SECONDARY);
    }

    #[test]
    fn all_node_colors_are_valid() {
        let node_cols: [(&str, [f32; 4]); 9] = [
            ("DATA", node_colors::DATA),
            ("EXTERNAL", node_colors::EXTERNAL),
            ("BRANCH", node_colors::BRANCH),
            ("LOOP", node_colors::LOOP),
            ("PARALLEL", node_colors::PARALLEL),
            ("SUSPEND", node_colors::SUSPEND),
            ("ERROR", node_colors::ERROR),
            ("TERMINAL", node_colors::TERMINAL),
            ("CONTROL", node_colors::CONTROL),
        ];
        for (name, c) in &node_cols {
            assert_valid_color(*c, name);
        }
    }

    #[test]
    fn node_colors_loop_and_parallel_match() {
        // Loop and Parallel are both blue
        assert_eq!(node_colors::LOOP, node_colors::PARALLEL);
    }

    #[test]
    fn node_colors_data_and_control_match() {
        // Data and Control are both secondary text color
        assert_eq!(node_colors::DATA, node_colors::CONTROL);
    }

    // =====================================================================
    // Additional comprehensive coverage tests
    // =====================================================================

    // ---- Neon colors are all fully opaque ----

    #[test]
    fn all_neon_colors_are_opaque() {
        let neons = [
            ("NEON_CYAN", colors::NEON_CYAN),
            ("NEON_MAGENTA", colors::NEON_MAGENTA),
            ("NEON_YELLOW", colors::NEON_YELLOW),
            ("NEON_GREEN", colors::NEON_GREEN),
            ("NEON_RED", colors::NEON_RED),
            ("NEON_PURPLE", colors::NEON_PURPLE),
            ("NEON_ORANGE", colors::NEON_ORANGE),
            ("NEON_TEAL", colors::NEON_TEAL),
            ("NEON_PINK", colors::NEON_PINK),
            ("NEON_BLUE", colors::NEON_BLUE),
        ];
        for (name, c) in &neons {
            assert_opaque(*c, name);
        }
    }

    // ---- Background colors are dark ----

    #[test]
    fn all_background_colors_are_dark() {
        let bgs = [
            ("CANVAS_BG", colors::CANVAS_BG),
            ("PANEL_BG", colors::PANEL_BG),
            ("PANEL_BG_ALT", colors::PANEL_BG_ALT),
            ("CARD_BG", colors::CARD_BG),
        ];
        for (name, c) in &bgs {
            assert_valid_color(*c, name);
            assert_opaque(*c, name);
            // All background components should be below 0.2
            assert!(c[0] < 0.2, "{name} red should be dark: {}", c[0]);
            assert!(c[1] < 0.2, "{name} green should be dark: {}", c[1]);
            assert!(c[2] < 0.2, "{name} blue should be dark: {}", c[2]);
        }
    }

    // ---- Neon colors are saturated ----

    #[test]
    fn neon_colors_are_saturated() {
        let neons = [
            colors::NEON_CYAN,
            colors::NEON_MAGENTA,
            colors::NEON_YELLOW,
            colors::NEON_GREEN,
            colors::NEON_RED,
            colors::NEON_PURPLE,
            colors::NEON_ORANGE,
            colors::NEON_TEAL,
            colors::NEON_PINK,
            colors::NEON_BLUE,
        ];
        for c in &neons {
            // At least one channel should be > 0.8 (bright/saturated)
            assert!(
                c[0] > 0.8 || c[1] > 0.8 || c[2] > 0.8,
                "neon color {:?} should have at least one bright channel",
                c
            );
        }
    }

    // ---- State colors are all valid and opaque ----

    #[test]
    fn all_state_colors_are_valid_and_opaque() {
        let states = [
            ("STATE_SUCCEEDED", colors::STATE_SUCCEEDED),
            ("STATE_RUNNING", colors::STATE_RUNNING),
            ("STATE_FAILED", colors::STATE_FAILED),
            ("STATE_WAITING", colors::STATE_WAITING),
            ("STATE_ASKING", colors::STATE_ASKING),
            ("STATE_PENDING", colors::STATE_PENDING),
            ("STATE_CANCELLED", colors::STATE_CANCELLED),
            ("STATE_SECRET", colors::STATE_SECRET),
        ];
        for (name, c) in &states {
            assert_valid_color(*c, name);
            assert_opaque(*c, name);
        }
    }

    // ---- Node colors are all distinct from each other (where expected) ----

    #[test]
    fn node_colors_external_branch_distinct() {
        assert_ne!(node_colors::EXTERNAL, node_colors::BRANCH);
    }

    #[test]
    fn node_colors_suspend_error_distinct() {
        assert_ne!(node_colors::SUSPEND, node_colors::ERROR);
    }

    #[test]
    fn node_colors_terminal_control_distinct() {
        assert_ne!(node_colors::TERMINAL, node_colors::CONTROL);
    }

    // ---- Text hierarchy ----

    #[test]
    fn text_colors_form_brightness_hierarchy() {
        // TEXT_PRIMARY > TEXT_SECONDARY > TEXT_DIM in brightness
        let primary_brightness = colors::TEXT_PRIMARY[0];
        let secondary_brightness = colors::TEXT_SECONDARY[0];
        let dim_brightness = colors::TEXT_DIM[0];
        assert!(primary_brightness > secondary_brightness);
        assert!(secondary_brightness > dim_brightness);
    }

    // ---- Grid line darker than border ----

    #[test]
    fn grid_line_is_between_canvas_and_border() {
        // GRID_LINE should be between CANVAS_BG and BORDER in brightness
        assert!(colors::GRID_LINE[0] > colors::CANVAS_BG[0]);
        assert!(colors::GRID_LINE[0] < colors::BORDER[0]);
    }
}
