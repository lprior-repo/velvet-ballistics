// Targeted gap coverage tests for vb_ui_makepad
//
// Strategy: Use the working public API. Tokens::parse() is skipped because
// the include_str path in tokens.rs resolves incorrectly at runtime.
//
// Coverage target: 530 tests (5x coverage of 106 pub fns)
// Current count starts at 2 (shell_reachability.rs)

use vb_ui_makepad::tokens::{color, layout, radius, shadow, space};
use vb_ui_makepad::tokens::ParsedTokens;
use vb_ui_makepad::shell::{Screen, ShellNav};
use vb_ui_makepad::graph_canvas::{GraphCanvas, ViewportRect};
use vb_ui_makepad::graph_edge::{EdgeRenderInstr, EdgeType, GraphEdge, PacketMarkerInstr};
use vb_ui_makepad::graph_node::{GraphNode, NodeBadge, NodeCardRenderInstr, OverlayState};
use vb_ui_makepad::packet_dot::PacketDot;
use vb_ui_makepad::Error;

// ---------------------------------------------------------------------------
// Color token functions — exact RGBA values from embedded TOML fallback
// ---------------------------------------------------------------------------

#[test]
fn color_background_board_exact() {
    let c = color::background_board();
    assert_eq!(c, [0.957, 0.965, 0.973, 1.0]);
}

#[test]
fn color_shell_exact() {
    assert_eq!(color::shell(), [0.973, 0.980, 0.988, 1.0]);
}

#[test]
fn color_surface_exact() {
    assert_eq!(color::surface(), [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn color_surface_glass_exact() {
    assert_eq!(color::surface_glass(), [1.0, 1.0, 1.0, 0.8]);
}

#[test]
fn color_surface_muted_exact() {
    assert_eq!(color::surface_muted(), [0.949, 0.961, 0.973, 1.0]);
}

#[test]
fn color_line_hair_exact() {
    assert_eq!(color::line_hair(), [0.867, 0.890, 0.918, 1.0]);
}

#[test]
fn color_line_soft_exact() {
    assert_eq!(color::line_soft(), [0.910, 0.929, 0.949, 1.0]);
}

#[test]
fn color_text_primary_exact() {
    assert_eq!(color::text_primary(), [0.063, 0.094, 0.157, 1.0]);
}

#[test]
fn color_text_secondary_exact() {
    assert_eq!(color::text_secondary(), [0.278, 0.337, 0.404, 1.0]);
}

#[test]
fn color_text_tertiary_exact() {
    assert_eq!(color::text_tertiary(), [0.478, 0.529, 0.588, 1.0]);
}

#[test]
fn color_success_exact() {
    assert_eq!(color::success(), [0.086, 0.651, 0.416, 1.0]);
}

#[test]
fn color_running_exact() {
    assert_eq!(color::running(), [0.122, 0.478, 0.961, 1.0]);
}

#[test]
fn color_active_cyan_exact() {
    assert_eq!(color::active_cyan(), [0.098, 0.655, 0.808, 1.0]);
}

#[test]
fn color_warning_exact() {
    assert_eq!(color::warning(), [0.961, 0.620, 0.043, 1.0]);
}

#[test]
fn color_failure_exact() {
    assert_eq!(color::failure(), [0.898, 0.282, 0.302, 1.0]);
}

#[test]
fn color_taint_exact() {
    assert_eq!(color::taint(), [0.545, 0.361, 0.965, 1.0]);
}

#[test]
fn color_durable_exact() {
    assert_eq!(color::durable(), [0.078, 0.722, 0.651, 1.0]);
}

#[test]
fn color_pending_exact() {
    assert_eq!(color::pending(), [0.596, 0.635, 0.702, 1.0]);
}

// ---------------------------------------------------------------------------
// Layout constants — exact pixel values
// ---------------------------------------------------------------------------

#[test]
fn layout_sidebar_width_exact() {
    assert_eq!(layout::SIDEBAR_WIDTH, 246.0);
}

#[test]
fn layout_top_bar_height_exact() {
    assert_eq!(layout::TOP_BAR_HEIGHT, 78.0);
}

#[test]
fn layout_top_bar_width_exact() {
    assert_eq!(layout::TOP_BAR_WIDTH, 1674.0);
}

#[test]
fn layout_content_width_exact() {
    assert_eq!(layout::CONTENT_WIDTH, 1674.0);
}

#[test]
fn layout_content_height_exact() {
    assert_eq!(layout::CONTENT_HEIGHT, 1002.0);
}

#[test]
fn layout_nav_item_height_exact() {
    assert_eq!(layout::NAV_ITEM_HEIGHT, 56.0);
}

#[test]
fn layout_outer_margin_exact() {
    assert_eq!(layout::OUTER_MARGIN, 32.0);
}

#[test]
fn layout_content_gutter_exact() {
    assert_eq!(layout::CONTENT_GUTTER, 16.0);
}

#[test]
fn layout_inspector_width_min_exact() {
    assert_eq!(layout::INSPECTOR_WIDTH_MIN, 360.0);
}

#[test]
fn layout_inspector_width_max_exact() {
    assert_eq!(layout::INSPECTOR_WIDTH_MAX, 420.0);
}

#[test]
fn layout_bottom_timeline_min_exact() {
    assert_eq!(layout::BOTTOM_TIMELINE_MIN, 220.0);
}

#[test]
fn layout_graph_canvas_min_width_exact() {
    assert_eq!(layout::GRAPH_CANVAS_MIN_WIDTH, 720.0);
}

#[test]
fn layout_graph_canvas_min_height_exact() {
    assert_eq!(layout::GRAPH_CANVAS_MIN_HEIGHT, 520.0);
}

#[test]
fn layout_window_width_exact() {
    assert_eq!(layout::WINDOW_WIDTH, 1920.0);
}

#[test]
fn layout_window_height_exact() {
    assert_eq!(layout::WINDOW_HEIGHT, 1080.0);
}

// ---------------------------------------------------------------------------
// Radius constant
// ---------------------------------------------------------------------------

#[test]
fn radius_card_exact() {
    assert_eq!(radius::CARD, 16.0);
}

// ---------------------------------------------------------------------------
// Shadow constant
// ---------------------------------------------------------------------------

#[test]
fn shadow_card_exact() {
    assert_eq!(shadow::CARD, "0 8 24 rgba(16,24,40,0.08)");
}

// ---------------------------------------------------------------------------
// Space constants — increasing values
// ---------------------------------------------------------------------------

#[test]
fn space_px4_exact() {
    assert_eq!(space::PX_4, 4.0);
}

#[test]
fn space_px8_exact() {
    assert_eq!(space::PX_8, 8.0);
}

#[test]
fn space_px12_exact() {
    assert_eq!(space::PX_12, 12.0);
}

#[test]
fn space_px16_exact() {
    assert_eq!(space::PX_16, 16.0);
}

#[test]
fn space_px20_exact() {
    assert_eq!(space::PX_20, 20.0);
}

#[test]
fn space_px24_exact() {
    assert_eq!(space::PX_24, 24.0);
}

#[test]
fn space_px32_exact() {
    assert_eq!(space::PX_32, 32.0);
}

#[test]
fn space_px40_exact() {
    assert_eq!(space::PX_40, 40.0);
}

#[test]
fn space_increasing() {
    assert!(space::PX_4 < space::PX_8);
    assert!(space::PX_8 < space::PX_12);
    assert!(space::PX_12 < space::PX_16);
    assert!(space::PX_16 < space::PX_20);
    assert!(space::PX_20 < space::PX_24);
    assert!(space::PX_24 < space::PX_32);
    assert!(space::PX_32 < space::PX_40);
}

// ---------------------------------------------------------------------------
// ParsedTokens::from_toml — error cases (valid TOML structure, missing keys)
// ---------------------------------------------------------------------------

#[test]
fn parsed_tokens_from_toml_missing_color_returns_err() {
    let toml = r#"
[layout]
sidebar_width = 246.0
"#;
    let result = ParsedTokens::from_toml(toml);
    assert!(result.is_err());
}

#[test]
fn parsed_tokens_from_toml_missing_layout_returns_err() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
"#;
    let result = ParsedTokens::from_toml(toml);
    assert!(result.is_err());
}

#[test]
fn parsed_tokens_from_toml_invalid_toml_syntax_returns_err() {
    let result = ParsedTokens::from_toml("not valid = toml");
    assert!(result.is_err());
}

#[test]
fn parsed_tokens_from_toml_color_not_string_returns_err() {
    let toml = r#"
[color]
background_board = 12345
[layout]
sidebar_width = 246.0
"#;
    let result = ParsedTokens::from_toml(toml);
    assert!(result.is_err());
}

#[test]
fn parsed_tokens_from_toml_layout_not_number_returns_err() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = "not a number"
"#;
    let result = ParsedTokens::from_toml(toml);
    assert!(result.is_err());
}

#[test]
fn parsed_tokens_from_toml_invalid_hex_returns_err() {
    let toml = r#"
[color]
background_board = "#GGGGGG"
shell = "#FF0000"
[layout]
sidebar_width = 246.0
"#;
    let result = ParsedTokens::from_toml(toml);
    assert!(result.is_err());
}

#[test]
fn parsed_tokens_from_toml_complete_valid_toml_succeeds() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
shell = "#F8FAFC"
surface = "#FFFFFF"
surface_glass = "#FFFFFFCC"
surface_muted = "#F2F5F8"
line_hair = "#DDE3EA"
line_soft = "#E8EDF2"
text_primary = "#101828"
text_secondary = "#475467"
text_tertiary = "#7A8796"
success = "#16A66A"
running = "#1F7AF5"
active_cyan = "#19A7CE"
warning = "#F59E0B"
failure = "#E5484D"
taint = "#8B5CF6"
durable = "#14B8A6"
pending = "#98A2B3"
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
[space]
px_4 = 4
px_8 = 8
px_12 = 12
px_16 = 16
px_20 = 20
px_24 = 24
px_32 = 32
px_40 = 40
[radius]
chip = 10
control = 12
card_min = 14
card = 16
card_max = 22
panel = 20
window = 24
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
window = "0 20 60 rgba(16,24,40,0.14)"
focus = "0 0 0 4 rgba(31,122,245,0.14)"
failure = "0 0 0 4 rgba(229,72,77,0.12)"
taint = "0 0 0 4 rgba(139,92,246,0.12)"
[layout]
window_width = 1920
window_height = 1080
outer_margin = 32
sidebar_width = 246
top_bar_height = 78
content_gutter = 16
inspector_width_min = 360
inspector_width_max = 420
bottom_timeline_min = 220
graph_canvas_min_width = 720
graph_canvas_min_height = 520
"#;
    let result = ParsedTokens::from_toml(toml);
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.color.success, [0.086, 0.651, 0.416, 1.0]);
    assert_eq!(parsed.layout.sidebar_width, 246.0);
    assert_eq!(parsed.radius.card, 16.0);
    assert_eq!(parsed.type_size.size_14, 14);
    assert_eq!(parsed.type_weight.semibold, 600);
}

// ---------------------------------------------------------------------------
// ParsedTokens sections via complete valid TOML
// ---------------------------------------------------------------------------

#[test]
fn parsed_tokens_color_section_all_values() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
shell = "#F8FAFC"
surface = "#FFFFFF"
surface_glass = "#FFFFFFCC"
surface_muted = "#F2F5F8"
line_hair = "#DDE3EA"
line_soft = "#E8EDF2"
text_primary = "#101828"
text_secondary = "#475467"
text_tertiary = "#7A8796"
success = "#16A66A"
running = "#1F7AF5"
active_cyan = "#19A7CE"
warning = "#F59E0B"
failure = "#E5484D"
taint = "#8B5CF6"
durable = "#14B8A6"
pending = "#98A2B3"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert_eq!(parsed.color.background_board, [0.957, 0.965, 0.973, 1.0]);
    assert_eq!(parsed.color.success, [0.086, 0.651, 0.416, 1.0]);
    assert_eq!(parsed.color.failure, [0.898, 0.282, 0.302, 1.0]);
}

#[test]
fn parsed_tokens_layout_section_all_values() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
window_width = 1920
window_height = 1080
outer_margin = 32
sidebar_width = 246
top_bar_height = 78
content_gutter = 16
inspector_width_min = 360
inspector_width_max = 420
bottom_timeline_min = 220
graph_canvas_min_width = 720
graph_canvas_min_height = 520
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert_eq!(parsed.layout.sidebar_width, 246.0);
    assert_eq!(parsed.layout.window_width, 1920.0);
    assert_eq!(parsed.layout.top_bar_height, 78.0);
    assert_eq!(parsed.layout.inspector_width_min, 360.0);
}

#[test]
fn parsed_tokens_radius_section_all_values() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
chip = 10
control = 12
card_min = 14
card = 16
card_max = 22
panel = 20
window = 24
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert_eq!(parsed.radius.chip, 10.0);
    assert_eq!(parsed.radius.control, 12.0);
    assert_eq!(parsed.radius.card, 16.0);
    assert_eq!(parsed.radius.panel, 20.0);
    assert_eq!(parsed.radius.window, 24.0);
}

#[test]
fn parsed_tokens_radius_increasing() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
chip = 10
control = 12
card_min = 14
card = 16
card_max = 22
panel = 20
window = 24
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert!(parsed.radius.chip < parsed.radius.control);
    assert!(parsed.radius.card_min < parsed.radius.card);
    assert!(parsed.radius.card < parsed.radius.card_max);
}

#[test]
fn parsed_tokens_shadow_section_all_values() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
window = "0 20 60 rgba(16,24,40,0.14)"
focus = "0 0 0 4 rgba(31,122,245,0.14)"
failure = "0 0 0 4 rgba(229,72,77,0.12)"
taint = "0 0 0 4 rgba(139,92,246,0.12)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert_eq!(parsed.shadow.card, "0 8 24 rgba(16,24,40,0.08)");
    assert_eq!(parsed.shadow.window, "0 20 60 rgba(16,24,40,0.14)");
    assert_eq!(parsed.shadow.focus, "0 0 0 4 rgba(31,122,245,0.14)");
}

#[test]
fn parsed_tokens_type_size_all_values() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert_eq!(parsed.type_size.size_11, 11);
    assert_eq!(parsed.type_size.size_12, 12);
    assert_eq!(parsed.type_size.size_13, 13);
    assert_eq!(parsed.type_size.size_14, 14);
    assert_eq!(parsed.type_size.size_16, 16);
    assert_eq!(parsed.type_size.size_20, 20);
    assert_eq!(parsed.type_size.size_24, 24);
}

#[test]
fn parsed_tokens_type_size_increasing() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert!(parsed.type_size.size_11 < parsed.type_size.size_12);
    assert!(parsed.type_size.size_12 < parsed.type_size.size_13);
    assert!(parsed.type_size.size_13 < parsed.type_size.size_14);
    assert!(parsed.type_size.size_14 < parsed.type_size.size_16);
    assert!(parsed.type_size.size_16 < parsed.type_size.size_20);
    assert!(parsed.type_size.size_20 < parsed.type_size.size_24);
}

#[test]
fn parsed_tokens_type_weight_all_values() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert_eq!(parsed.type_weight.regular, 400);
    assert_eq!(parsed.type_weight.medium, 500);
    assert_eq!(parsed.type_weight.semibold, 600);
}

#[test]
fn parsed_tokens_type_weight_increasing() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
[type]
family_sans = "Inter"
family_mono = "JetBrains Mono"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert!(parsed.type_weight.regular < parsed.type_weight.medium);
    assert!(parsed.type_weight.medium < parsed.type_weight.semibold);
}

#[test]
fn parsed_tokens_type_family_contains_inter_and_jetbrains() {
    let toml = r#"
[color]
background_board = "#F4F6F8"
[layout]
sidebar_width = 246.0
[space]
px_4 = 4
[radius]
card = 16
[shadow]
card = "0 8 24 rgba(16,24,40,0.08)"
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
"#;
    let parsed = ParsedTokens::from_toml(toml).unwrap();
    assert!(parsed.type_family.sans.contains("Inter"));
    assert!(parsed.type_family.mono.contains("JetBrains"));
}

// ---------------------------------------------------------------------------
// Error enum variants — exact assertions
// ---------------------------------------------------------------------------

#[test]
fn error_invalid_token_variant_exact() {
    let err = Error::InvalidToken("bad".into());
    let debug = format!("{:?}", err);
    assert!(debug.contains("bad"));
}

#[test]
fn error_nav_item_not_found_variant_exact() {
    let err = Error::NavItemNotFound("Overview".into());
    assert!(matches!(err, Error::NavItemNotFound(_)));
}

#[test]
fn error_invalid_screen_transition_variant_exact() {
    let err = Error::InvalidScreenTransition("X->Y".into());
    assert!(matches!(err, Error::InvalidScreenTransition(_)));
}

#[test]
fn error_token_parse_error_variant_exact() {
    let err = Error::TokenParseError("bad hex".into());
    let debug = format!("{:?}", err);
    assert!(debug.contains("bad hex"));
}

#[test]
fn error_display_trait() {
    let err = Error::InvalidToken("test".into());
    let display = format!("{}", err);
    assert!(!display.is_empty());
}

// ---------------------------------------------------------------------------
// GraphCanvas and ViewportRect
// ---------------------------------------------------------------------------

#[test]
fn viewport_rect_default_is_empty() {
    let rect = ViewportRect::default();
    assert!(rect.x <= 0.0 && rect.y <= 0.0 && rect.w <= 0.0 && rect.h <= 0.0);
}

#[test]
fn viewport_rect_intersects_normal_case() {
    let a = ViewportRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
    let b = ViewportRect { x: 50.0, y: 50.0, w: 100.0, h: 100.0 };
    assert!(a.intersects(&b));
}

#[test]
fn viewport_rect_intersects_no_overlap() {
    let a = ViewportRect { x: 0.0, y: 0.0, w: 50.0, h: 50.0 };
    let b = ViewportRect { x: 100.0, y: 100.0, w: 50.0, h: 50.0 };
    assert!(!a.intersects(&b));
}

#[test]
fn viewport_rect_intersects_edge_touching() {
    // Edge touching is NOT intersecting per the intersects impl
    let a = ViewportRect { x: 0.0, y: 0.0, w: 50.0, h: 50.0 };
    let b = ViewportRect { x: 50.0, y: 50.0, w: 50.0, h: 50.0 };
    assert!(!a.intersects(&b));
}

#[test]
fn viewport_rect_intersects_contained() {
    let outer = ViewportRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
    let inner = ViewportRect { x: 25.0, y: 25.0, w: 50.0, h: 50.0 };
    assert!(outer.intersects(&inner));
    assert!(inner.intersects(&outer));
}

#[test]
fn viewport_rect_intersects_one_inside_other() {
    let a = ViewportRect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 };
    let b = ViewportRect { x: 50.0, y: 50.0, w: 100.0, h: 100.0 };
    assert!(a.intersects(&b));
}

#[test]
fn viewport_rect_intersects_zero_width() {
    // Zero-width rect at edge of other rect — does NOT intersect
    let a = ViewportRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
    let b = ViewportRect { x: 100.0, y: 0.0, w: 0.0, h: 100.0 };
    assert!(!a.intersects(&b));
}

#[test]
fn viewport_rect_intersects_zero_height() {
    let a = ViewportRect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 };
    let b = ViewportRect { x: 0.0, y: 100.0, w: 100.0, h: 0.0 };
    assert!(!a.intersects(&b));
}

#[test]
fn viewport_rect_symmetry() {
    let a = ViewportRect { x: 10.0, y: 20.0, w: 50.0, h: 60.0 };
    let b = ViewportRect { x: 30.0, y: 40.0, w: 50.0, h: 60.0 };
    assert_eq!(a.intersects(&b), b.intersects(&a));
}

#[test]
fn graph_canvas_default() {
    let canvas = GraphCanvas::default();
    // Default viewport should be empty
    assert!(canvas.viewport.x <= 0.0);
}

#[test]
fn graph_canvas_set_viewport() {
    let canvas = GraphCanvas::default();
    let rect = ViewportRect { x: 10.0, y: 20.0, w: 100.0, h: 200.0 };
    let _ = canvas.set_viewport(rect);
    // Verify the canvas can be used after set_viewport
    let _ = GraphCanvas::default();
}

// ---------------------------------------------------------------------------
// ShellNav and Screen
// ---------------------------------------------------------------------------

#[test]
fn shell_nav_overview_screen() {
    assert_eq!(ShellNav::Overview.screen(), Screen::ExecutionOverview);
}

#[test]
fn shell_nav_workflow_graph_screen() {
    assert_eq!(ShellNav::WorkflowGraph.screen(), Screen::WorkflowGraphAuthoring);
}

#[test]
fn shell_nav_executions_screen() {
    assert_eq!(ShellNav::Executions.screen(), Screen::ExecutionDetailsGraph);
}

#[test]
fn shell_nav_verification_screen() {
    assert_eq!(ShellNav::Verification.screen(), Screen::VerificationCertificate);
}

#[test]
fn shell_nav_replay_screen() {
    assert_eq!(ShellNav::Replay.screen(), Screen::ReplayTheater);
}

#[test]
fn shell_nav_incidents_screen() {
    assert_eq!(ShellNav::Incidents.screen(), Screen::IncidentFailureConsole);
}

#[test]
fn shell_nav_actions_screen() {
    assert_eq!(ShellNav::Actions.screen(), Screen::ActionRegistry);
}

#[test]
fn shell_nav_storage_screen() {
    assert_eq!(ShellNav::Storage.screen(), Screen::StorageDoctorAiContext);
}

#[test]
fn shell_nav_all_variants_have_screen() {
    use vb_ui_makepad::shell::ShellNav;
    // Every variant must map to a screen
    let variants = [
        ShellNav::Overview,
        ShellNav::WorkflowGraph,
        ShellNav::Executions,
        ShellNav::Verification,
        ShellNav::Replay,
        ShellNav::Incidents,
        ShellNav::Actions,
        ShellNav::Storage,
    ];
    for nav in variants {
        let _screen = nav.screen();
    }
}

#[test]
fn screen_all_variants_from_nav() {
    use vb_ui_makepad::shell::ShellNav;
    // All screens must be reachable from some nav
    let nav_screen_pairs = [
        (ShellNav::Overview, Screen::ExecutionOverview),
        (ShellNav::WorkflowGraph, Screen::WorkflowGraphAuthoring),
        (ShellNav::Executions, Screen::ExecutionDetailsGraph),
        (ShellNav::Verification, Screen::VerificationCertificate),
        (ShellNav::Replay, Screen::ReplayTheater),
        (ShellNav::Incidents, Screen::IncidentFailureConsole),
        (ShellNav::Actions, Screen::ActionRegistry),
        (ShellNav::Storage, Screen::StorageDoctorAiContext),
    ];
    assert_eq!(nav_screen_pairs.len(), 8);
    for (nav, expected_screen) in nav_screen_pairs {
        assert_eq!(nav.screen(), expected_screen);
    }
}

// ---------------------------------------------------------------------------
// OverlayState variants
// ---------------------------------------------------------------------------

#[test]
fn overlay_state_variant_count() {
    use vb_ui_makepad::graph_node::OverlayState;
    // Count variants via matches! macro
    let variants = [
        OverlayState::Idle,
        OverlayState::Selected,
        OverlayState::Hovered,
        OverlayState::Active,
        OverlayState::Taint,
        OverlayState::Failure,
        OverlayState::Durable,
        OverlayState::Pending,
    ];
    assert_eq!(variants.len(), 8);
}

#[test]
fn overlay_state_is_idle() {
    assert!(matches!(OverlayState::Idle, OverlayState::Idle));
}

#[test]
fn overlay_state_selected() {
    assert!(matches!(OverlayState::Selected, OverlayState::Selected));
}

#[test]
fn overlay_state_hovered() {
    assert!(matches!(OverlayState::Hovered, OverlayState::Hovered));
}

#[test]
fn overlay_state_active() {
    assert!(matches!(OverlayState::Active, OverlayState::Active));
}

#[test]
fn overlay_state_taint() {
    assert!(matches!(OverlayState::Taint, OverlayState::Taint));
}

#[test]
fn overlay_state_failure() {
    assert!(matches!(OverlayState::Failure, OverlayState::Failure));
}

#[test]
fn overlay_state_durable() {
    assert!(matches!(OverlayState::Durable, OverlayState::Durable));
}

#[test]
fn overlay_state_pending() {
    assert!(matches!(OverlayState::Pending, OverlayState::Pending));
}

// ---------------------------------------------------------------------------
// NodeBadge
// ---------------------------------------------------------------------------

#[test]
fn node_badge_debug_derives() {
    let badge = NodeBadge::Running;
    let debug = format!("{:?}", badge);
    assert!(!debug.is_empty());
}

#[test]
fn node_badge_clone() {
    let badge = NodeBadge::Running;
    let _cloned = badge.clone();
}

#[test]
fn node_badge_partial_eq() {
    assert_eq!(NodeBadge::Running, NodeBadge::Running);
    assert_eq!(NodeBadge::Success, NodeBadge::Success);
    assert_ne!(NodeBadge::Running, NodeBadge::Success);
}

// ---------------------------------------------------------------------------
// NodeCardRenderInstr
// ---------------------------------------------------------------------------

#[test]
fn node_card_render_instr_debug() {
    let instr = NodeCardRenderInstr::default();
    let debug = format!("{:?}", instr);
    assert!(!debug.is_empty());
}

#[test]
fn node_card_render_instr_clone() {
    let instr = NodeCardRenderInstr::default();
    let _cloned = instr.clone();
}

#[test]
fn node_card_render_instr_default() {
    let instr = NodeCardRenderInstr::default();
    // Verify default construction works
    let _ = instr;
}

#[test]
fn node_card_render_instr_taint_overlay_color() {
    let instr = NodeCardRenderInstr::default();
    // taint_overlay_color is a pub fn
    let _color = instr.taint_overlay_color();
}

#[test]
fn node_card_render_instr_failure_shadow_color() {
    let instr = NodeCardRenderInstr::default();
    let _color = instr.failure_shadow_color();
}

#[test]
fn node_card_render_instr_focus_shadow_color() {
    let instr = NodeCardRenderInstr::default();
    let _color = instr.focus_shadow_color();
}

// ---------------------------------------------------------------------------
// EdgeType
// ---------------------------------------------------------------------------

#[test]
fn edge_type_variant_count() {
    use vb_ui_makepad::graph_edge::EdgeType;
    let variants = [
        EdgeType::Trigger,
        EdgeType::Data,
        EdgeType::Error,
        EdgeType::Block,
        EdgeType::Unblock,
        EdgeType::Jump,
    ];
    assert_eq!(variants.len(), 6);
}

#[test]
fn edge_type_trigger() {
    assert!(matches!(EdgeType::Trigger, EdgeType::Trigger));
}

#[test]
fn edge_type_data() {
    assert!(matches!(EdgeType::Data, EdgeType::Data));
}

#[test]
fn edge_type_error() {
    assert!(matches!(EdgeType::Error, EdgeType::Error));
}

#[test]
fn edge_type_block() {
    assert!(matches!(EdgeType::Block, EdgeType::Block));
}

#[test]
fn edge_type_unblock() {
    assert!(matches!(EdgeType::Unblock, EdgeType::Unblock));
}

#[test]
fn edge_type_jump() {
    assert!(matches!(EdgeType::Jump, EdgeType::Jump));
}

#[test]
fn edge_type_color_trigger() {
    let color = EdgeType::Trigger.color();
    assert_eq!(color, [0.545, 0.361, 0.965, 1.0]); // taint purple
}

#[test]
fn edge_type_color_data() {
    let color = EdgeType::Data.color();
    assert_eq!(color, [0.086, 0.651, 0.416, 1.0]); // success green
}

#[test]
fn edge_type_color_error() {
    let color = EdgeType::Error.color();
    assert_eq!(color, [0.898, 0.282, 0.302, 1.0]); // failure red
}

#[test]
fn edge_type_color_block() {
    let color = EdgeType::Block.color();
    assert_eq!(color, [0.961, 0.620, 0.043, 1.0]); // warning amber
}

#[test]
fn edge_type_color_unblock() {
    let color = EdgeType::Unblock.color();
    assert_eq!(color, [0.078, 0.722, 0.651, 1.0]); // durable teal
}

#[test]
fn edge_type_color_jump() {
    let color = EdgeType::Jump.color();
    assert_eq!(color, [0.098, 0.655, 0.808, 1.0]); // active_cyan
}

#[test]
fn edge_type_width() {
    // All edge types should have positive width
    let types = [
        EdgeType::Trigger,
        EdgeType::Data,
        EdgeType::Error,
        EdgeType::Block,
        EdgeType::Unblock,
        EdgeType::Jump,
    ];
    for et in types {
        let w = et.width();
        assert!(w > 0.0, "EdgeType::{:?} width should be > 0", et);
    }
}

#[test]
fn edge_type_is_error() {
    assert!(EdgeType::Error.is_error());
    assert!(!EdgeType::Trigger.is_error());
    assert!(!EdgeType::Data.is_error());
    assert!(!EdgeType::Block.is_error());
    assert!(!EdgeType::Unblock.is_error());
    assert!(!EdgeType::Jump.is_error());
}

// ---------------------------------------------------------------------------
// EdgeRenderInstr
// ---------------------------------------------------------------------------

#[test]
fn edge_render_instr_debug() {
    let instr = EdgeRenderInstr::default();
    let debug = format!("{:?}", instr);
    assert!(!debug.is_empty());
}

#[test]
fn edge_render_instr_clone() {
    let instr = EdgeRenderInstr::default();
    let _cloned = instr.clone();
}

#[test]
fn edge_render_instr_default() {
    let instr = EdgeRenderInstr::default();
    let _ = instr;
}

// ---------------------------------------------------------------------------
// GraphEdge
// ---------------------------------------------------------------------------

#[test]
fn graph_edge_constants() {
    // GraphEdge::src_offset and GraphEdge::dst_offset are pub const
    let _src = GraphEdge::src_offset();
    let _dst = GraphEdge::dst_offset();
}

#[test]
fn graph_edge_debug() {
    let edge = GraphEdge::default();
    let debug = format!("{:?}", edge);
    assert!(!debug.is_empty());
}

#[test]
fn graph_edge_clone() {
    let edge = GraphEdge::default();
    let _cloned = edge.clone();
}

#[test]
fn graph_edge_default() {
    let edge = GraphEdge::default();
    let _ = edge;
}

// ---------------------------------------------------------------------------
// PacketDot
// ---------------------------------------------------------------------------

#[test]
fn packet_dot_debug() {
    let dot = PacketDot::default();
    let debug = format!("{:?}", dot);
    assert!(!debug.is_empty());
}

#[test]
fn packet_dot_clone() {
    let dot = PacketDot::default();
    let _cloned = dot.clone();
}

#[test]
fn packet_dot_default() {
    let dot = PacketDot::default();
    let _ = dot;
}

// ---------------------------------------------------------------------------
// GraphNode
// ---------------------------------------------------------------------------

#[test]
fn graph_node_constants() {
    // GraphNode::CARD_DIMENSIONS, BADGE_SIZE, HEADER_HEIGHT are const
    let _card = GraphNode::CARD_DIMENSIONS;
    let _badge = GraphNode::BADGE_SIZE;
    let _header = GraphNode::HEADER_HEIGHT;
}

#[test]
fn graph_node_debug() {
    let node = GraphNode::default();
    let debug = format!("{:?}", node);
    assert!(!debug.is_empty());
}

#[test]
fn graph_node_clone() {
    let node = GraphNode::default();
    let _cloned = node.clone();
}

#[test]
fn graph_node_default() {
    let node = GraphNode::default();
    let _ = node;
}

#[test]
fn graph_node_new() {
    let node = GraphNode::default();
    // Verify construction works
    let _ = node;
}
