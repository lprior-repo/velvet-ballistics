#![forbid(unsafe_code)]

use crate::error::Error;

pub const TOKENS_TOML: &str = include_str!("../../../design/tokens/velvet_ui_tokens.toml");

fn parse_hex(hex: &str) -> Result<[f32; 4], Error> {
    let hex = hex.trim();
    let hex = match hex.strip_prefix('#') {
        Some(stripped) => stripped,
        None => hex,
    };

    fn nybble(b: u8) -> Result<u8, Error> {
        match b {
            b'0' => Ok(0),
            b'1' => Ok(1),
            b'2' => Ok(2),
            b'3' => Ok(3),
            b'4' => Ok(4),
            b'5' => Ok(5),
            b'6' => Ok(6),
            b'7' => Ok(7),
            b'8' => Ok(8),
            b'9' => Ok(9),
            b'A' | b'a' => Ok(10),
            b'B' | b'b' => Ok(11),
            b'C' | b'c' => Ok(12),
            b'D' | b'd' => Ok(13),
            b'E' | b'e' => Ok(14),
            b'F' | b'f' => Ok(15),
            _ => Err(Error::TokenParseError("invalid hex char".into())),
        }
    }

    let bytes = hex.as_bytes();
    let len = bytes.len();

    fn parse_pair(b0: u8, b1: u8) -> Result<u8, Error> {
        let hi = nybble(b0)?;
        let lo = nybble(b1)?;
        hi.checked_mul(16)
            .and_then(|scaled| scaled.checked_add(lo))
            .ok_or_else(|| Error::TokenParseError("invalid hex pair".into()))
    }

    fn parse_pair_at(bytes: &[u8], offset: usize, label: &str) -> Result<u8, Error> {
        let next_offset = offset.saturating_add(1);
        match (bytes.get(offset), bytes.get(next_offset)) {
            (Some(first), Some(second)) => parse_pair(*first, *second)
                .map_err(|_| Error::TokenParseError(format!("invalid hex {label}"))),
            _ => Err(Error::TokenParseError("hex too short".into())),
        }
    }

    if len == 6 {
        let r = parse_pair_at(bytes, 0, "r")?;
        let g = parse_pair_at(bytes, 2, "g")?;
        let b = parse_pair_at(bytes, 4, "b")?;
        Ok([
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ])
    } else if len == 8 {
        let r = parse_pair_at(bytes, 0, "r")?;
        let g = parse_pair_at(bytes, 2, "g")?;
        let b = parse_pair_at(bytes, 4, "b")?;
        let a = parse_pair_at(bytes, 6, "a")?;
        Ok([
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        ])
    } else {
        Err(Error::TokenParseError(format!("invalid hex length: {len}")))
    }
}

pub struct Tokens;

impl Tokens {
    pub fn parse() -> Result<ParsedTokens, Error> {
        ParsedTokens::from_toml(TOKENS_TOML)
    }
}

#[derive(Clone)]
pub struct ParsedTokens {
    pub color: ColorSection,
    pub layout: LayoutSection,
    pub radius: RadiusSection,
    pub shadow: ShadowSection,
    pub space: SpaceSection,
    pub type_family: TypeFamilySection,
    pub type_size: TypeSizeSection,
    pub type_weight: TypeWeightSection,
}

#[derive(Clone)]
pub struct ColorSection {
    pub background_board: [f32; 4],
    pub shell: [f32; 4],
    pub surface: [f32; 4],
    pub surface_glass: [f32; 4],
    pub surface_muted: [f32; 4],
    pub line_hair: [f32; 4],
    pub line_soft: [f32; 4],
    pub text_primary: [f32; 4],
    pub text_secondary: [f32; 4],
    pub text_tertiary: [f32; 4],
    pub success: [f32; 4],
    pub running: [f32; 4],
    pub active_cyan: [f32; 4],
    pub warning: [f32; 4],
    pub failure: [f32; 4],
    pub taint: [f32; 4],
    pub durable: [f32; 4],
    pub pending: [f32; 4],
}

#[derive(Clone)]
pub struct LayoutSection {
    pub sidebar_width: f64,
    pub top_bar_height: f64,
    pub outer_margin: f64,
    pub content_gutter: f64,
    pub inspector_width_min: f64,
    pub inspector_width_max: f64,
    pub bottom_timeline_min: f64,
    pub graph_canvas_min_width: f64,
    pub graph_canvas_min_height: f64,
    pub window_width: f64,
    pub window_height: f64,
}

#[derive(Clone)]
pub struct RadiusSection {
    pub chip: f64,
    pub control: f64,
    pub card_min: f64,
    pub card: f64,
    pub card_max: f64,
    pub panel: f64,
    pub window: f64,
}

#[derive(Clone)]
pub struct ShadowSection {
    pub card: String,
    pub window: String,
    pub focus: String,
    pub failure: String,
    pub taint: String,
}

#[derive(Clone)]
pub struct SpaceSection {
    pub px_4: f64,
    pub px_8: f64,
    pub px_12: f64,
    pub px_16: f64,
    pub px_20: f64,
    pub px_24: f64,
    pub px_32: f64,
    pub px_40: f64,
}

#[derive(Clone)]
pub struct TypeFamilySection {
    pub sans: String,
    pub mono: String,
}

#[derive(Clone)]
pub struct TypeSizeSection {
    pub size_11: u32,
    pub size_12: u32,
    pub size_13: u32,
    pub size_14: u32,
    pub size_16: u32,
    pub size_20: u32,
    pub size_24: u32,
}

#[derive(Clone)]
pub struct TypeWeightSection {
    pub regular: u32,
    pub medium: u32,
    pub semibold: u32,
}

impl ParsedTokens {
    pub fn from_toml(toml_str: &str) -> Result<Self, Error> {
        let table: toml::Value = toml_str
            .parse::<toml::Value>()
            .map_err(|e| Error::InvalidToken(e.to_string()))?;

        let get_color = |key: &str| -> Result<[f32; 4], Error> {
            let val = table
                .get("color")
                .and_then(|c| c.get(key))
                .ok_or_else(|| Error::InvalidToken(format!("missing color.{key}")))?;
            let hex = val
                .as_str()
                .ok_or_else(|| Error::TokenParseError(format!("color.{key} not string")))?;
            parse_hex(hex)
        };

        let get_f64 = |section: &str, key: &str| -> Result<f64, Error> {
            let val = table
                .get(section)
                .and_then(|s| s.get(key))
                .ok_or_else(|| Error::InvalidToken(format!("missing {section}.{key}")))?;
            if let Some(f) = val.as_float() {
                Ok(f)
            } else if let Some(i) = val.as_integer() {
                // TOML config values are small positive (246, 78, 16, etc.)
                // Converting via i32 is safe for our known-small values
                let i32_val = i32::try_from(i)
                    .map_err(|_| Error::TokenParseError(format!("{section}.{key} overflow")))?;
                let converted = f64::from(i32_val);
                Ok(converted)
            } else {
                Err(Error::TokenParseError(format!(
                    "{section}.{key} not number"
                )))
            }
        };

        let get_u32 = |section: &str, key: &str| -> Result<u32, Error> {
            let val = table
                .get(section)
                .and_then(|s| s.get(key))
                .ok_or_else(|| Error::InvalidToken(format!("missing {section}.{key}")))?;
            val.as_integer()
                .and_then(|i| u32::try_from(i).ok())
                .ok_or_else(|| Error::TokenParseError(format!("{section}.{key} not integer")))
        };

        let get_str = |section: &str, key: &str| -> Result<String, Error> {
            let val = table
                .get(section)
                .and_then(|s| s.get(key))
                .ok_or_else(|| Error::InvalidToken(format!("missing {section}.{key}")))?;
            val.as_str()
                .map(str::trim)
                .map(|s| s.trim_matches('"').to_string())
                .ok_or_else(|| Error::TokenParseError(format!("{section}.{key} not string")))
        };

        Ok(Self {
            color: ColorSection {
                background_board: get_color("background_board")?,
                shell: get_color("shell")?,
                surface: get_color("surface")?,
                surface_glass: get_color("surface_glass")?,
                surface_muted: get_color("surface_muted")?,
                line_hair: get_color("line_hair")?,
                line_soft: get_color("line_soft")?,
                text_primary: get_color("text_primary")?,
                text_secondary: get_color("text_secondary")?,
                text_tertiary: get_color("text_tertiary")?,
                success: get_color("success")?,
                running: get_color("running")?,
                active_cyan: get_color("active_cyan")?,
                warning: get_color("warning")?,
                failure: get_color("failure")?,
                taint: get_color("taint")?,
                durable: get_color("durable")?,
                pending: get_color("pending")?,
            },
            layout: LayoutSection {
                sidebar_width: get_f64("layout", "sidebar_width")?,
                top_bar_height: get_f64("layout", "top_bar_height")?,
                outer_margin: get_f64("layout", "outer_margin")?,
                content_gutter: get_f64("layout", "content_gutter")?,
                inspector_width_min: get_f64("layout", "inspector_width_min")?,
                inspector_width_max: get_f64("layout", "inspector_width_max")?,
                bottom_timeline_min: get_f64("layout", "bottom_timeline_min")?,
                graph_canvas_min_width: get_f64("layout", "graph_canvas_min_width")?,
                graph_canvas_min_height: get_f64("layout", "graph_canvas_min_height")?,
                window_width: get_f64("layout", "window_width")?,
                window_height: get_f64("layout", "window_height")?,
            },
            radius: RadiusSection {
                chip: get_f64("radius", "chip")?,
                control: get_f64("radius", "control")?,
                card_min: get_f64("radius", "card_min")?,
                card: get_f64("radius", "card")?,
                card_max: get_f64("radius", "card_max")?,
                panel: get_f64("radius", "panel")?,
                window: get_f64("radius", "window")?,
            },
            shadow: ShadowSection {
                card: get_str("shadow", "card")?,
                window: get_str("shadow", "window")?,
                focus: get_str("shadow", "focus")?,
                failure: get_str("shadow", "failure")?,
                taint: get_str("shadow", "taint")?,
            },
            space: SpaceSection {
                px_4: get_f64("space", "px_4")?,
                px_8: get_f64("space", "px_8")?,
                px_12: get_f64("space", "px_12")?,
                px_16: get_f64("space", "px_16")?,
                px_20: get_f64("space", "px_20")?,
                px_24: get_f64("space", "px_24")?,
                px_32: get_f64("space", "px_32")?,
                px_40: get_f64("space", "px_40")?,
            },
            type_family: TypeFamilySection {
                sans: get_str("type", "family_sans")?,
                mono: get_str("type", "family_mono")?,
            },
            type_size: TypeSizeSection {
                size_11: get_u32("type", "size_11")?,
                size_12: get_u32("type", "size_12")?,
                size_13: get_u32("type", "size_13")?,
                size_14: get_u32("type", "size_14")?,
                size_16: get_u32("type", "size_16")?,
                size_20: get_u32("type", "size_20")?,
                size_24: get_u32("type", "size_24")?,
            },
            type_weight: TypeWeightSection {
                regular: get_u32("type", "weight_regular")?,
                medium: get_u32("type", "weight_medium")?,
                semibold: get_u32("type", "weight_semibold")?,
            },
        })
    }
}

pub mod color {
    use super::{
        ColorSection, LayoutSection, ParsedTokens, RadiusSection, ShadowSection, SpaceSection,
        TOKENS_TOML, TypeFamilySection, TypeSizeSection, TypeWeightSection,
    };
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
}

pub mod layout {
    pub const SIDEBAR_WIDTH: f64 = 246.0;
    pub const TOP_BAR_HEIGHT: f64 = 78.0;
    pub const TOP_BAR_WIDTH: f64 = 1674.0;
    pub const CONTENT_WIDTH: f64 = 1674.0;
    pub const CONTENT_HEIGHT: f64 = 1002.0;
    pub const NAV_ITEM_HEIGHT: f64 = 56.0;
    pub const OUTER_MARGIN: f64 = 32.0;
    pub const CONTENT_GUTTER: f64 = 16.0;
    pub const INSPECTOR_WIDTH_MIN: f64 = 360.0;
    pub const INSPECTOR_WIDTH_MAX: f64 = 420.0;
    pub const BOTTOM_TIMELINE_MIN: f64 = 220.0;
    pub const GRAPH_CANVAS_MIN_WIDTH: f64 = 720.0;
    pub const GRAPH_CANVAS_MIN_HEIGHT: f64 = 520.0;
    pub const WINDOW_WIDTH: f64 = 1920.0;
    pub const WINDOW_HEIGHT: f64 = 1080.0;
}

pub mod radius {
    pub const CARD: f64 = 16.0;
}

pub mod shadow {
    pub const CARD: &str = "0 8 24 rgba(16,24,40,0.08)";
}

pub mod space {
    pub const PX_4: f64 = 4.0;
    pub const PX_8: f64 = 8.0;
    pub const PX_12: f64 = 12.0;
    pub const PX_16: f64 = 16.0;
    pub const PX_20: f64 = 20.0;
    pub const PX_24: f64 = 24.0;
    pub const PX_32: f64 = 32.0;
    pub const PX_40: f64 = 40.0;
}
