#![forbid(unsafe_code)]

use crate::error::Error;
use super::parse::parse_hex;

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
