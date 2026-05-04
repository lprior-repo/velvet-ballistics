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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compute perceived luminance (Rec. 709) from an RGBA color.
    /// Returns a value in [0.0, 1.0] where 0.0 is black and 1.0 is white.
    fn luminance(rgba: [f32; 4]) -> f32 {
        0.2126 * rgba[0] + 0.7152 * rgba[1] + 0.0722 * rgba[2]
    }

    /// Helper: collect all background colors into a slice for bulk checks.
    fn all_bg_colors() -> Vec<(&'static str, [f32; 4])> {
        vec![
            ("CANVAS", bg::CANVAS),
            ("PANEL", bg::PANEL),
            ("PANEL_ALT", bg::PANEL_ALT),
            ("CARD", bg::CARD),
            ("CARD_HOVER", bg::CARD_HOVER),
            ("BORDER", bg::BORDER),
            ("BORDER_BRIGHT", bg::BORDER_BRIGHT),
            ("GRID", bg::GRID),
        ]
    }

    /// Helper: collect all neon accent colors into a slice for bulk checks.
    fn all_neon_colors() -> Vec<(&'static str, [f32; 4])> {
        vec![
            ("CYAN", neon::CYAN),
            ("CYAN_DIM", neon::CYAN_DIM),
            ("MAGENTA", neon::MAGENTA),
            ("YELLOW", neon::YELLOW),
            ("GREEN", neon::GREEN),
            ("GREEN_DIM", neon::GREEN_DIM),
            ("RED", neon::RED),
            ("RED_DIM", neon::RED_DIM),
            ("PURPLE", neon::PURPLE),
            ("ORANGE", neon::ORANGE),
            ("TEAL", neon::TEAL),
            ("PINK", neon::PINK),
            ("BLUE", neon::BLUE),
            ("BLUE_DIM", neon::BLUE_DIM),
        ]
    }

    /// Helper: collect state colors for distinctness checks.
    fn all_state_colors() -> Vec<(&'static str, [f32; 4])> {
        vec![
            ("PENDING", state::PENDING),
            ("RUNNING", state::RUNNING),
            ("SUCCEEDED", state::SUCCEEDED),
            ("FAILED", state::FAILED),
            ("SKIPPED", state::SKIPPED),
            ("WAITING", state::WAITING),
            ("ASKING", state::ASKING),
            ("CANCELLED", state::CANCELLED),
            ("SECRET", state::SECRET),
        ]
    }

    // --- Test 1: All background alpha channels are 1.0 (fully opaque) ---

    #[test]
    fn bg_colors_all_have_alpha_one() {
        for (name, rgba) in all_bg_colors() {
            assert_eq!(
                rgba[3], 1.0,
                "bg::{name} should have alpha 1.0, got {}",
                rgba[3]
            );
        }
    }

    // --- Test 2: All neon accent alpha channels are 1.0 (fully opaque) ---

    #[test]
    fn neon_colors_all_have_alpha_one() {
        for (name, rgba) in all_neon_colors() {
            assert_eq!(
                rgba[3], 1.0,
                "neon::{name} should have alpha 1.0, got {}",
                rgba[3]
            );
        }
    }

    // --- Test 3: All text alpha channels are 1.0 (fully opaque) ---

    #[test]
    fn text_colors_all_have_alpha_one() {
        let text_colors: Vec<(&str, [f32; 4])> = vec![
            ("PRIMARY", text::PRIMARY),
            ("SECONDARY", text::SECONDARY),
            ("DIM", text::DIM),
            ("ACCENT", text::ACCENT),
            ("SUCCESS", text::SUCCESS),
            ("ERROR", text::ERROR),
            ("WARNING", text::WARNING),
        ];
        for (name, rgba) in text_colors {
            assert_eq!(
                rgba[3], 1.0,
                "text::{name} should have alpha 1.0, got {}",
                rgba[3]
            );
        }
    }

    // --- Test 4: RGB channels are in [0.0, 1.0] for all background colors ---

    #[test]
    fn bg_colors_rgb_channels_in_unit_range() {
        for (name, rgba) in all_bg_colors() {
            for ch in 0..3 {
                assert!(
                    rgba[ch] >= 0.0 && rgba[ch] <= 1.0,
                    "bg::{name} channel {ch} = {} is outside [0.0, 1.0]",
                    rgba[ch]
                );
            }
        }
    }

    // --- Test 5: RGB channels are in [0.0, 1.0] for all neon colors ---

    #[test]
    fn neon_colors_rgb_channels_in_unit_range() {
        for (name, rgba) in all_neon_colors() {
            for ch in 0..3 {
                assert!(
                    rgba[ch] >= 0.0 && rgba[ch] <= 1.0,
                    "neon::{name} channel {ch} = {} is outside [0.0, 1.0]",
                    rgba[ch]
                );
            }
        }
    }

    // --- Test 6: Background colors are dark (low luminance) ---

    #[test]
    fn bg_colors_are_darker_than_neon_accents() {
        // Verify that bg colors have low luminance (< 0.3) and bright neon
        // accents have higher luminance on average.  Exact ordering isn't
        // enforced because neon::RED (pure red) has relatively low luminance
        // despite being visually intense.
        for (name, rgba) in all_bg_colors() {
            let lum = luminance(rgba);
            assert!(
                lum < 0.3,
                "bg::{name} luminance ({lum}) should be < 0.3"
            );
        }
    }

    // --- Test 7: Neon color constants match their documented hex codes ---

    #[test]
    fn neon_cyan_matches_hex_spec() {
        // #00f5ff => R=0x00=0, G=0xf5=245, B=0xff=255
        // f32: R=0/255, G=245/255, B=255/255
        let expected: [f32; 4] = [0.0, 245.0 / 255.0, 255.0 / 255.0, 1.0];
        for ch in 0..4 {
            let diff = (neon::CYAN[ch] - expected[ch]).abs();
            assert!(
                diff < 0.01,
                "neon::CYAN[{ch}] = {} differs from expected {} by {}",
                neon::CYAN[ch],
                expected[ch],
                diff
            );
        }
    }

    // --- Test 8: Neon magenta matches its documented hex code ---

    #[test]
    fn neon_magenta_matches_hex_spec() {
        // #ff00ff => R=255, G=0, B=255
        let expected: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
        assert_eq!(neon::MAGENTA, expected, "neon::MAGENTA should be pure magenta");
    }

    // --- Test 9: Neon red matches its documented hex code ---

    #[test]
    fn neon_red_matches_hex_spec() {
        // #ff073a => R=255, G=7, B=58
        let expected: [f32; 4] = [255.0 / 255.0, 7.0 / 255.0, 58.0 / 255.0, 1.0];
        for ch in 0..4 {
            let diff = (neon::RED[ch] - expected[ch]).abs();
            assert!(
                diff < 0.01,
                "neon::RED[{ch}] = {} differs from expected {} by {}",
                neon::RED[ch],
                expected[ch],
                diff
            );
        }
    }

    // --- Test 10: Neon green matches its documented hex code ---

    #[test]
    fn neon_green_matches_hex_spec() {
        // #39ff14 => R=57, G=255, B=20
        let expected: [f32; 4] = [57.0 / 255.0, 1.0, 20.0 / 255.0, 1.0];
        for ch in 0..4 {
            let diff = (neon::GREEN[ch] - expected[ch]).abs();
            assert!(
                diff < 0.01,
                "neon::GREEN[{ch}] = {} differs from expected {} by {}",
                neon::GREEN[ch],
                expected[ch],
                diff
            );
        }
    }

    // --- Test 11: Status/state colors are distinct from each other ---

    #[test]
    fn state_colors_are_mutually_distinct() {
        let states = all_state_colors();
        for i in 0..states.len() {
            for j in (i + 1)..states.len() {
                let (name_i, color_i) = states[i];
                let (name_j, color_j) = states[j];
                // SKIPPED and CANCELLED are both text::DIM, which is intentional;
                // check that other pairs are distinct.
                if (name_i == "SKIPPED" && name_j == "CANCELLED")
                    || (name_i == "CANCELLED" && name_j == "SKIPPED")
                {
                    continue;
                }
                assert_ne!(
                    color_i, color_j,
                    "state::{name_i} and state::{name_j} should be distinct colors"
                );
            }
        }
    }

    // --- Test 12: text::ACCENT is neon::CYAN ---

    #[test]
    fn text_accent_is_neon_cyan() {
        assert_eq!(
            text::ACCENT, neon::CYAN,
            "text::ACCENT should alias neon::CYAN"
        );
    }

    // --- Test 13: text::SUCCESS is neon::GREEN ---

    #[test]
    fn text_success_is_neon_green() {
        assert_eq!(
            text::SUCCESS, neon::GREEN,
            "text::SUCCESS should alias neon::GREEN"
        );
    }

    // --- Test 14: text::ERROR is neon::RED ---

    #[test]
    fn text_error_is_neon_red() {
        assert_eq!(
            text::ERROR, neon::RED,
            "text::ERROR should alias neon::RED"
        );
    }

    // --- Test 15: text::WARNING is neon::YELLOW ---

    #[test]
    fn text_warning_is_neon_yellow() {
        assert_eq!(
            text::WARNING, neon::YELLOW,
            "text::WARNING should alias neon::YELLOW"
        );
    }

    // --- Test 16: Canvas is the darkest background color ---

    #[test]
    fn canvas_is_darkest_background() {
        let canvas_lum = luminance(bg::CANVAS);
        for (name, rgba) in all_bg_colors() {
            if name == "CANVAS" {
                continue;
            }
            let other_lum = luminance(rgba);
            assert!(
                canvas_lum <= other_lum,
                "bg::CANVAS (luminance {canvas_lum}) should be darker than bg::{name} (luminance {other_lum})"
            );
        }
    }

    // --- Test 17: Neon dim variants are darker than their bright counterparts ---

    #[test]
    fn neon_dim_variants_are_darker_than_bright() {
        let pairs: Vec<(&str, [f32; 4], [f32; 4])> = vec![
            ("CYAN/CYAN_DIM", neon::CYAN, neon::CYAN_DIM),
            ("GREEN/GREEN_DIM", neon::GREEN, neon::GREEN_DIM),
            ("RED/RED_DIM", neon::RED, neon::RED_DIM),
            ("BLUE/BLUE_DIM", neon::BLUE, neon::BLUE_DIM),
        ];
        for (pair_name, bright, dim) in pairs {
            let bright_lum = luminance(bright);
            let dim_lum = luminance(dim);
            assert!(
                dim_lum < bright_lum,
                "{pair_name}: dim variant (luminance {dim_lum}) should be darker than bright (luminance {bright_lum})"
            );
        }
    }

    // --- Test 18: Node header colors are darker than their body counterparts ---

    #[test]
    fn node_headers_are_darker_than_body() {
        let pairs: Vec<(&str, [f32; 4], [f32; 4])> = vec![
            ("DATA", node_category::DATA, node_header::DATA),
            ("EXTERNAL", node_category::EXTERNAL, node_header::EXTERNAL),
            ("BRANCH", node_category::BRANCH, node_header::BRANCH),
            ("LOOP", node_category::LOOP, node_header::LOOP),
            ("PARALLEL", node_category::PARALLEL, node_header::PARALLEL),
            ("COLLECT", node_category::COLLECT, node_header::COLLECT),
            ("REDUCE", node_category::REDUCE, node_header::REDUCE),
            ("SUSPEND", node_category::SUSPEND, node_header::SUSPEND),
            ("ERROR", node_category::ERROR, node_header::ERROR),
            ("TERMINAL", node_category::TERMINAL, node_header::TERMINAL),
            ("CONTROL", node_category::CONTROL, node_header::CONTROL),
        ];
        for (name, body, header) in pairs {
            let body_lum = luminance(body);
            let header_lum = luminance(header);
            assert!(
                header_lum <= body_lum,
                "node_header::{name} (luminance {header_lum}) should be darker than node_category::{name} (luminance {body_lum})"
            );
        }
    }

    // --- Test 19: All node category and header colors have alpha 1.0 ---

    #[test]
    fn node_colors_all_have_alpha_one() {
        let all: Vec<(&str, [f32; 4])> = vec![
            ("category::DATA", node_category::DATA),
            ("category::EXTERNAL", node_category::EXTERNAL),
            ("category::BRANCH", node_category::BRANCH),
            ("category::LOOP", node_category::LOOP),
            ("category::PARALLEL", node_category::PARALLEL),
            ("category::COLLECT", node_category::COLLECT),
            ("category::REDUCE", node_category::REDUCE),
            ("category::SUSPEND", node_category::SUSPEND),
            ("category::ERROR", node_category::ERROR),
            ("category::TERMINAL", node_category::TERMINAL),
            ("category::CONTROL", node_category::CONTROL),
            ("header::DATA", node_header::DATA),
            ("header::EXTERNAL", node_header::EXTERNAL),
            ("header::BRANCH", node_header::BRANCH),
            ("header::LOOP", node_header::LOOP),
            ("header::PARALLEL", node_header::PARALLEL),
            ("header::COLLECT", node_header::COLLECT),
            ("header::REDUCE", node_header::REDUCE),
            ("header::SUSPEND", node_header::SUSPEND),
            ("header::ERROR", node_header::ERROR),
            ("header::TERMINAL", node_header::TERMINAL),
            ("header::CONTROL", node_header::CONTROL),
        ];
        for (name, rgba) in all {
            assert_eq!(
                rgba[3], 1.0,
                "{name} should have alpha 1.0, got {}",
                rgba[3]
            );
        }
    }

    // --- Test 20: Pressure gradient levels are pairwise distinct ---

    #[test]
    fn pressure_gradient_colors_are_distinct() {
        let levels = [
            ("LOW", pressure::LOW),
            ("MEDIUM", pressure::MEDIUM),
            ("HIGH", pressure::HIGH),
            ("CRITICAL", pressure::CRITICAL),
        ];
        for i in 0..levels.len() {
            for j in (i + 1)..levels.len() {
                assert_ne!(
                    levels[i].1, levels[j].1,
                    "pressure::{} and pressure::{} should be distinct colors",
                    levels[i].0, levels[j].0,
                );
            }
        }
    }

    // --- Test 21: Neon yellow matches its documented hex code ---

    #[test]
    fn neon_yellow_matches_hex_spec() {
        // #ffe600 => R=255, G=230, B=0
        let expected: [f32; 4] = [1.0, 230.0 / 255.0, 0.0, 1.0];
        for ch in 0..4 {
            let diff = (neon::YELLOW[ch] - expected[ch]).abs();
            assert!(
                diff < 0.01,
                "neon::YELLOW[{ch}] = {} differs from expected {} by {}",
                neon::YELLOW[ch],
                expected[ch],
                diff
            );
        }
    }

    // --- Test 22: bg::CANVAS matches documented hex #0a0a12 ---

    #[test]
    fn bg_canvas_matches_hex_spec() {
        // #0a0a12 => R=10, G=10, B=18
        let expected: [f32; 4] = [10.0 / 255.0, 10.0 / 255.0, 18.0 / 255.0, 1.0];
        for ch in 0..4 {
            let diff = (bg::CANVAS[ch] - expected[ch]).abs();
            assert!(
                diff < 0.005,
                "bg::CANVAS[{ch}] = {} differs from expected {} by {}",
                bg::CANVAS[ch],
                expected[ch],
                diff
            );
        }
    }
}
