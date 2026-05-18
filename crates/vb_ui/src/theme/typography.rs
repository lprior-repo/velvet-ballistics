#![forbid(unsafe_code)]
use crate::theme::colors;

/// Font weight variants with their numeric CSS values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontWeight {
    Light,
    Regular,
    Medium,
    Bold,
}

impl FontWeight {
    /// Returns the numeric CSS weight value.
    pub const fn value(self) -> u16 {
        match self {
            FontWeight::Light => 300,
            FontWeight::Regular => 400,
            FontWeight::Medium => 500,
            FontWeight::Bold => 700,
        }
    }
}

/// Semantic role for a piece of text in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontRole {
    Display,
    Heading,
    Body,
    Caption,
    Mono,
    Badge,
}

impl FontRole {
    /// Default pixel size for this role.
    pub const fn default_size_px(self) -> f32 {
        match self {
            FontRole::Display => 36.0,
            FontRole::Heading => 24.0,
            FontRole::Body => 14.0,
            FontRole::Caption => 11.0,
            FontRole::Mono => 13.0,
            FontRole::Badge => 10.0,
        }
    }

    /// Default font weight for this role.
    pub const fn default_weight(self) -> FontWeight {
        match self {
            FontRole::Display => FontWeight::Bold,
            FontRole::Heading => FontWeight::Medium,
            FontRole::Body => FontWeight::Regular,
            FontRole::Caption => FontWeight::Regular,
            FontRole::Mono => FontWeight::Regular,
            FontRole::Badge => FontWeight::Bold,
        }
    }
}

/// A complete text style specification for rendering.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub role: FontRole,
    pub size_px: f32,
    pub weight: FontWeight,
    pub color: [f32; 4],
    pub letter_spacing: f32,
}

impl TextStyle {
    /// Create a `TextStyle` populated with defaults for the given role.
    pub fn for_role(role: FontRole) -> Self {
        let (size, weight, color, spacing) = Self::role_defaults(role);
        Self {
            role,
            size_px: size,
            weight,
            color,
            letter_spacing: spacing,
        }
    }

    /// Per-role defaults: (size_px, weight, color, letter_spacing).
    const fn role_defaults(role: FontRole) -> (f32, FontWeight, [f32; 4], f32) {
        match role {
            FontRole::Display => (
                FontRole::Display.default_size_px(),
                FontRole::Display.default_weight(),
                colors::text::PRIMARY,
                1.5,
            ),
            FontRole::Heading => (
                FontRole::Heading.default_size_px(),
                FontRole::Heading.default_weight(),
                colors::text::PRIMARY,
                0.8,
            ),
            FontRole::Body => (
                FontRole::Body.default_size_px(),
                FontRole::Body.default_weight(),
                colors::text::SECONDARY,
                0.3,
            ),
            FontRole::Caption => (
                FontRole::Caption.default_size_px(),
                FontRole::Caption.default_weight(),
                colors::text::DIM,
                0.2,
            ),
            FontRole::Mono => (
                FontRole::Mono.default_size_px(),
                FontRole::Mono.default_weight(),
                colors::neon::CYAN,
                0.0,
            ),
            FontRole::Badge => (
                FontRole::Badge.default_size_px(),
                FontRole::Badge.default_weight(),
                colors::neon::CYAN,
                0.5,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Predefined cyberpunk-palette text styles
// ---------------------------------------------------------------------------

pub const DISPLAY: TextStyle = TextStyle {
    role: FontRole::Display,
    size_px: 36.0,
    weight: FontWeight::Bold,
    color: colors::text::PRIMARY,
    letter_spacing: 1.5,
};

pub const HEADING: TextStyle = TextStyle {
    role: FontRole::Heading,
    size_px: 24.0,
    weight: FontWeight::Medium,
    color: colors::text::PRIMARY,
    letter_spacing: 0.8,
};

pub const BODY: TextStyle = TextStyle {
    role: FontRole::Body,
    size_px: 14.0,
    weight: FontWeight::Regular,
    color: colors::text::SECONDARY,
    letter_spacing: 0.3,
};

pub const CAPTION: TextStyle = TextStyle {
    role: FontRole::Caption,
    size_px: 11.0,
    weight: FontWeight::Regular,
    color: colors::text::DIM,
    letter_spacing: 0.2,
};

pub const MONO: TextStyle = TextStyle {
    role: FontRole::Mono,
    size_px: 13.0,
    weight: FontWeight::Regular,
    color: colors::neon::CYAN,
    letter_spacing: 0.0,
};

pub const BADGE: TextStyle = TextStyle {
    role: FontRole::Badge,
    size_px: 10.0,
    weight: FontWeight::Bold,
    color: colors::neon::CYAN,
    letter_spacing: 0.5,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- FontWeight tests ----------------------------------------------------

    #[test]
    fn font_weight_values_match_css_spec() {
        assert_eq!(FontWeight::Light.value(), 300);
        assert_eq!(FontWeight::Regular.value(), 400);
        assert_eq!(FontWeight::Medium.value(), 500);
        assert_eq!(FontWeight::Bold.value(), 700);
    }

    #[test]
    fn font_weight_ordering() {
        assert!(FontWeight::Light.value() < FontWeight::Regular.value());
        assert!(FontWeight::Regular.value() < FontWeight::Medium.value());
        assert!(FontWeight::Medium.value() < FontWeight::Bold.value());
    }

    // -- FontRole default tests ----------------------------------------------

    #[test]
    fn role_default_sizes_are_positive() {
        let roles = [
            FontRole::Display,
            FontRole::Heading,
            FontRole::Body,
            FontRole::Caption,
            FontRole::Mono,
            FontRole::Badge,
        ];
        for role in roles {
            assert!(
                role.default_size_px() > 0.0,
                "size must be positive for {role:?}"
            );
        }
    }

    #[test]
    fn display_is_largest_role() {
        let max_size = FontRole::Display.default_size_px();
        assert!(max_size > FontRole::Heading.default_size_px());
        assert!(max_size > FontRole::Body.default_size_px());
        assert!(max_size > FontRole::Caption.default_size_px());
        assert!(max_size > FontRole::Mono.default_size_px());
        assert!(max_size > FontRole::Badge.default_size_px());
    }

    #[test]
    fn badge_is_smallest_role() {
        let min_size = FontRole::Badge.default_size_px();
        assert!(min_size < FontRole::Display.default_size_px());
        assert!(min_size < FontRole::Heading.default_size_px());
        assert!(min_size < FontRole::Body.default_size_px());
        assert!(min_size < FontRole::Mono.default_size_px());
        assert!(min_size < FontRole::Caption.default_size_px());
    }

    // -- TextStyle::for_role tests -------------------------------------------

    #[test]
    fn for_role_display_matches_predefined() {
        let style = TextStyle::for_role(FontRole::Display);
        assert_eq!(style.role, DISPLAY.role);
        assert!((style.size_px - DISPLAY.size_px).abs() < f32::EPSILON);
        assert_eq!(style.weight, DISPLAY.weight);
        assert_eq!(style.color, DISPLAY.color);
    }

    #[test]
    fn for_role_heading_matches_predefined() {
        let style = TextStyle::for_role(FontRole::Heading);
        assert_eq!(style.role, HEADING.role);
        assert!((style.size_px - HEADING.size_px).abs() < f32::EPSILON);
        assert_eq!(style.weight, HEADING.weight);
        assert_eq!(style.color, HEADING.color);
    }

    #[test]
    fn for_role_body_matches_predefined() {
        let style = TextStyle::for_role(FontRole::Body);
        assert_eq!(style.role, BODY.role);
        assert_eq!(style.weight, BODY.weight);
        assert_eq!(style.color, BODY.color);
    }

    #[test]
    fn for_role_mono_uses_neon_cyan() {
        let style = TextStyle::for_role(FontRole::Mono);
        assert_eq!(style.color, colors::neon::CYAN);
        assert_eq!(style.weight, FontWeight::Regular);
    }

    #[test]
    fn for_role_badge_is_bold_and_small() {
        let style = TextStyle::for_role(FontRole::Badge);
        assert_eq!(style.weight, FontWeight::Bold);
        assert!(style.size_px < FontRole::Caption.default_size_px());
    }

    // -- Predefined-const consistency tests ----------------------------------

    #[test]
    fn all_predefined_styles_match_for_role_factory() {
        assert_eq!(TextStyle::for_role(FontRole::Display), DISPLAY);
        assert_eq!(TextStyle::for_role(FontRole::Heading), HEADING);
        assert_eq!(TextStyle::for_role(FontRole::Body), BODY);
        assert_eq!(TextStyle::for_role(FontRole::Caption), CAPTION);
        assert_eq!(TextStyle::for_role(FontRole::Mono), MONO);
        assert_eq!(TextStyle::for_role(FontRole::Badge), BADGE);
    }

    #[test]
    fn predefined_styles_use_correct_color_palette() {
        assert_eq!(DISPLAY.color, colors::text::PRIMARY);
        assert_eq!(HEADING.color, colors::text::PRIMARY);
        assert_eq!(BODY.color, colors::text::SECONDARY);
        assert_eq!(CAPTION.color, colors::text::DIM);
        assert_eq!(MONO.color, colors::neon::CYAN);
        assert_eq!(BADGE.color, colors::neon::CYAN);
    }

    #[test]
    fn text_style_copy_is_independent() {
        let original = BODY;
        let mut clone = original;
        clone.size_px = 99.0;
        assert!((BODY.size_px - 14.0).abs() < f32::EPSILON);
        assert!((clone.size_px - 99.0).abs() < f32::EPSILON);
    }

    #[test]
    fn letter_spacing_is_non_negative_for_all_roles() {
        assert!(DISPLAY.letter_spacing >= 0.0);
        assert!(HEADING.letter_spacing >= 0.0);
        assert!(BODY.letter_spacing >= 0.0);
        assert!(CAPTION.letter_spacing >= 0.0);
        assert!(MONO.letter_spacing >= 0.0);
        assert!(BADGE.letter_spacing >= 0.0);
    }
}
