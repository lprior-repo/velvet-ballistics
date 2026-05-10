#![forbid(unsafe_code)]

use crate::error::UiSnapshotError;
use alloc::{
    format,
    string::{String, ToString},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiTokens {
    pub window_width: u32,
    pub window_height: u32,
    pub outer_margin: u32,
    pub sidebar_width: u32,
    pub top_bar_height: u32,
    pub content_gutter: u32,
    pub inspector_width_min: u32,
    pub inspector_width_max: u32,
    pub bottom_timeline_min: u32,
    pub graph_canvas_min_width: u32,
    pub graph_canvas_min_height: u32,
    pub background_board: String,
    pub shell: String,
    pub surface: String,
    pub surface_glass: String,
    pub surface_muted: String,
    pub line_hair: String,
    pub line_soft: String,
    pub text_primary: String,
    pub text_secondary: String,
    pub text_tertiary: String,
    pub success: String,
    pub running: String,
    pub active_cyan: String,
    pub warning: String,
    pub failure: String,
    pub taint: String,
    pub durable: String,
    pub pending: String,
    pub chip_radius: f32,
    pub control_radius: f32,
    pub card_min_radius: f32,
    pub card_radius: f32,
    pub card_max_radius: f32,
    pub panel_radius: f32,
    pub window_radius: f32,
    pub family_sans: String,
    pub family_mono: String,
    pub size_11: u32,
    pub size_12: u32,
    pub size_13: u32,
    pub size_14: u32,
    pub size_16: u32,
    pub size_20: u32,
    pub size_24: u32,
    pub weight_regular: u32,
    pub weight_medium: u32,
    pub weight_semibold: u32,
}

impl Default for UiTokens {
    fn default() -> Self {
        Self {
            window_width: 1920,
            window_height: 1080,
            outer_margin: 32,
            sidebar_width: 246,
            top_bar_height: 78,
            content_gutter: 16,
            inspector_width_min: 360,
            inspector_width_max: 420,
            bottom_timeline_min: 220,
            graph_canvas_min_width: 720,
            graph_canvas_min_height: 520,
            background_board: "#F4F6F8".to_string(),
            shell: "#F8FAFC".to_string(),
            surface: "#FFFFFF".to_string(),
            surface_glass: "#FFFFFFCC".to_string(),
            surface_muted: "#F2F5F8".to_string(),
            line_hair: "#DDE3EA".to_string(),
            line_soft: "#E8EDF2".to_string(),
            text_primary: "#101828".to_string(),
            text_secondary: "#475467".to_string(),
            text_tertiary: "#7A8796".to_string(),
            success: "#16A66A".to_string(),
            running: "#1F7AF5".to_string(),
            active_cyan: "#19A7CE".to_string(),
            warning: "#F59E0B".to_string(),
            failure: "#E5484D".to_string(),
            taint: "#8B5CF6".to_string(),
            durable: "#14B8A6".to_string(),
            pending: "#98A2B3".to_string(),
            chip_radius: 10.0,
            control_radius: 12.0,
            card_min_radius: 14.0,
            card_radius: 16.0,
            card_max_radius: 22.0,
            panel_radius: 20.0,
            window_radius: 24.0,
            family_sans: "Inter, SF Pro, system-ui".to_string(),
            family_mono: "JetBrains Mono, SF Mono, ui-monospace".to_string(),
            size_11: 11,
            size_12: 12,
            size_13: 13,
            size_14: 14,
            size_16: 16,
            size_20: 20,
            size_24: 24,
            weight_regular: 400,
            weight_medium: 500,
            weight_semibold: 600,
        }
    }
}

#[cfg(feature = "std")]
pub fn load_tokens_from_file(path: &Path) -> Result<UiTokens, UiSnapshotError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| UiSnapshotError::IoError(format!("Failed to read {}: {e}", path.display())))?;

    parse_tokens_from_toml(&content)
}

pub fn parse_tokens_from_toml(content: &str) -> Result<UiTokens, UiSnapshotError> {
    let value = toml::from_str::<toml::Value>(content)
        .map_err(|e| UiSnapshotError::TokenParseError(format!("TOML parse error: {e}")))?;

    let mut tokens = UiTokens::default();

    if let Some(table) = value.get("layout").and_then(|t| t.as_table()) {
        if let Some(v) = table.get("window_width").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.window_width, v);
        }
        if let Some(v) = table.get("window_height").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.window_height, v);
        }
        if let Some(v) = table.get("outer_margin").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.outer_margin, v);
        }
        if let Some(v) = table.get("sidebar_width").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.sidebar_width, v);
        }
        if let Some(v) = table.get("top_bar_height").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.top_bar_height, v);
        }
        if let Some(v) = table.get("content_gutter").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.content_gutter, v);
        }
        if let Some(v) = table
            .get("inspector_width_min")
            .and_then(|v| v.as_integer())
        {
            set_u32(&mut tokens.inspector_width_min, v);
        }
        if let Some(v) = table
            .get("inspector_width_max")
            .and_then(|v| v.as_integer())
        {
            set_u32(&mut tokens.inspector_width_max, v);
        }
        if let Some(v) = table
            .get("bottom_timeline_min")
            .and_then(|v| v.as_integer())
        {
            set_u32(&mut tokens.bottom_timeline_min, v);
        }
        if let Some(v) = table
            .get("graph_canvas_min_width")
            .and_then(|v| v.as_integer())
        {
            set_u32(&mut tokens.graph_canvas_min_width, v);
        }
        if let Some(v) = table
            .get("graph_canvas_min_height")
            .and_then(|v| v.as_integer())
        {
            set_u32(&mut tokens.graph_canvas_min_height, v);
        }
    }

    if let Some(table) = value.get("color").and_then(|t| t.as_table()) {
        macro_rules! get_color {
            ($field:ident) => {
                if let Some(v) = table.get(stringify!($field)).and_then(|v| v.as_str()) {
                    tokens.$field = v.to_string();
                }
            };
        }
        get_color!(background_board);
        get_color!(shell);
        get_color!(surface);
        get_color!(surface_glass);
        get_color!(surface_muted);
        get_color!(line_hair);
        get_color!(line_soft);
        get_color!(text_primary);
        get_color!(text_secondary);
        get_color!(text_tertiary);
        get_color!(success);
        get_color!(running);
        get_color!(active_cyan);
        get_color!(warning);
        get_color!(failure);
        get_color!(taint);
        get_color!(durable);
        get_color!(pending);
    }

    if let Some(table) = value.get("radius").and_then(|t| t.as_table()) {
        if let Some(v) = table.get("chip").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.chip_radius, v);
        }
        if let Some(v) = table.get("control").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.control_radius, v);
        }
        if let Some(v) = table.get("card_min").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.card_min_radius, v);
        }
        if let Some(v) = table.get("card").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.card_radius, v);
        }
        if let Some(v) = table.get("card_max").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.card_max_radius, v);
        }
        if let Some(v) = table.get("panel").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.panel_radius, v);
        }
        if let Some(v) = table.get("window").and_then(|v| v.as_float()) {
            set_f32(&mut tokens.window_radius, v);
        }
    }

    if let Some(table) = value.get("type").and_then(|t| t.as_table()) {
        if let Some(v) = table.get("family_sans").and_then(|v| v.as_str()) {
            tokens.family_sans = v.to_string();
        }
        if let Some(v) = table.get("family_mono").and_then(|v| v.as_str()) {
            tokens.family_mono = v.to_string();
        }
        macro_rules! get_size {
            ($field:ident) => {
                if let Some(v) = table.get(stringify!($field)).and_then(|v| v.as_integer()) {
                    set_u32(&mut tokens.$field, v);
                }
            };
        }
        get_size!(size_11);
        get_size!(size_12);
        get_size!(size_13);
        get_size!(size_14);
        get_size!(size_16);
        get_size!(size_20);
        get_size!(size_24);
        if let Some(v) = table.get("weight_regular").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.weight_regular, v);
        }
        if let Some(v) = table.get("weight_medium").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.weight_medium, v);
        }
        if let Some(v) = table.get("weight_semibold").and_then(|v| v.as_integer()) {
            set_u32(&mut tokens.weight_semibold, v);
        }
    }

    Ok(tokens)
}

fn set_u32(target: &mut u32, value: i64) {
    if let Ok(parsed) = u32::try_from(value) {
        *target = parsed;
    }
}

fn set_f32(target: &mut f32, value: f64) {
    if let Ok(parsed) = value.to_string().parse::<f32>()
        && parsed.is_finite()
    {
        *target = parsed;
    }
}

pub fn tokens_to_rust_constants(tokens: &UiTokens) -> String {
    let mut out = String::new();
    out.push_str("// Generated from velvet_ui_tokens.toml - DO NOT EDIT\n\n");

    out.push_str("#[derive(Debug, Clone, Copy)]\npub struct TokenColors {\n");
    out.push_str("    pub surface:        [f32; 4],\n");
    out.push_str("    pub text_primary:   [f32; 4],\n");
    out.push_str("    pub success:        [f32; 4],\n");
    out.push_str("    pub running:        [f32; 4],\n");
    out.push_str("    pub failure:        [f32; 4],\n");
    out.push_str("    pub taint:          [f32; 4],\n");
    out.push_str("    pub durable:        [f32; 4],\n");
    out.push_str("    pub warning:        [f32; 4],\n");
    out.push_str("}\n\n");

    out.push_str("pub const TOKENS: TokenColors = TokenColors {\n");
    out.push_str(&format!(
        "    surface:      {},\n",
        hex_to_f32_literal(&tokens.surface)
    ));
    out.push_str(&format!(
        "    text_primary: {},\n",
        hex_to_f32_literal(&tokens.text_primary)
    ));
    out.push_str(&format!(
        "    success:      {},\n",
        hex_to_f32_literal(&tokens.success)
    ));
    out.push_str(&format!(
        "    running:      {},\n",
        hex_to_f32_literal(&tokens.running)
    ));
    out.push_str(&format!(
        "    failure:      {},\n",
        hex_to_f32_literal(&tokens.failure)
    ));
    out.push_str(&format!(
        "    taint:        {},\n",
        hex_to_f32_literal(&tokens.taint)
    ));
    out.push_str(&format!(
        "    durable:      {},\n",
        hex_to_f32_literal(&tokens.durable)
    ));
    out.push_str(&format!(
        "    warning:      {},\n",
        hex_to_f32_literal(&tokens.warning)
    ));
    out.push_str("};\n\n");

    out.push_str("pub const LAYOUT: TokenLayout = TokenLayout {\n");
    out.push_str(&format!(
        "    window_width:          {},\n",
        tokens.window_width
    ));
    out.push_str(&format!(
        "    window_height:         {},\n",
        tokens.window_height
    ));
    out.push_str(&format!(
        "    outer_margin:          {},\n",
        tokens.outer_margin
    ));
    out.push_str(&format!(
        "    sidebar_width:         {},\n",
        tokens.sidebar_width
    ));
    out.push_str(&format!(
        "    top_bar_height:        {},\n",
        tokens.top_bar_height
    ));
    out.push_str(&format!(
        "    content_gutter:        {},\n",
        tokens.content_gutter
    ));
    out.push_str(&format!(
        "    chip_radius:           {:.1},\n",
        tokens.chip_radius
    ));
    out.push_str("};\n\n");

    out.push_str("#[derive(Debug, Clone, Copy)]\npub struct TokenLayout {\n");
    out.push_str("    pub window_width:     u32,\n");
    out.push_str("    pub window_height:    u32,\n");
    out.push_str("    pub outer_margin:     u32,\n");
    out.push_str("    pub sidebar_width:    u32,\n");
    out.push_str("    pub top_bar_height:   u32,\n");
    out.push_str("    pub content_gutter:   u32,\n");
    out.push_str("    pub chip_radius:      f32,\n");
    out.push_str("}\n");

    out
}

fn hex_to_f32_literal(hex: &str) -> String {
    match parse_hex_rgb(hex) {
        Some((r, g, b)) => format!(
            "[{:.6}, {:.6}, {:.6}, 1.0]",
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0
        ),
        None => "[0.0, 0.0, 0.0, 1.0]".to_string(),
    }
}

fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let bytes = hex.trim_start_matches('#').as_bytes();
    let r = parse_hex_byte(bytes.get(0..2)?)?;
    let g = parse_hex_byte(bytes.get(2..4)?)?;
    let b = parse_hex_byte(bytes.get(4..6)?)?;
    Some((r, g, b))
}

fn parse_hex_byte(bytes: &[u8]) -> Option<u8> {
    let text = core::str::from_utf8(bytes).ok()?;
    u8::from_str_radix(text, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn require_value<T: PartialEq>(
        actual: T,
        expected: T,
        field: &str,
    ) -> Result<(), UiSnapshotError> {
        if actual == expected {
            Ok(())
        } else {
            Err(UiSnapshotError::TokenParseError(format!(
                "Unexpected parsed value for {field}"
            )))
        }
    }

    #[test]
    fn test_parse_tokens_from_toml() -> Result<(), UiSnapshotError> {
        let content = r##"
[layout]
window_width = 1920
window_height = 1080
outer_margin = 32
sidebar_width = 246
top_bar_height = 78

[radius]
chip = 10
control = 12
card = 16

[color]
surface = "#FFFFFF"
text_primary = "#101828"
success = "#16A66A"
running = "#1F7AF5"
failure = "#E5484D"
taint = "#8B5CF6"
durable = "#14B8A6"
warning = "#F59E0B"

[type]
family_sans = "Inter"
size_14 = 14
"##;
        let tokens = parse_tokens_from_toml(content)?;
        require_value(tokens.window_width, 1920, "window_width")?;
        require_value(tokens.window_height, 1080, "window_height")?;
        require_value(tokens.outer_margin, 32, "outer_margin")?;
        require_value(tokens.sidebar_width, 246, "sidebar_width")?;
        require_value(tokens.top_bar_height, 78, "top_bar_height")?;
        require_value(
            tokens.chip_radius.to_bits(),
            10.0_f32.to_bits(),
            "chip_radius",
        )?;
        require_value(tokens.surface.as_str(), "#FFFFFF", "surface")?;
        require_value(tokens.text_primary.as_str(), "#101828", "text_primary")?;
        require_value(tokens.success.as_str(), "#16A66A", "success")?;
        require_value(tokens.running.as_str(), "#1F7AF5", "running")?;
        require_value(tokens.failure.as_str(), "#E5484D", "failure")?;
        require_value(tokens.taint.as_str(), "#8B5CF6", "taint")?;
        require_value(tokens.durable.as_str(), "#14B8A6", "durable")?;
        require_value(tokens.warning.as_str(), "#F59E0B", "warning")?;
        require_value(tokens.family_sans.as_str(), "Inter", "family_sans")?;
        require_value(tokens.size_14, 14, "size_14")?;

        Ok(())
    }

    #[test]
    fn test_parse_tokens_partial_toml_uses_defaults() -> Result<(), UiSnapshotError> {
        let tokens = parse_tokens_from_toml("")?;
        // All fields should be default values
        assert_eq!(tokens.window_width, 1920);
        assert_eq!(tokens.window_height, 1080);
        assert_eq!(tokens.surface.as_str(), "#FFFFFF");
        Ok(())
    }

    #[test]
    fn test_parse_tokens_invalid_toml() {
        let result = parse_tokens_from_toml("not valid toml {{{{");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tokens_with_radius_table() -> Result<(), UiSnapshotError> {
        let content = r##"
[radius]
chip = 15.5
control = 8.0
card_min = 5.0
card = 20.0
card_max = 30.0
panel = 25.0
window = 35.0
"##;
        let tokens = parse_tokens_from_toml(content)?;
        require_value(tokens.chip_radius.to_bits(), 15.5_f32.to_bits(), "chip_radius")?;
        require_value(tokens.control_radius.to_bits(), 8.0_f32.to_bits(), "control_radius")?;
        require_value(tokens.card_min_radius.to_bits(), 5.0_f32.to_bits(), "card_min_radius")?;
        require_value(tokens.card_radius.to_bits(), 20.0_f32.to_bits(), "card_radius")?;
        require_value(tokens.card_max_radius.to_bits(), 30.0_f32.to_bits(), "card_max_radius")?;
        require_value(tokens.panel_radius.to_bits(), 25.0_f32.to_bits(), "panel_radius")?;
        require_value(tokens.window_radius.to_bits(), 35.0_f32.to_bits(), "window_radius")?;
        Ok(())
    }

    #[test]
    fn test_parse_tokens_with_type_table() -> Result<(), UiSnapshotError> {
        let content = r##"
[type]
family_sans = "Roboto"
family_mono = "Fira Code"
size_11 = 11
size_12 = 12
size_13 = 13
size_16 = 16
size_20 = 20
size_24 = 24
weight_regular = 300
weight_medium = 600
weight_semibold = 700
"##;
        let tokens = parse_tokens_from_toml(content)?;
        require_value(tokens.family_sans.as_str(), "Roboto", "family_sans")?;
        require_value(tokens.family_mono.as_str(), "Fira Code", "family_mono")?;
        require_value(tokens.weight_regular, 300, "weight_regular")?;
        require_value(tokens.weight_medium, 600, "weight_medium")?;
        require_value(tokens.weight_semibold, 700, "weight_semibold")?;
        Ok(())
    }

    #[test]
    fn test_parse_tokens_all_color_fields() -> Result<(), UiSnapshotError> {
        let content = r##"
[color]
background_board = "#000000"
shell = "#111111"
surface = "#222222"
surface_glass = "#333333CC"
surface_muted = "#444444"
line_hair = "#555555"
line_soft = "#666666"
text_primary = "#777777"
text_secondary = "#888888"
text_tertiary = "#999999"
success = "#AAAAAA"
running = "#BBBBBB"
active_cyan = "#CCCCCC"
warning = "#DDDDDD"
failure = "#EEEEEE"
taint = "#FFFFFF"
durable = "#ABABAB"
pending = "#BCBCBC"
"##;
        let tokens = parse_tokens_from_toml(content)?;
        assert_eq!(tokens.background_board, "#000000");
        assert_eq!(tokens.shell, "#111111");
        assert_eq!(tokens.text_secondary, "#888888");
        assert_eq!(tokens.text_tertiary, "#999999");
        assert_eq!(tokens.active_cyan, "#CCCCCC");
        Ok(())
    }

    #[test]
    fn test_parse_tokens_layout_fields() -> Result<(), UiSnapshotError> {
        let content = r##"
[layout]
window_width = 3840
window_height = 2160
outer_margin = 64
sidebar_width = 300
top_bar_height = 100
content_gutter = 24
inspector_width_min = 400
inspector_width_max = 600
bottom_timeline_min = 300
graph_canvas_min_width = 800
graph_canvas_min_height = 600
"##;
        let tokens = parse_tokens_from_toml(content)?;
        require_value(tokens.window_width, 3840, "window_width")?;
        require_value(tokens.window_height, 2160, "window_height")?;
        require_value(tokens.outer_margin, 64, "outer_margin")?;
        require_value(tokens.sidebar_width, 300, "sidebar_width")?;
        require_value(tokens.top_bar_height, 100, "top_bar_height")?;
        require_value(tokens.content_gutter, 24, "content_gutter")?;
        require_value(tokens.inspector_width_min, 400, "inspector_width_min")?;
        require_value(tokens.inspector_width_max, 600, "inspector_width_max")?;
        require_value(tokens.bottom_timeline_min, 300, "bottom_timeline_min")?;
        require_value(tokens.graph_canvas_min_width, 800, "graph_canvas_min_width")?;
        require_value(tokens.graph_canvas_min_height, 600, "graph_canvas_min_height")?;
        Ok(())
    }

    #[test]
    fn test_set_u32_converts_valid_i64() {
        let mut val = 0u32;
        super::set_u32(&mut val, 42);
        assert_eq!(val, 42);
    }

    #[test]
    fn test_set_u32_ignores_negative_i64() {
        let mut val = 999u32;
        super::set_u32(&mut val, -1);
        assert_eq!(val, 999); // unchanged
    }

    #[test]
    fn test_set_u32_ignores_overflow_i64() {
        let mut val = 0u32;
        super::set_u32(&mut val, i64::MAX);
        assert_eq!(val, 0); // unchanged
    }

    #[test]
    fn test_set_f32_converts_valid_f64() {
        let mut val = 0.0_f32;
        super::set_f32(&mut val, 3.14159);
        assert!((val - 3.14159).abs() < 0.0001);
    }

    #[test]
    fn test_set_f32_ignores_nan() {
        let mut val = 1.0_f32;
        super::set_f32(&mut val, f64::NAN);
        assert_eq!(val, 1.0); // unchanged
    }

    #[test]
    fn test_set_f32_ignores_infinity() {
        let mut val = 1.0_f32;
        super::set_f32(&mut val, f64::INFINITY);
        assert_eq!(val, 1.0); // unchanged
    }

    #[test]
    fn test_parse_hex_rgb_valid() {
        assert_eq!(super::parse_hex_rgb("#FF8040"), Some((0xFF, 0x80, 0x40)));
        // 7 chars gets parsed as 3 pairs: 9A, BC, DE — trailing "F" is dropped
        assert_eq!(super::parse_hex_rgb("9ABCDEF"), Some((0x9A, 0xBC, 0xDE)));
    }

    #[test]
    fn test_parse_hex_rgb_without_hash() {
        assert_eq!(super::parse_hex_rgb("AABBCC"), Some((0xAA, 0xBB, 0xCC)));
    }

    #[test]
    fn test_parse_hex_rgb_invalid_too_short() {
        assert_eq!(super::parse_hex_rgb("#ABC"), None);
    }

    #[test]
    fn test_parse_hex_rgb_invalid_empty() {
        assert_eq!(super::parse_hex_rgb(""), None);
    }

    #[test]
    fn test_parse_hex_byte_valid() {
        assert_eq!(super::parse_hex_byte(b"FF"), Some(255));
        assert_eq!(super::parse_hex_byte(b"00"), Some(0));
        assert_eq!(super::parse_hex_byte(b"1A"), Some(26));
        assert_eq!(super::parse_hex_byte(b"aB"), Some(171));
    }

    #[test]
    fn test_parse_hex_byte_invalid() {
        assert_eq!(super::parse_hex_byte(b"GG"), None);
        assert_eq!(super::parse_hex_byte(b""), None);
    }

    #[test]
    fn test_hex_to_f32_literal_converts_correctly() {
        let literal = super::hex_to_f32_literal("#FFFFFF");
        assert!(literal.contains("1.0")); // R=255/255=1.0
    }

    #[test]
    fn test_hex_to_f32_literal_black() {
        let literal = super::hex_to_f32_literal("#000000");
        assert!(literal.contains("0.0"));
    }

    #[test]
    fn test_hex_to_f32_literal_invalid_returns_black() {
        let literal = super::hex_to_f32_literal("not hex");
        assert_eq!(literal, "[0.0, 0.0, 0.0, 1.0]".to_string());
    }

    #[test]
    fn test_tokens_to_rust_constants_produces_output() {
        let tokens = UiTokens::default();
        let output = super::tokens_to_rust_constants(&tokens);
        assert!(output.contains("pub struct TokenColors"));
        assert!(output.contains("pub struct TokenLayout"));
        assert!(output.contains("pub const TOKENS: TokenColors"));
        assert!(output.contains("pub const LAYOUT: TokenLayout"));
    }

    #[test]
    fn test_tokens_to_rust_constants_contains_color_values() {
        let tokens = UiTokens::default();
        let output = super::tokens_to_rust_constants(&tokens);
        // Default surface is #FFFFFF which is [1.0, 1.0, 1.0, 1.0]
        assert!(output.contains("surface"));
        assert!(output.contains("text_primary"));
        assert!(output.contains("success"));
    }

    #[test]
    fn test_default_tokens_has_expected_values() {
        let tokens = UiTokens::default();
        assert_eq!(tokens.window_width, 1920);
        assert_eq!(tokens.window_height, 1080);
        assert_eq!(tokens.outer_margin, 32);
        assert_eq!(tokens.chip_radius, 10.0);
        assert_eq!(tokens.family_sans, "Inter, SF Pro, system-ui");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_load_tokens_from_file_missing_file() {
        use std::path::Path;
        let result = super::load_tokens_from_file(Path::new("/nonexistent/path/tokens.toml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        let display = alloc::format!("{err}");
        assert!(display.contains("Failed to read"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_load_tokens_from_file_invalid_toml() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "invalid {{{")?;
        let result = super::load_tokens_from_file(&path);
        assert!(result.is_err());
        Ok(())
    }
}
