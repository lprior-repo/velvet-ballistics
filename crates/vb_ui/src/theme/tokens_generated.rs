// Generated from velvet_ui_tokens.toml - DO NOT EDIT

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

pub const TOKENS: TokenColors = TokenColors {
    surface:      [1.000000, 1.000000, 1.000000, 1.0],
    text_primary: [0.062745, 0.094118, 0.156863, 1.0],
    success:      [0.086275, 0.650980, 0.415686, 1.0],
    running:      [0.121569, 0.478431, 0.960784, 1.0],
    failure:      [0.898039, 0.282353, 0.301961, 1.0],
    taint:        [0.545098, 0.360784, 0.964706, 1.0],
    durable:      [0.078431, 0.721569, 0.650980, 1.0],
    warning:      [0.960784, 0.619608, 0.043137, 1.0],
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
