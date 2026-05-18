#![forbid(unsafe_code)]

use super::sections::{
    ColorSection, LayoutSection, ParsedTokens, RadiusSection, ShadowSection, SpaceSection,
    TypeFamilySection, TypeSizeSection, TypeWeightSection,
};
use crate::tokens::parse::TOKENS_TOML;
use std::sync::OnceLock;

static CACHED_PARSED: OnceLock<ParsedTokens> = OnceLock::new();
static CACHED_FALLBACK: OnceLock<ParsedTokens> = OnceLock::new();

fn get_parsed() -> &'static ParsedTokens {
    CACHED_PARSED.get_or_init(|| match ParsedTokens::from_toml(TOKENS_TOML) {
        Ok(parsed) => parsed,
        Err(_) => get_fallback().clone(),
    })
}

fn get_fallback() -> &'static ParsedTokens {
    CACHED_FALLBACK.get_or_init(|| ParsedTokens {
        color: ColorSection {
            background_board: [0.957, 0.965, 0.973, 1.0],
            shell: [0.973, 0.980, 0.988, 1.0],
            surface: [1.0, 1.0, 1.0, 1.0],
            surface_glass: [1.0, 1.0, 1.0, 0.8],
            surface_muted: [0.949, 0.961, 0.973, 1.0],
            line_hair: [0.867, 0.890, 0.918, 1.0],
            line_soft: [0.910, 0.929, 0.949, 1.0],
            text_primary: [0.063, 0.094, 0.157, 1.0],
            text_secondary: [0.278, 0.337, 0.404, 1.0],
            text_tertiary: [0.478, 0.529, 0.588, 1.0],
            success: [0.086, 0.651, 0.416, 1.0],
            running: [0.122, 0.478, 0.961, 1.0],
            active_cyan: [0.098, 0.655, 0.808, 1.0],
            warning: [0.961, 0.620, 0.043, 1.0],
            failure: [0.898, 0.282, 0.302, 1.0],
            taint: [0.545, 0.361, 0.965, 1.0],
            durable: [0.078, 0.722, 0.651, 1.0],
            pending: [0.596, 0.635, 0.702, 1.0],
        },
        layout: LayoutSection {
            sidebar_width: 246.0,
            top_bar_height: 78.0,
            outer_margin: 32.0,
            content_gutter: 16.0,
            inspector_width_min: 360.0,
            inspector_width_max: 420.0,
            bottom_timeline_min: 220.0,
            graph_canvas_min_width: 720.0,
            graph_canvas_min_height: 520.0,
            window_width: 1920.0,
            window_height: 1080.0,
        },
        radius: RadiusSection {
            chip: 10.0,
            control: 12.0,
            card_min: 14.0,
            card: 16.0,
            card_max: 22.0,
            panel: 20.0,
            window: 24.0,
        },
        shadow: ShadowSection {
            card: String::from("0 8 24 rgba(16,24,40,0.08)"),
            window: String::from("0 20 60 rgba(16,24,40,0.14)"),
            focus: String::from("0 0 0 4 rgba(31,122,245,0.14)"),
            failure: String::from("0 0 0 4 rgba(229,72,77,0.12)"),
            taint: String::from("0 0 0 4 rgba(139,92,246,0.12)"),
        },
        space: SpaceSection {
            px_4: 4.0,
            px_8: 8.0,
            px_12: 12.0,
            px_16: 16.0,
            px_20: 20.0,
            px_24: 24.0,
            px_32: 32.0,
            px_40: 40.0,
        },
        type_family: TypeFamilySection {
            sans: String::from("Inter, SF Pro, system-ui"),
            mono: String::from("JetBrains Mono, SF Mono, ui-monospace"),
        },
        type_size: TypeSizeSection {
            size_11: 11,
            size_12: 12,
            size_13: 13,
            size_14: 14,
            size_16: 16,
            size_20: 20,
            size_24: 24,
        },
        type_weight: TypeWeightSection {
            regular: 400,
            medium: 500,
            semibold: 600,
        },
    })
}

fn get_color() -> &'static ColorSection {
    &get_parsed().color
}

pub fn background_board() -> [f32; 4] {
    get_color().background_board
}
pub fn shell() -> [f32; 4] {
    get_color().shell
}
pub fn surface() -> [f32; 4] {
    get_color().surface
}
pub fn surface_glass() -> [f32; 4] {
    get_color().surface_glass
}
pub fn surface_muted() -> [f32; 4] {
    get_color().surface_muted
}
pub fn line_hair() -> [f32; 4] {
    get_color().line_hair
}
pub fn line_soft() -> [f32; 4] {
    get_color().line_soft
}
pub fn text_primary() -> [f32; 4] {
    get_color().text_primary
}
pub fn text_secondary() -> [f32; 4] {
    get_color().text_secondary
}
pub fn text_tertiary() -> [f32; 4] {
    get_color().text_tertiary
}
pub fn success() -> [f32; 4] {
    get_color().success
}
pub fn running() -> [f32; 4] {
    get_color().running
}
pub fn active_cyan() -> [f32; 4] {
    get_color().active_cyan
}
pub fn warning() -> [f32; 4] {
    get_color().warning
}
pub fn failure() -> [f32; 4] {
    get_color().failure
}
pub fn taint() -> [f32; 4] {
    get_color().taint
}
pub fn durable() -> [f32; 4] {
    get_color().durable
}
pub fn pending() -> [f32; 4] {
    get_color().pending
}
