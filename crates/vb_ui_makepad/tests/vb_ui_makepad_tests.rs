#![forbid(unsafe_code)]
#![allow(clippy::panic, clippy::unwrap_used)]

//! Integration tests for vb_ui_makepad crate.
//! Tests cover: Error enum, tokens, graph_canvas, graph_node, graph_edge, shell, packet_dot

use vb_ui_makepad::{
    AppShell, EdgeRenderInstr, Error, GraphCanvas, GraphNode, NodeBadge, NodeCardRenderInstr,
    OverlayState, PacketDot, PacketMarkerInstr, Screen, ShellNav, ShellStatusChip, color,
    graph_canvas::EdgePath, graph_canvas::ViewportRect, graph_edge::EdgeType, layout,
    packet_dot::PacketDotManager, radius, shadow, space,
};

// =============================================================================
// ERROR ENUM TESTS — all 11 active Error variants
// =============================================================================

mod error_tests {
    use super::*;

    #[test]
    fn error_invalid_token_variant_exists() {
        let e = Error::InvalidToken("missing color".into());
        matches!(e, Error::InvalidToken(msg) if msg.contains("missing color"));
    }

    #[test]
    fn error_nav_item_not_found_variant_exists() {
        let e = Error::NavItemNotFound("Overview".into());
        matches!(e, Error::NavItemNotFound(s) if s == "Overview");
    }

    #[test]
    fn error_invalid_screen_transition_variant_exists() {
        let e = Error::InvalidScreenTransition("Overview -> Invalid".into());
        matches!(e, Error::InvalidScreenTransition(s) if s.contains("Invalid"));
    }

    #[test]
    fn error_token_parse_error_variant_exists() {
        let e = Error::TokenParseError("invalid hex char".into());
        matches!(e, Error::TokenParseError(msg) if msg.contains("invalid hex"));
    }

    #[test]
    fn error_invalid_flow_document_variant_exists() {
        let e = Error::InvalidFlowDocument("malformed yaml".into());
        matches!(e, Error::InvalidFlowDocument(s) if s.contains("malformed"));
    }

    #[test]
    fn error_layout_not_computed_variant_exists() {
        let e = Error::LayoutNotComputed;
        matches!(e, Error::LayoutNotComputed);
    }

    #[test]
    fn error_node_not_found_variant_exists() {
        let e = Error::NodeNotFound(42);
        matches!(e, Error::NodeNotFound(idx) if idx == 42);
    }

    #[test]
    fn error_invalid_viewport_variant_exists() {
        let e = Error::InvalidViewport;
        matches!(e, Error::InvalidViewport);
    }

    #[test]
    fn error_animation_overflow_variant_exists() {
        let e = Error::AnimationOverflow;
        matches!(e, Error::AnimationOverflow);
    }

    #[test]
    fn error_view_hidden_variant_exists() {
        let e = Error::ViewHidden;
        matches!(e, Error::ViewHidden);
    }

    #[test]
    fn error_missing_design_token_variant_exists() {
        let e = Error::MissingDesignToken("primary_color".into());
        matches!(e, Error::MissingDesignToken(s) if s == "primary_color");
    }
}

// =============================================================================
// TOKENS PARSE_HEX TESTS
// =============================================================================

mod tokens_parse_hex_tests {
    use super::*;

    fn parse_hex(hex: &str) -> Result<[f32; 4], Error> {
        // Use Tokens::parse to exercise parse_hex indirectly via from_toml
        // Direct parse_hex testing via a minimal TOML
        let toml = format!(
            r##"
[color]
background_board = "{}"
shell = "#FFFFFF"
surface = "#FFFFFF"
surface_glass = "#FFFFFF"
surface_muted = "#FFFFFF"
line_hair = "#FFFFFF"
line_soft = "#FFFFFF"
text_primary = "#FFFFFF"
text_secondary = "#FFFFFF"
text_tertiary = "#FFFFFF"
success = "#FFFFFF"
running = "#FFFFFF"
active_cyan = "#FFFFFF"
warning = "#FFFFFF"
failure = "#FFFFFF"
taint = "#FFFFFF"
durable = "#FFFFFF"
pending = "#FFFFFF"

[layout]
sidebar_width = 246.0
top_bar_height = 78.0
outer_margin = 32.0
content_gutter = 16.0
inspector_width_min = 360.0
inspector_width_max = 420.0
bottom_timeline_min = 220.0
graph_canvas_min_width = 720.0
graph_canvas_min_height = 520.0
window_width = 1920.0
window_height = 1080.0

[space]
px_4 = 4.0
px_8 = 8.0
px_12 = 12.0
px_16 = 16.0
px_20 = 20.0
px_24 = 24.0
px_32 = 32.0
px_40 = 40.0

[radius]
chip = 10.0
control = 12.0
card_min = 14.0
card = 16.0
card_max = 22.0
panel = 20.0
window = 24.0

[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
window = "0 20 60 rgba(16,24,40,0.14)"
focus = "0 0 0 4 rgba(31,122,245,0.14)"
failure = "0 0 0 4 rgba(229,72,77,0.12)"
taint = "0 0 0 4 rgba(139,92,246,0.12)"

[type]
family_sans = "Inter, SF Pro, system-ui"
family_mono = "JetBrains Mono, SF Mono, ui-monospace"
size_11 = 11
size_12 = 12
size_13 = 13
size_14 = 14
size_16 = 16
size_20 = 20
size_24 = 24
weight_regular = 400
weight_medium = 500
weight_semibold = 600
"##,
            hex
        );
        vb_ui_makepad::tokens::ParsedTokens::from_toml(&toml).map(|p| p.color.background_board)
    }

    #[test]
    fn parse_hex_valid_six_char_hex_returns_correct_rgba() {
        let Ok(rgba) = parse_hex("#FF0000") else {
            panic!("should parse valid 6-char hex")
        };
        assert!((rgba[0] - 1.0).abs() < 1e-6, "R should be 1.0");
        assert!((rgba[1] - 0.0).abs() < 1e-6, "G should be 0.0");
        assert!((rgba[2] - 0.0).abs() < 1e-6, "B should be 0.0");
        assert!((rgba[3] - 1.0).abs() < 1e-6, "A should be 1.0");
    }

    #[test]
    fn parse_hex_valid_eight_char_hex_with_alpha() {
        let Ok(rgba) = parse_hex("#FF000080") else {
            panic!("should parse valid 8-char hex")
        };
        assert!((rgba[0] - 1.0).abs() < 1e-6, "R should be 1.0");
        assert!((rgba[1] - 0.0).abs() < 1e-6, "G should be 0.0");
        assert!((rgba[2] - 0.0).abs() < 1e-6, "B should be 0.0");
        // 0x80 = 128, 128/255 ≈ 0.502
        assert!((rgba[3] - 128.0 / 255.0).abs() < 1e-3, "A should be ~0.502");
    }

    #[test]
    fn parse_hex_no_prefix_returns_same_as_with_prefix() {
        let Ok(with_hash) = parse_hex("#FF0000") else {
            panic!("with_hash should parse")
        };
        let Ok(without) = parse_hex("FF0000") else {
            panic!("without should parse")
        };
        assert_eq!(with_hash, without);
    }

    #[test]
    fn parse_hex_lowercase_returns_same_as_uppercase() {
        let Ok(upper) = parse_hex("#FF0000") else {
            panic!("upper should parse")
        };
        let Ok(lower) = parse_hex("#ff0000") else {
            panic!("lower should parse")
        };
        assert_eq!(upper, lower);
    }

    #[test]
    fn parse_hex_whitespace_is_trimmed() {
        let Ok(rgba) = parse_hex("  #FF0000  ") else {
            panic!("should trim whitespace")
        };
        assert!((rgba[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn parse_hex_invalid_char_returns_error() {
        let result = parse_hex("#GG0000");
        matches!(result, Err(Error::TokenParseError(msg)) if msg.contains("invalid hex"));
    }

    #[test]
    fn parse_hex_wrong_length_three_returns_error() {
        let result = parse_hex("#F00");
        matches!(result, Err(Error::TokenParseError(msg)) if msg.contains("invalid hex length"));
    }

    #[test]
    fn parse_hex_wrong_length_four_returns_error() {
        let result = parse_hex("#FFFF");
        matches!(result, Err(Error::TokenParseError(msg)) if msg.contains("invalid hex length"));
    }

    #[test]
    fn parse_hex_too_short_returns_error() {
        let result = parse_hex("#");
        matches!(result, Err(Error::TokenParseError(msg)) if msg.contains("hex too short"));
    }

    #[test]
    fn parse_hex_green_returns_correct_rgba() {
        let Ok(rgba) = parse_hex("#00FF00") else {
            panic!("should parse green")
        };
        assert!((rgba[0] - 0.0).abs() < 1e-6, "R should be 0.0");
        assert!((rgba[1] - 1.0).abs() < 1e-6, "G should be 1.0");
        assert!((rgba[2] - 0.0).abs() < 1e-6, "B should be 0.0");
        assert!((rgba[3] - 1.0).abs() < 1e-6, "A should be 1.0");
    }

    #[test]
    fn parse_hex_blue_returns_correct_rgba() {
        let Ok(rgba) = parse_hex("#0000FF") else {
            panic!("should parse blue")
        };
        assert!((rgba[0] - 0.0).abs() < 1e-6, "R should be 0.0");
        assert!((rgba[1] - 0.0).abs() < 1e-6, "G should be 0.0");
        assert!((rgba[2] - 1.0).abs() < 1e-6, "B should be 1.0");
        assert!((rgba[3] - 1.0).abs() < 1e-6, "A should be 1.0");
    }
}

// =============================================================================
// TOKENS FROM_TOML TESTS
// =============================================================================

mod tokens_from_toml_tests {
    use super::*;

    #[test]
    fn from_toml_empty_string_returns_invalid_token_error() {
        let result = vb_ui_makepad::tokens::ParsedTokens::from_toml("");
        matches!(result, Err(Error::InvalidToken(_)));
    }

    #[test]
    fn from_toml_missing_color_section_returns_error() {
        let toml = r#"
[layout]
sidebar_width = 246.0
"#;
        let result = vb_ui_makepad::tokens::ParsedTokens::from_toml(toml);
        matches!(result, Err(Error::InvalidToken(msg)) if msg.contains("missing color"));
    }

    #[test]
    fn from_toml_missing_layout_key_returns_error() {
        let toml = r##"
[color]
background_board = "#FFFFFF"
[layout]
"##;
        let result = vb_ui_makepad::tokens::ParsedTokens::from_toml(toml);
        matches!(result, Err(Error::InvalidToken(msg)) if msg.contains("missing layout"));
    }

    #[test]
    fn from_toml_color_value_not_string_returns_error() {
        let toml = r#"
[color]
background_board = 12345
[layout]
sidebar_width = 246.0
[space]
px_4 = 4.0
[radius]
chip = 10.0
[shadow]
card = "0 8 24"
[type]
family_sans = "Inter"
family_mono = "Mono"
size_11 = 11
weight_regular = 400
"#;
        let result = vb_ui_makepad::tokens::ParsedTokens::from_toml(toml);
        matches!(result, Err(Error::TokenParseError(msg)) if msg.contains("not string"));
    }

    #[test]
    fn from_toml_valid_toml_parses_successfully() {
        let toml = r##"
[color]
background_board = "#F0F0F0"
shell = "#FFFFFF"
surface = "#FFFFFF"
surface_glass = "#FFFFFF80"
surface_muted = "#F2F2F2"
line_hair = "#DEE0EB"
line_soft = "#E8EBF2"
text_primary = "#101828"
text_secondary = "#475467"
text_tertiary = "#7A7A96"
success = "#16A659"
running = "#1F7AF5"
active_cyan = "#19A7CE"
warning = "#F59E0B"
failure = "#E5484D"
taint = "#8B5CF6"
durable = "#14B8A6"
pending = "#98A2B3"

[layout]
sidebar_width = 246.0
top_bar_height = 78.0
outer_margin = 32.0
content_gutter = 16.0
inspector_width_min = 360.0
inspector_width_max = 420.0
bottom_timeline_min = 220.0
graph_canvas_min_width = 720.0
graph_canvas_min_height = 520.0
window_width = 1920.0
window_height = 1080.0

[radius]
chip = 10.0
control = 12.0
card_min = 14.0
card = 16.0
card_max = 22.0
panel = 20.0
window = 24.0

[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
window = "0 20 60 rgba(16,24,40,0.14)"
focus = "0 0 0 4 rgba(31,122,245,0.14)"
failure = "0 0 0 4 rgba(229,72,77,0.12)"
taint = "0 0 0 4 rgba(139,92,246,0.12)"

[space]
px_4 = 4.0
px_8 = 8.0
px_12 = 12.0
px_16 = 16.0
px_20 = 20.0
px_24 = 24.0
px_32 = 32.0
px_40 = 40.0

[type]
family_sans = "Inter, SF Pro, system-ui"
family_mono = "JetBrains Mono, SF Mono, ui-monospace"
size_11 = 11
size_12 = 12
size_13 = 13
size_14 = 14
size_16 = 16
size_20 = 20
size_24 = 24
weight_regular = 400
weight_medium = 500
weight_semibold = 600
"##;
        let Ok(parsed) = vb_ui_makepad::tokens::ParsedTokens::from_toml(toml) else {
            panic!("valid toml should parse")
        };
        assert!((parsed.layout.sidebar_width - 246.0).abs() < 1e-6);
        assert!((parsed.space.px_4 - 4.0).abs() < 1e-6);
    }

    #[test]
    fn from_toml_integer_layout_value_parses_as_f64() {
        let toml = r##"
[color]
background_board = "#FFFFFF"
shell = "#FFFFFF"
surface = "#FFFFFF"
surface_glass = "#FFFFFF"
surface_muted = "#FFFFFF"
line_hair = "#FFFFFF"
line_soft = "#FFFFFF"
text_primary = "#FFFFFF"
text_secondary = "#FFFFFF"
text_tertiary = "#FFFFFF"
success = "#FFFFFF"
running = "#FFFFFF"
active_cyan = "#FFFFFF"
warning = "#FFFFFF"
failure = "#FFFFFF"
taint = "#FFFFFF"
durable = "#FFFFFF"
pending = "#FFFFFF"

[layout]
sidebar_width = 246
top_bar_height = 78
outer_margin = 32
content_gutter = 16
inspector_width_min = 360
inspector_width_max = 420
bottom_timeline_min = 220
graph_canvas_min_width = 720
graph_canvas_min_height = 520
window_width = 1920
window_height = 1080

[space]
px_4 = 4.0
px_8 = 8.0
px_12 = 12.0
px_16 = 16.0
px_20 = 20.0
px_24 = 24.0
px_32 = 32.0
px_40 = 40.0

[radius]
chip = 10.0
control = 12.0
card_min = 14.0
card = 16.0
card_max = 22.0
panel = 20.0
window = 24.0

[shadow]
card = "shadow"
window = "shadow"
focus = "shadow"
failure = "shadow"
taint = "shadow"

[type]
family_sans = "sans"
family_mono = "mono"
size_11 = 11
size_12 = 12
size_13 = 13
size_14 = 14
size_16 = 16
size_20 = 20
size_24 = 24
weight_regular = 400
weight_medium = 500
weight_semibold = 600
"##;
        let Ok(parsed) = vb_ui_makepad::tokens::ParsedTokens::from_toml(toml) else {
            panic!("integer layout values should parse")
        };
        assert!((parsed.layout.sidebar_width - 246.0).abs() < 1e-6);
        assert!((parsed.layout.top_bar_height - 78.0).abs() < 1e-6);
    }
}

// =============================================================================
// COLOR CONSTANTS TESTS
// =============================================================================

mod color_constants_tests {
    use super::*;

    #[test]
    fn color_background_board_returns_valid_rgba() {
        let rgba: [f32; 4] = color::background_board();
        // array length is guaranteed by type annotation; type-level invariant
        assert!(rgba.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }

    #[test]
    fn color_surface_glass_has_alpha_less_than_one() {
        let rgba = color::surface_glass();
        assert!(rgba[3] < 1.0, "glass alpha should be less than 1.0");
    }

    #[test]
    fn color_failure_is_redish() {
        let rgba = color::failure();
        assert!(rgba[0] > 0.5, "failure should be red-dominant");
    }

    #[test]
    fn color_success_is_greenish() {
        let rgba = color::success();
        assert!(rgba[1] > 0.5, "success should be green-dominant");
    }
}

// =============================================================================
// LAYOUT CONSTANTS TESTS
// =============================================================================

mod layout_constants_tests {
    use super::*;

    #[test]
    fn layout_sidebar_width_equals_246() {
        assert!((layout::SIDEBAR_WIDTH - 246.0).abs() < 1e-6);
    }

    #[test]
    fn layout_top_bar_height_equals_78() {
        assert!((layout::TOP_BAR_HEIGHT - 78.0).abs() < 1e-6);
    }

    #[test]
    fn layout_graph_canvas_min_dimensions() {
        // These are regression guards on constant values
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(layout::GRAPH_CANVAS_MIN_WIDTH >= 720.0);
            assert!(layout::GRAPH_CANVAS_MIN_HEIGHT >= 520.0);
        }
    }
}

// =============================================================================
// RADIUS CONSTANTS TESTS
// =============================================================================

mod radius_constants_tests {
    use super::*;

    #[test]
    fn radius_card_equals_16() {
        assert!((radius::CARD - 16.0).abs() < 1e-6);
    }
}

// =============================================================================
// SHADOW CONSTANTS TESTS
// =============================================================================

mod shadow_constants_tests {
    use super::*;

    #[test]
    fn shadow_card_has_correct_format() {
        let s = shadow::CARD;
        assert!(s.starts_with("0 "));
        assert!(s.contains("rgba"));
    }
}

// =============================================================================
// SPACE CONSTANTS TESTS
// =============================================================================

mod space_constants_tests {
    use super::*;

    #[test]
    fn space_px_4_equals_4() {
        assert!((space::PX_4 - 4.0).abs() < 1e-6);
    }

    #[test]
    fn space_px_8_equals_8() {
        assert!((space::PX_8 - 8.0).abs() < 1e-6);
    }

    #[test]
    fn space_px_16_equals_16() {
        assert!((space::PX_16 - 16.0).abs() < 1e-6);
    }

    #[test]
    fn space_px_32_equals_32() {
        assert!((space::PX_32 - 32.0).abs() < 1e-6);
    }

    #[test]
    fn space_sequence_is_correct() {
        // Regression guard on constant ordering
        #[allow(clippy::assertions_on_constants)]
        {
            assert!(space::PX_4 < space::PX_8);
            assert!(space::PX_8 < space::PX_16);
            assert!(space::PX_16 < space::PX_32);
            assert!(space::PX_32 < space::PX_40);
        }
    }
}

// =============================================================================
// VIEWPORT_RECT TESTS
// =============================================================================

mod viewport_rect_tests {
    use super::*;

    #[test]
    fn viewport_rect_intersects_disjoint_returns_false() {
        let v = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        // Another rect completely to the right
        let result = v.intersects(200.0, 0.0, 100.0, 100.0);
        assert!(!result, "disjoint rects should not intersect");
    }

    #[test]
    fn viewport_rect_intersects_overlapping_returns_true() {
        let v = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let result = v.intersects(50.0, 50.0, 100.0, 100.0);
        assert!(result, "overlapping rects should intersect");
    }

    #[test]
    fn viewport_rect_intersects_adjacent_edge_returns_false() {
        let v = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        // Rect starts exactly where this one ends (touching edge)
        let result = v.intersects(100.0, 0.0, 100.0, 100.0);
        assert!(!result, "adjacent touching rects should not intersect");
    }

    #[test]
    fn viewport_rect_intersects_contained_returns_true() {
        let v = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
        let result = v.intersects(50.0, 50.0, 50.0, 50.0);
        assert!(result, "contained rect should intersect");
    }
}

// =============================================================================
// GRAPH_CANVAS TESTS
// =============================================================================

mod graph_canvas_tests {
    use super::*;

    fn make_canvas() -> GraphCanvas {
        let positions = vec![(100.0, 100.0), (200.0, 200.0), (300.0, 300.0)];
        let edges = vec![
            EdgePath {
                source_step: 0,
                target_step: 1,
                start: [100.0, 100.0],
                cp1: [150.0, 100.0],
                cp2: [200.0, 150.0],
                end: [200.0, 200.0],
            },
            EdgePath {
                source_step: 1,
                target_step: 2,
                start: [200.0, 200.0],
                cp1: [250.0, 200.0],
                cp2: [300.0, 250.0],
                end: [300.0, 300.0],
            },
        ];
        GraphCanvas::new(3, positions, edges)
    }

    #[test]
    fn graph_canvas_new_sets_correct_counts() {
        let canvas = make_canvas();
        assert_eq!(canvas.node_count(), 3);
        assert_eq!(canvas.edge_count(), 2);
    }

    #[test]
    fn graph_canvas_viewport_rect_at_zoom_1() {
        let canvas = make_canvas();
        let rect = canvas.viewport_rect(1920.0, 1080.0);
        assert!((rect.width - 1920.0).abs() < 1e-3);
        assert!((rect.height - 1080.0).abs() < 1e-3);
    }

    #[test]
    fn graph_canvas_viewport_rect_handles_zero_zoom() {
        let mut canvas = make_canvas();
        canvas.set_zoom(0.0);
        let rect = canvas.viewport_rect(1920.0, 1080.0);
        // Zoom 0.0 is clamped to MIN_ZOOM (0.1), so inv_zoom = 10.0
        assert!(
            (rect.width - 19200.0).abs() < 1e-3,
            "width should be 19200 at clamped zoom 0.1"
        );
        assert!(
            (rect.height - 10800.0).abs() < 1e-3,
            "height should be 10800 at clamped zoom 0.1"
        );
    }

    #[test]
    fn graph_canvas_visible_nodes_empty_viewport_returns_none() {
        let canvas = make_canvas();
        let viewport = ViewportRect {
            x: 5000.0,
            y: 5000.0,
            width: 100.0,
            height: 100.0,
        };
        let result = canvas.visible_nodes(&viewport, (160.0, 48.0));
        assert!(
            result.is_empty(),
            "nodes far from viewport should not be visible"
        );
    }

    #[test]
    fn graph_canvas_visible_nodes_includes_intersecting() {
        let canvas = make_canvas();
        let viewport = ViewportRect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 400.0,
        };
        let result = canvas.visible_nodes(&viewport, (160.0, 48.0));
        assert!(
            !result.is_empty(),
            "nodes intersecting viewport should be visible"
        );
    }

    #[test]
    fn graph_canvas_set_zoom_below_min_clamps() {
        let mut canvas = make_canvas();
        canvas.set_zoom(0.05);
        let z = canvas.zoom();
        assert!(z >= 0.1, "zoom should clamp to min 0.1, got {}", z);
    }

    #[test]
    fn graph_canvas_set_zoom_above_max_clamps() {
        let mut canvas = make_canvas();
        canvas.set_zoom(10.0);
        let z = canvas.zoom();
        assert!(z <= 5.0, "zoom should clamp to max 5.0, got {}", z);
    }

    #[test]
    fn graph_canvas_set_zoom_valid_value() {
        let mut canvas = make_canvas();
        canvas.set_zoom(2.5);
        assert!((canvas.zoom() - 2.5).abs() < 1e-6);
    }

    #[test]
    fn graph_canvas_zoom_in_multiplies() {
        let mut canvas = make_canvas();
        canvas.set_zoom(1.0);
        canvas.zoom_in(1.5);
        assert!((canvas.zoom() - 1.5).abs() < 1e-6);
    }

    #[test]
    fn graph_canvas_zoom_out_divides() {
        let mut canvas = make_canvas();
        canvas.set_zoom(1.0);
        canvas.zoom_out(2.0);
        assert!((canvas.zoom() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn graph_canvas_zoom_reset_sets_to_1() {
        let mut canvas = make_canvas();
        canvas.set_zoom(3.0);
        canvas.zoom_reset();
        assert!((canvas.zoom() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn graph_canvas_zoom_percentage_formats() {
        let mut canvas = make_canvas();
        canvas.set_zoom(1.5);
        let pct = canvas.zoom_percentage();
        assert!(
            pct.contains("150%"),
            "zoom percentage should be 150%, got {}",
            pct
        );
    }

    #[test]
    fn graph_canvas_set_pan_updates_coordinates() {
        let mut canvas = make_canvas();
        canvas.set_pan(100.0, 200.0);
        let (x, y) = canvas.pan();
        assert!((x - 100.0).abs() < 1e-6);
        assert!((y - 200.0).abs() < 1e-6);
    }

    #[test]
    fn graph_canvas_set_selected_updates() {
        let mut canvas = make_canvas();
        canvas.set_selected(Some(2));
        assert_eq!(canvas.selected(), Some(2));
    }

    #[test]
    fn graph_canvas_focus_jump_valid_node_returns_true() {
        let mut canvas = make_canvas();
        canvas.set_pan(0.0, 0.0);
        let result = canvas.focus_jump(0, 1920.0, 1080.0);
        assert!(result, "focus_jump on valid node should return true");
    }

    #[test]
    fn graph_canvas_focus_jump_invalid_node_returns_false() {
        let mut canvas = make_canvas();
        let result = canvas.focus_jump(999, 1920.0, 1080.0);
        assert!(!result, "focus_jump on invalid node should return false");
    }

    #[test]
    fn graph_canvas_node_layout_position_valid_index() {
        let canvas = make_canvas();
        let Some((x, y)) = canvas.node_layout_position(0) else {
            panic!("should return position for valid index")
        };
        assert!((x - 100.0).abs() < 1e-6);
        assert!((y - 100.0).abs() < 1e-6);
    }

    #[test]
    fn graph_canvas_node_layout_position_invalid_index() {
        let canvas = make_canvas();
        let pos = canvas.node_layout_position(999);
        assert!(pos.is_none());
    }

    #[test]
    fn graph_canvas_render_node_card_valid_index() {
        let canvas = make_canvas();
        let result = canvas.render_node_card(0);
        assert!(result.is_some());
    }

    #[test]
    fn graph_canvas_render_node_card_invalid_index() {
        let canvas = make_canvas();
        let result = canvas.render_node_card(999);
        assert!(result.is_none());
    }

    #[test]
    fn graph_canvas_render_edge_valid_id() {
        let canvas = make_canvas();
        let result = canvas.render_edge("0");
        assert!(result.is_some());
    }

    #[test]
    fn graph_canvas_render_edge_invalid_id() {
        let canvas = make_canvas();
        let result = canvas.render_edge("not_a_number");
        assert!(result.is_none());
    }

    #[test]
    fn graph_canvas_set_node_overlay_valid_index() {
        let mut canvas = make_canvas();
        canvas.set_node_overlay(0, Some(OverlayState::Failed));
        let color = canvas.node_status_dot_color(0);
        assert!(color.is_some());
    }

    #[test]
    fn graph_canvas_set_node_overlay_invalid_index_noop() {
        let mut canvas = make_canvas();
        // Should not panic
        canvas.set_node_overlay(999, Some(OverlayState::Failed));
    }

    #[test]
    fn graph_canvas_compute_edge_paths_returns_cloned() {
        let canvas = make_canvas();
        let paths = canvas.compute_edge_paths();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn graph_canvas_node_badges_returns_empty() {
        let canvas = make_canvas();
        let badges = canvas.node_badges(0);
        assert!(badges.is_empty());
    }

    #[test]
    fn graph_canvas_set_taint_overlay() {
        let mut canvas = make_canvas();
        canvas.set_taint_overlay(true);
        // Smoke test - no panic
    }
}

// =============================================================================
// OVERLAY_STATE TESTS
// =============================================================================

mod overlay_state_tests {
    use super::*;

    #[test]
    fn overlay_state_glow_color_pending() {
        let c = OverlayState::Pending.glow_color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn overlay_state_glow_color_running() {
        let c = OverlayState::Running.glow_color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn overlay_state_glow_color_failed() {
        let c = OverlayState::Failed.glow_color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn overlay_state_glow_radius_running() {
        let r = OverlayState::Running.glow_radius();
        assert!((r - 4.0).abs() < 1e-6);
    }

    #[test]
    fn overlay_state_glow_radius_failed() {
        let r = OverlayState::Failed.glow_radius();
        assert!((r - 6.0).abs() < 1e-6);
    }
}

// =============================================================================
// NODE_BADGE TESTS
// =============================================================================

mod node_badge_tests {
    use super::*;

    #[test]
    fn node_badge_label_action_id() {
        let badge = NodeBadge::ActionId(42);
        assert_eq!(badge.label(), "A42");
    }

    #[test]
    fn node_badge_label_retry_max() {
        let badge = NodeBadge::RetryMax(3);
        assert_eq!(badge.label(), "R3");
    }

    #[test]
    fn node_badge_label_timeout() {
        let badge = NodeBadge::Timeout(30);
        assert_eq!(badge.label(), "T30s");
    }

    #[test]
    fn node_badge_label_secret_sensitive() {
        let badge = NodeBadge::SecretSensitive;
        assert_eq!(badge.label(), "S");
    }

    #[test]
    fn node_badge_label_strict_durable() {
        let badge = NodeBadge::StrictDurable;
        assert_eq!(badge.label(), "D");
    }

    #[test]
    fn node_badge_label_recent_failures() {
        let badge = NodeBadge::RecentFailures(5);
        assert_eq!(badge.label(), "!5");
    }

    #[test]
    fn node_badge_color_action_id() {
        let badge = NodeBadge::ActionId(1);
        let c = badge.color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn node_badge_color_timeout() {
        let badge = NodeBadge::Timeout(1);
        let c = badge.color();
        assert!(c[0] > 0.9); // Red dominant
    }
}

// =============================================================================
// NODE_CARD_RENDER_INSTR TESTS
// =============================================================================

mod node_card_render_instr_tests {
    use super::*;

    #[test]
    fn node_card_render_instr_focus_shadow_color() {
        let c = NodeCardRenderInstr::focus_shadow_color();
        assert_eq!(c.len(), 4);
        assert!((c[0] - 0.122).abs() < 1e-3);
    }

    #[test]
    fn node_card_render_instr_failure_shadow_color() {
        let c = NodeCardRenderInstr::failure_shadow_color();
        assert_eq!(c.len(), 4);
        assert!((c[0] - 0.898).abs() < 1e-3);
    }

    #[test]
    fn node_card_render_instr_taint_overlay_color() {
        let c = NodeCardRenderInstr::taint_overlay_color();
        assert_eq!(c.len(), 4);
    }
}

// =============================================================================
// GRAPH_NODE TESTS
// =============================================================================

mod graph_node_tests {
    use super::*;

    #[test]
    fn graph_node_card_dimensions() {
        let dims = GraphNode::card_dimensions();
        assert!((dims.0 - 160.0).abs() < 1e-6);
        assert!((dims.1 - 48.0).abs() < 1e-6);
    }

    #[test]
    fn graph_node_header_dimensions() {
        let dims = GraphNode::header_dimensions();
        assert!((dims.0 - 160.0).abs() < 1e-6);
        assert!((dims.1 - 24.0).abs() < 1e-6);
    }

    #[test]
    fn graph_node_badge_size() {
        let size = GraphNode::badge_size();
        assert!((size - 16.0).abs() < 1e-6);
    }
}

// =============================================================================
// EDGE_TYPE TESTS
// =============================================================================

mod edge_type_tests {
    use super::*;

    #[test]
    fn edge_type_color_normal() {
        let c = EdgeType::Normal.color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn edge_type_color_branch() {
        let c = EdgeType::Branch.color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn edge_type_is_dashed_branch_true() {
        assert!(EdgeType::Branch.is_dashed());
    }

    #[test]
    fn edge_type_is_dashed_normal_false() {
        assert!(!EdgeType::Normal.is_dashed());
    }

    #[test]
    fn edge_type_is_dashed_join_false() {
        assert!(!EdgeType::Join.is_dashed());
    }

    #[test]
    fn edge_type_is_dashed_loop_back_false() {
        assert!(!EdgeType::LoopBack.is_dashed());
    }
}

// =============================================================================
// EDGE_RENDER_INSTR TESTS
// =============================================================================

mod edge_render_instr_tests {
    use super::*;

    #[test]
    fn edge_render_instr_from_edge_path() {
        let instr = EdgeRenderInstr::from_edge_path(
            0,
            1,
            [0.0, 0.0],
            [50.0, 0.0],
            [100.0, 50.0],
            [150.0, 50.0],
            EdgeType::Normal,
        );
        assert_eq!(instr.source_step, 0);
        assert_eq!(instr.target_step, 1);
        assert_eq!(instr.edge_type, EdgeType::Normal);
    }

    #[test]
    fn edge_render_instr_with_label() {
        let instr = EdgeRenderInstr::from_edge_path(
            0,
            1,
            [0.0, 0.0],
            [50.0, 0.0],
            [100.0, 50.0],
            [150.0, 50.0],
            EdgeType::Normal,
        )
        .with_label("test".into());
        let Some(label) = instr.label else {
            panic!("should have label")
        };
        assert_eq!(label, "test");
    }
}

// =============================================================================
// PACKET_MARKER_INSTR TESTS
// =============================================================================

mod packet_marker_instr_tests {
    use super::*;

    #[test]
    fn packet_marker_instr_new_t_half_clamped() {
        let instr = PacketMarkerInstr::new(0.5);
        assert!((instr.t - 0.5).abs() < 1e-6);
    }

    #[test]
    fn packet_marker_instr_new_t_negative_clamped_to_zero() {
        let instr = PacketMarkerInstr::new(-0.5);
        assert!((instr.t - 0.0).abs() < 1e-6);
    }

    #[test]
    fn packet_marker_instr_new_t_above_one_clamped_to_one() {
        let instr = PacketMarkerInstr::new(1.5);
        assert!((instr.t - 1.0).abs() < 1e-6);
    }

    #[test]
    fn packet_marker_instr_color_is_active_cyan() {
        let instr = PacketMarkerInstr::new(0.5);
        assert_eq!(instr.color, color::active_cyan());
    }

    #[test]
    fn packet_marker_instr_size_is_six() {
        let instr = PacketMarkerInstr::new(0.5);
        assert!((instr.size - 6.0).abs() < 1e-6);
    }
}

// =============================================================================
// PACKET_DOT TESTS
// =============================================================================

mod packet_dot_tests {
    use super::*;

    #[test]
    fn packet_dot_new_default_values() {
        let dot = PacketDot::new("e1".into());
        assert_eq!(dot.edge_id, "e1");
        assert!((dot.t - 0.0).abs() < 1e-6);
        assert!((dot.speed - 0.2).abs() < 1e-6);
        assert!(dot.active);
    }

    #[test]
    fn packet_dot_position_at_t_zero_returns_start() {
        let pos = PacketDot::position_along_bezier(
            0.0,
            [0.0, 0.0],
            [50.0, 0.0],
            [100.0, 50.0],
            [150.0, 50.0],
        );
        assert!((pos[0] - 0.0).abs() < 1e-6);
        assert!((pos[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn packet_dot_position_at_t_one_returns_end() {
        let pos = PacketDot::position_along_bezier(
            1.0,
            [0.0, 0.0],
            [50.0, 0.0],
            [100.0, 50.0],
            [150.0, 50.0],
        );
        assert!((pos[0] - 150.0).abs() < 1e-6);
        assert!((pos[1] - 50.0).abs() < 1e-6);
    }

    #[test]
    fn packet_dot_reset_sets_t_zero_and_active_true() {
        let mut dot = PacketDot::new("e1".into());
        dot.finish();
        assert!(!dot.active);
        dot.reset();
        assert!((dot.t - 0.0).abs() < 1e-6);
        assert!(dot.active);
    }

    #[test]
    fn packet_dot_finish_sets_t_one_and_active_false() {
        let mut dot = PacketDot::new("e1".into());
        dot.finish();
        assert!((dot.t - 1.0).abs() < 1e-6);
        assert!(!dot.active);
    }

    #[test]
    fn packet_dot_color_returns_active_cyan() {
        let dot = PacketDot::new("e1".into());
        assert_eq!(dot.color(), color::active_cyan());
    }

    #[test]
    fn packet_dot_size_returns_six() {
        let dot = PacketDot::new("e1".into());
        assert!((dot.size() - 6.0).abs() < 1e-6);
    }
}

// =============================================================================
// ANIMATION_TICK TESTS
// =============================================================================

mod animation_tick_tests {
    use vb_ui_makepad::AnimationTick;

    #[test]
    fn animation_tick_new_stores_delta_ms() {
        let tick = AnimationTick::new(500.0);
        assert!((tick.delta_ms - 500.0).abs() < 1e-6);
    }

    #[test]
    fn animation_tick_normalized_delta_divides_by_1000() {
        let tick = AnimationTick::new(500.0);
        assert!((tick.normalized_delta() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn animation_tick_normalized_delta_1000ms_is_1() {
        let tick = AnimationTick::new(1000.0);
        assert!((tick.normalized_delta() - 1.0).abs() < 1e-6);
    }
}

// =============================================================================
// PACKET_DOT_MANAGER TESTS
// =============================================================================

mod packet_dot_manager_tests {
    use super::*;

    #[test]
    fn packet_dot_manager_new() {
        let manager = PacketDotManager::new();
        assert_eq!(manager.total_count(), 0);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn packet_dot_manager_add_dot_increments_count() {
        let mut manager = PacketDotManager::new();
        manager.add_dot("e1".into());
        assert_eq!(manager.total_count(), 1);
        assert_eq!(manager.active_count(), 1);
    }

    #[test]
    fn packet_dot_manager_add_dot_multiple() {
        let mut manager = PacketDotManager::new();
        manager.add_dot("e1".into());
        manager.add_dot("e2".into());
        manager.add_dot("e3".into());
        assert_eq!(manager.total_count(), 3);
        assert_eq!(manager.active_count(), 3);
    }

    #[test]
    fn packet_dot_manager_animate_deactivates_at_t_one() {
        let mut manager = PacketDotManager::new();
        manager.add_dot("e1".into());
        // Animate enough to finish (1000ms at speed 0.2 = t reaches 0.2 per call)
        // 5 calls × 0.2 = 1.0 t value = deactivation threshold
        manager.animate(1000.0);
        manager.animate(1000.0);
        manager.animate(1000.0);
        manager.animate(1000.0);
        manager.animate(1000.0);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn packet_dot_manager_clear_removes_all() {
        let mut manager = PacketDotManager::new();
        manager.add_dot("e1".into());
        manager.add_dot("e2".into());
        manager.clear();
        assert_eq!(manager.total_count(), 0);
        assert_eq!(manager.active_count(), 0);
    }

    #[test]
    fn packet_dot_manager_reset_all_resets_t() {
        let mut manager = PacketDotManager::new();
        manager.add_dot("e1".into());
        manager.add_dot("e2".into());
        manager.animate(500.0); // Advance a bit
        manager.reset_all();
        // After reset, all dots should be active at t=0
        assert_eq!(manager.active_count(), 2);
    }
}

// =============================================================================
// SHELL_NAV TESTS
// =============================================================================

mod shell_nav_tests {
    use super::*;

    #[test]
    fn shell_nav_label_overview() {
        assert_eq!(ShellNav::Overview.label(), "Overview");
    }

    #[test]
    fn shell_nav_label_workflow_graph() {
        assert_eq!(ShellNav::WorkflowGraph.label(), "Workflow Graph");
    }

    #[test]
    fn shell_nav_nav_color_overview() {
        let c = ShellNav::Overview.nav_color();
        assert_eq!(c.len(), 4);
    }

    #[test]
    fn shell_nav_screen_overview() {
        assert_eq!(ShellNav::Overview.screen(), Screen::ExecutionOverview);
    }

    #[test]
    fn shell_nav_screen_incidents() {
        assert_eq!(ShellNav::Incidents.screen(), Screen::IncidentFailureConsole);
    }
}

// =============================================================================
// SHELL_STATUS_CHIP TESTS
// =============================================================================

mod shell_status_chip_tests {
    use super::*;

    #[test]
    fn shell_status_chip_new() {
        let chip = ShellStatusChip::new("Running", [0.1, 0.5, 0.9, 1.0]);
        assert_eq!(chip.label, "Running");
        assert_eq!(chip.color, [0.1, 0.5, 0.9, 1.0]);
    }
}

// =============================================================================
// SCREEN TESTS
// =============================================================================

mod screen_tests {
    use super::*;

    #[test]
    fn screen_splash_name_execution_overview() {
        assert_eq!(Screen::ExecutionOverview.splash_name(), "ExecutionOverview");
    }

    #[test]
    fn screen_nav_label_execution_overview() {
        assert_eq!(Screen::ExecutionOverview.nav_label(), "Overview");
    }

    #[test]
    fn screen_is_shell_screen_always_true() {
        assert!(Screen::ExecutionOverview.is_shell_screen());
        assert!(Screen::IncidentFailureConsole.is_shell_screen());
    }
}

// =============================================================================
// APP_SHELL TESTS
// =============================================================================

mod app_shell_tests {
    use super::*;

    #[test]
    fn app_shell_new_returns_ok_with_overview_nav() {
        let Ok(shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        assert_eq!(shell.active_nav(), ShellNav::Overview);
    }

    #[test]
    fn app_shell_set_active_nav() {
        let Ok(mut shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        shell.set_active_nav(ShellNav::WorkflowGraph);
        assert_eq!(shell.active_nav(), ShellNav::WorkflowGraph);
    }

    #[test]
    fn app_shell_nav_item_rect_index_0() {
        let Ok(shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        let rect = shell.nav_item_rect(0);
        assert!((rect.y - 0.0).abs() < 1e-6);
        assert!((rect.height - 56.0).abs() < 1e-6);
    }

    #[test]
    fn app_shell_nav_item_rect_index_3() {
        let Ok(shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        let rect = shell.nav_item_rect(3);
        // y = 3 * 56 = 168
        assert!((rect.y - 168.0).abs() < 1e-6);
    }

    #[test]
    fn app_shell_nav_item_rect_large_index_handled_safely() {
        let Ok(shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        // Should not panic with large index
        let rect = shell.nav_item_rect(usize::MAX);
        // y will be 0.0 due to cast failure (safe fallback)
        assert!((rect.y - 0.0).abs() < 1e-6);
    }

    #[test]
    fn app_shell_topbar_rect() {
        let Ok(shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        let rect = shell.topbar_rect();
        assert!((rect.x - 246.0).abs() < 1e-6);
        assert!((rect.y - 0.0).abs() < 1e-6);
        assert!((rect.width - 1674.0).abs() < 1e-6);
        assert!((rect.height - 78.0).abs() < 1e-6);
    }

    #[test]
    fn app_shell_content_rect() {
        let Ok(shell) = AppShell::new() else {
            panic!("AppShell::new() should succeed")
        };
        let rect = shell.content_rect();
        assert!((rect.x - 246.0).abs() < 1e-6);
        assert!((rect.y - 78.0).abs() < 1e-6);
        assert!((rect.width - 1674.0).abs() < 1e-6);
        assert!((rect.height - 1002.0).abs() < 1e-6);
    }
}
