#![forbid(unsafe_code)]
//! Status index state markers.

/// State marker byte for status index entries.
///
/// These values are encoded directly into the index key to allow
/// range queries filtered by state without decoding the full key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum IndexStatusState {
    /// Submitted — run has been accepted but not yet started.
    Submitted = 0,
    /// Active — run is currently executing.
    Active = 1,
    /// Completed — run finished successfully.
    Completed = 2,
    /// Unknown or custom state marker.
    Other(u8) = 255,
}

impl IndexStatusState {
    /// State byte reserved for `Other(v)` payloads that would collide with a
    /// named variant. Callers must reject `v < 3` so the encoded key byte is
    /// always invertible through [`from_u8`](Self::from_u8).
    pub const MIN_OTHER_BYTE: u8 = 3;

    /// Construct a state from a raw u8 byte.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Submitted,
            1 => Self::Active,
            2 => Self::Completed,
            _ => Self::Other(value),
        }
    }

    /// Returns the raw u8 encoding used in storage keys.
    #[must_use]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Submitted => 0,
            Self::Active => 1,
            Self::Completed => 2,
            Self::Other(v) => v,
        }
    }

    /// Returns `true` if the byte would round-trip through
    /// [`from_u8`](Self::from_u8) and [`to_u8`](Self::to_u8) without
    /// collapsing into a named variant.
    #[must_use]
    pub const fn is_valid_key_byte(byte: u8) -> bool {
        byte >= Self::MIN_OTHER_BYTE
    }

    /// Fallible constructor for the [`Other`](Self::Other) variant that
    /// rejects byte values which would collide with the named variant
    /// discriminants in the storage key.
    #[must_use]
    pub const fn try_new_other(value: u8) -> Option<Self> {
        if value < Self::MIN_OTHER_BYTE {
            None
        } else {
            Some(Self::Other(value))
        }
    }
}
