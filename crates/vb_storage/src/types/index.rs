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
    /// Unknown or custom state marker. The payload byte is offset by
    /// [`MIN_OTHER_BYTE`](Self::MIN_OTHER_BYTE) on the wire so the encoded
    /// byte is always in `[MIN_OTHER_BYTE..=u8::MAX]` and never collides
    /// with the named-variant discriminants.
    Other(u8) = 255,
}

impl IndexStatusState {
    /// State byte reserved for `Other(v)` payloads that would collide with a
    /// named variant. Callers must reject `v < 3` so the encoded key byte is
    /// always invertible through [`from_u8`](Self::from_u8).
    pub const MIN_OTHER_BYTE: u8 = 3;

    /// Maximum payload byte accepted by [`try_new_other`](Self::try_new_other).
    ///
    /// The wire encoding for `Other(v)` is `v + MIN_OTHER_BYTE`, so the
    /// payload `v` must satisfy `v + MIN_OTHER_BYTE <= u8::MAX` to avoid
    /// overflow on the wire. This bounds `v` to `0..=252`.
    pub const MAX_OTHER_BYTE: u8 = u8::MAX - Self::MIN_OTHER_BYTE;

    /// Construct a state from a raw u8 byte.
    ///
    /// The byte is the on-the-wire form: named variants occupy `0..=2`
    /// and `Other(v)` is encoded as `v + MIN_OTHER_BYTE` for `v` in
    /// `0..=MAX_OTHER_BYTE`. This is a strict bijection over `0..=u8::MAX`.
    #[must_use]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Submitted,
            1 => Self::Active,
            2 => Self::Completed,
            b if b >= Self::MIN_OTHER_BYTE => Self::Other(b - Self::MIN_OTHER_BYTE),
            // Defensive fallback: a wire byte below `MIN_OTHER_BYTE` outside
            // the named-variant range is unreachable under the current
            // encoder. Coerce it to the canonical `Other(0)` so the function
            // stays total and never panics on unknown bytes.
            _ => Self::Other(0),
        }
    }

    /// Returns the raw u8 encoding used in storage keys.
    ///
    /// Named variants return their discriminant (0/1/2). `Other(v)` returns
    /// `v + MIN_OTHER_BYTE` so the wire byte is always distinct from the
    /// named-variant discriminants. See bead vb-f1xkn (SC-001).
    #[must_use]
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Submitted => 0,
            Self::Active => 1,
            Self::Completed => 2,
            Self::Other(v) => v.saturating_add(Self::MIN_OTHER_BYTE),
        }
    }

    /// Returns `true` when the byte is a valid on-the-wire `Other` payload
    /// byte (i.e. in `[MIN_OTHER_BYTE..=u8::MAX]`).
    #[must_use]
    pub const fn is_valid_key_byte(byte: u8) -> bool {
        byte >= Self::MIN_OTHER_BYTE
    }

    /// Fallible constructor for the [`Other`](Self::Other) variant that
    /// rejects payload values which would overflow on the wire or collide
    /// with the named-variant discriminants after offset encoding.
    #[must_use]
    pub const fn try_new_other(value: u8) -> Option<Self> {
        if value > Self::MAX_OTHER_BYTE {
            None
        } else {
            Some(Self::Other(value))
        }
    }
}
