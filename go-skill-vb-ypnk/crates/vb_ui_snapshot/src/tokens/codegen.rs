#![forbid(unsafe_code)]

use alloc::string::String;
use super::UiTokens;

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
