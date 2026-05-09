// Generated from velvet_ui_tokens.toml — DO NOT EDIT

use crate::theme::colors;

#[derive(Debug, Clone, Copy)]
pub struct TokenColors {
    pub surface:        [f32; 4],
    pub text_primary:   [f32; 4],
    pub success:        [f32; 4],
    pub running:        [f32; 4],
    pub failure:        [f32; 4],
    pub taint:          [f32; 4],
    pub durable:        [f32; 4],
    pub warning:        [f32; 4],
}

pub fn hex_to_f32(c: &str) -> [f32; 4] {
    let hex = c.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    [r, g, b, 1.0]
}

pub const TOKENS: TokenColors = TokenColors {
    surface:      hex_to_f32("#FFFFFF"),
    text_primary: hex_to_f32("#101828"),
    success:      hex_to_f32("#16A66A"),
    running:      hex_to_f32("#1F7AF5"),
    failure:      hex_to_f32("#E5484D"),
    taint:        hex_to_f32("#8B5CF6"),
    durable:      hex_to_f32("#14B8A6"),
    warning:      hex_to_f32("#F59E0B"),
};

pub const LAYOUT: TokenLayout = TokenLayout {
    window_width:          1920,
    window_height:         1080,
    outer_margin:          32,
    sidebar_width:         246,
    top_bar_height:        78,
    content_gutter:        16,
    chip_radius:           10.0,
};

#[derive(Debug, Clone, Copy)]
pub struct TokenLayout {
    pub window_width:     u32,
    pub window_height:    u32,
    pub outer_margin:     u32,
    pub sidebar_width:    u32,
    pub top_bar_height:   u32,
    pub content_gutter:   u32,
    pub chip_radius:      f32,
}
