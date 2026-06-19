/// Distinguishes the failure modes of input-bin → slot mapping at the
/// runtime boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputMappingFailureKind {
    /// Input bin was empty; this is treated as "no inputs to map" and is
    /// not itself an error, but is preserved as a distinct outcome for
    /// diagnostics and parity.
    EmptyInputBin,
    /// Postcard decoder rejected the input bin.
    MalformedPostcard,
    /// A decoded value could not be coerced into the expected slot type.
    TypeMismatch {
        /// Expected slot type tag (compact, runtime-internal).
        expected: u16,
    },
}

impl InputMappingFailureKind {
    /// Stable lower-snake phrase used by legacy diagnostic renderers.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyInputBin => "empty_input_bin",
            Self::MalformedPostcard => "malformed_postcard",
            Self::TypeMismatch { .. } => "type_mismatch",
        }
    }

    /// Returns the legacy human-readable diagnostic phrase.
    #[must_use]
    pub const fn legacy_diagnostic_phrase(self) -> &'static str {
        match self {
            Self::EmptyInputBin => "INPUT_MAPPING_FAILED: input-bin is empty",
            Self::MalformedPostcard => "INPUT_MAPPING_FAILED: input-bin decode failed",
            Self::TypeMismatch { .. } => "INPUT_MAPPING_FAILED: input slot type mismatch",
        }
    }

    /// Compact `u32` code suitable for log or metric emission. The
    /// high 16 bits hold the diagnostic code (`0x201F`); the low 16
    /// bits distinguish the kind.
    #[must_use]
    pub const fn code(self) -> u32 {
        match self {
            Self::EmptyInputBin => 0x201F_0001,
            Self::MalformedPostcard => 0x201F_0002,
            Self::TypeMismatch { .. } => 0x201F_0003,
        }
    }
}

impl std::fmt::Display for InputMappingFailureKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.legacy_diagnostic_phrase())
    }
}
