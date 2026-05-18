//! Tests for tokens_to_rust_constants codegen output shape and hex parsing.

use vb_ui_snapshot::tokens::{UiTokens, tokens_to_rust_constants};

fn default_tokens() -> UiTokens {
    UiTokens::default()
}

//
// tokens_to_rust_constants output shape
//

#[test]
fn tokens_to_rust_constants_emits_file_header_comment() {
    let output = tokens_to_rust_constants(&default_tokens());
    assert!(output.starts_with("// Generated from velvet_ui_tokens.toml - DO NOT EDIT\n\n"));
}

#[test]
fn tokens_to_rust_constants_emits_token_colors_struct() {
    let output = tokens_to_rust_constants(&default_tokens());
    assert!(output.contains("#[derive(Debug, Clone, Copy)]"));
    assert!(output.contains("pub struct TokenColors {"));
    assert!(output.contains("pub surface:        [f32; 4],"));
    assert!(output.contains("pub text_primary:   [f32; 4],"));
    assert!(output.contains("pub success:        [f32; 4],"));
    assert!(output.contains("pub running:        [f32; 4],"));
    assert!(output.contains("pub failure:        [f32; 4],"));
    assert!(output.contains("pub taint:          [f32; 4],"));
    assert!(output.contains("pub durable:        [f32; 4],"));
    assert!(output.contains("pub warning:        [f32; 4],"));
}

#[test]
fn tokens_to_rust_constants_emits_tokens_const() {
    let output = tokens_to_rust_constants(&default_tokens());
    assert!(output.contains("pub const TOKENS: TokenColors = TokenColors {"));
    // Default surface is "#FFFFFF" → [1.0, 1.0, 1.0, 1.0]
    assert!(output.contains("surface:      [1.000000, 1.000000, 1.000000, 1.0]"));
    // Default text_primary is "#101828" → 16/255=0.062745, 24/255=0.094118, 40/255=0.156863
    assert!(output.contains("text_primary: [0.062745, 0.094118, 0.156863, 1.0]"));
}

#[test]
fn tokens_to_rust_constants_emits_token_layout_struct() {
    let output = tokens_to_rust_constants(&default_tokens());
    assert!(output.contains("pub const LAYOUT: TokenLayout = TokenLayout {"));
    assert!(output.contains("#[derive(Debug, Clone, Copy)]"));
    assert!(output.contains("pub struct TokenLayout {"));
    assert!(output.contains("pub window_width:     u32,"));
    assert!(output.contains("pub window_height:    u32,"));
    assert!(output.contains("pub outer_margin:     u32,"));
    assert!(output.contains("pub sidebar_width:    u32,"));
    assert!(output.contains("pub top_bar_height:   u32,"));
    assert!(output.contains("pub content_gutter:   u32,"));
    assert!(output.contains("pub chip_radius:      f32,"));
    assert!(output.contains("chip_radius:           10.0"));
}

#[test]
fn tokens_to_rust_constants_includes_all_eight_color_fields() {
    let output = tokens_to_rust_constants(&default_tokens());
    // All 8 colors in token_color_pairs
    assert!(output.contains("surface:      "));
    assert!(output.contains("text_primary: "));
    assert!(output.contains("success:      "));
    assert!(output.contains("running:      "));
    assert!(output.contains("failure:      "));
    assert!(output.contains("taint:        "));
    assert!(output.contains("durable:      "));
    assert!(output.contains("warning:      "));
}

#[test]
fn tokens_to_rust_constants_layout_constants_match_default() {
    let output = tokens_to_rust_constants(&default_tokens());
    assert!(output.contains("window_width:          1920"));
    assert!(output.contains("window_height:         1080"));
    assert!(output.contains("outer_margin:          32"));
    assert!(output.contains("sidebar_width:         246"));
    assert!(output.contains("top_bar_height:        78"));
    assert!(output.contains("content_gutter:        16"));
}

#[test]
fn tokens_to_rust_constants_zero_alpha_always_set() {
    // All colors emit 4-element arrays with alpha = 1.0
    let output = tokens_to_rust_constants(&default_tokens());
    // Every color literal ends in ", 1.0]"
    assert!(output.contains(", 1.0]"));
}

#[test]
fn tokens_to_rust_constants_unknown_hex_defaults_to_black() {
    let mut tokens = default_tokens();
    tokens.surface = "#ZZZZZZ".to_string();
    let output = tokens_to_rust_constants(&tokens);
    assert!(output.contains("[0.0, 0.0, 0.0, 1.0]"));
}

#[test]
fn tokens_to_rust_constants_single_hex_digit_color_defaults_to_black() {
    let mut tokens = default_tokens();
    tokens.success = "#FFF".to_string(); // Only 3 chars after #
    let output = tokens_to_rust_constants(&tokens);
    // 3-char hex is invalid (needs 6), so should fall back to black
    assert!(output.contains("[0.0, 0.0, 0.0, 1.0]"));
}

#[test]
fn tokens_to_rust_constants_all_zeros_black() {
    let mut tokens = default_tokens();
    tokens.failure = "#000000".to_string();
    let output = tokens_to_rust_constants(&tokens);
    assert!(output.contains("failure:      [0.000000, 0.000000, 0.000000, 1.0]"));
}

#[test]
fn tokens_to_rust_constants_all_ffs_white() {
    let mut tokens = default_tokens();
    tokens.surface = "#FFFFFF".to_string();
    let output = tokens_to_rust_constants(&tokens);
    assert!(output.contains("surface:      [1.000000, 1.000000, 1.000000, 1.0]"));
}

//
// UiTokens Default — all fields have expected defaults
//

#[test]
fn ui_tokens_default_layout_fields() {
    let t = UiTokens::default();
    assert_eq!(t.window_width, 1920);
    assert_eq!(t.window_height, 1080);
    assert_eq!(t.outer_margin, 32);
    assert_eq!(t.sidebar_width, 246);
    assert_eq!(t.top_bar_height, 78);
    assert_eq!(t.content_gutter, 16);
    assert_eq!(t.inspector_width_min, 360);
    assert_eq!(t.inspector_width_max, 420);
    assert_eq!(t.bottom_timeline_min, 220);
    assert_eq!(t.graph_canvas_min_width, 720);
    assert_eq!(t.graph_canvas_min_height, 520);
}

#[test]
fn ui_tokens_default_color_fields() {
    let t = UiTokens::default();
    assert_eq!(t.surface, "#FFFFFF");
    assert_eq!(t.shell, "#F8FAFC");
    assert_eq!(t.surface_glass, "#FFFFFFCC");
    assert_eq!(t.surface_muted, "#F2F5F8");
    assert_eq!(t.text_primary, "#101828");
    assert_eq!(t.text_secondary, "#475467");
    assert_eq!(t.text_tertiary, "#7A8796");
    assert_eq!(t.success, "#16A66A");
    assert_eq!(t.running, "#1F7AF5");
    assert_eq!(t.active_cyan, "#19A7CE");
    assert_eq!(t.warning, "#F59E0B");
    assert_eq!(t.failure, "#E5484D");
    assert_eq!(t.taint, "#8B5CF6");
    assert_eq!(t.durable, "#14B8A6");
    assert_eq!(t.pending, "#98A2B3");
}

#[test]
fn ui_tokens_default_radius_fields() {
    let t = UiTokens::default();
    assert_eq!(t.chip_radius, 10.0);
    assert_eq!(t.control_radius, 12.0);
    assert_eq!(t.card_min_radius, 14.0);
    assert_eq!(t.card_radius, 16.0);
    assert_eq!(t.card_max_radius, 22.0);
    assert_eq!(t.panel_radius, 20.0);
    assert_eq!(t.window_radius, 24.0);
}

#[test]
fn ui_tokens_default_type_fields() {
    let t = UiTokens::default();
    assert_eq!(t.family_sans.as_str(), "Inter, SF Pro, system-ui");
    assert_eq!(
        t.family_mono.as_str(),
        "JetBrains Mono, SF Mono, ui-monospace"
    );
    assert_eq!(t.size_11, 11);
    assert_eq!(t.size_12, 12);
    assert_eq!(t.size_13, 13);
    assert_eq!(t.size_14, 14);
    assert_eq!(t.size_16, 16);
    assert_eq!(t.size_20, 20);
    assert_eq!(t.size_24, 24);
    assert_eq!(t.weight_regular, 400);
    assert_eq!(t.weight_medium, 500);
    assert_eq!(t.weight_semibold, 600);
}

//
// parse_tokens_from_toml — partial TOML override behavior
//

#[test]
fn parse_tokens_from_toml_partial_overrides_only_specified_fields() {
    let content = "[layout]\nwindow_width = 3840\n";
    let tokens = vb_ui_snapshot::tokens::parse_tokens_from_toml(content).expect("parse ok");
    assert_eq!(tokens.window_width, 3840);
    // Others still default
    assert_eq!(tokens.window_height, 1080);
    assert_eq!(tokens.outer_margin, 32);
}

#[test]
fn parse_tokens_from_toml_missing_section_leaves_defaults() {
    let content = "[color]\nsurface = \"#CC0000\"\n";
    let tokens = vb_ui_snapshot::tokens::parse_tokens_from_toml(content).expect("parse ok");
    assert_eq!(tokens.surface.as_str(), "#CC0000");
    // layout section missing → defaults
    assert_eq!(tokens.window_width, 1920);
}

#[test]
fn parse_tokens_from_toml_unknown_section_ignored() {
    let content = "[nonexistent]\nfoo = \"bar\"\n[layout]\nwindow_width = 800\n";
    let tokens = vb_ui_snapshot::tokens::parse_tokens_from_toml(content).expect("parse ok");
    assert_eq!(tokens.window_width, 800);
}

#[test]
fn parse_tokens_from_toml_radius_integer_overrides() {
    // Note: TOML integers require as_integer() not as_float() to retrieve
    // Using a float value to test the float path
    let content = "[radius]\nchip = 15.0\n";
    let tokens = vb_ui_snapshot::tokens::parse_tokens_from_toml(content).expect("parse ok");
    assert_eq!(tokens.chip_radius, 15.0);
    // Others default
    assert_eq!(tokens.control_radius, 12.0);
}

#[test]
fn parse_tokens_from_toml_type_weights() {
    let content = "[type]\nweight_regular = 300\nweight_medium = 600\nweight_semibold = 800\n";
    let tokens = vb_ui_snapshot::tokens::parse_tokens_from_toml(content).expect("parse ok");
    assert_eq!(tokens.weight_regular, 300);
    assert_eq!(tokens.weight_medium, 600);
    assert_eq!(tokens.weight_semibold, 800);
}

#[test]
fn parse_tokens_from_toml_rejects_invalid_toml() {
    let content = "this is not TOML {";
    let result = vb_ui_snapshot::tokens::parse_tokens_from_toml(content);
    assert!(result.is_err());
}

#[test]
fn parse_tokens_from_toml_empty_string_succeeds_with_all_defaults() {
    let tokens = vb_ui_snapshot::tokens::parse_tokens_from_toml("").expect("empty is valid toml");
    // All defaults because no overrides
    assert_eq!(tokens.window_width, 1920);
    assert_eq!(tokens.surface.as_str(), "#FFFFFF");
}
