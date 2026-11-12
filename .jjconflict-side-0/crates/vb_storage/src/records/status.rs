/// Typed interpretation of the persisted run-header status byte.
///
/// The byte values are owned by the runtime status model. Storage keeps this
/// type deliberately lossless so old and future records can be read without
/// changing the persisted `u8` layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct RunHeaderStatus(u8);

/// Known run-header status bytes used by the current runtime model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KnownRunHeaderStatus {
    /// Run is pending execution.
    Pending,
    /// Run has been accepted.
    Accepted,
    /// Run is active.
    Active,
    /// Run has finished.
    Finished,
}

/// Typed error for a status byte that is not known by this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownRunHeaderStatus {
    byte: u8,
}

/// Lossless classification of a persisted run-header status byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunHeaderStatusClass {
    /// The byte is known by this build.
    Known(KnownRunHeaderStatus),
    /// The byte is explicit but not known by this build.
    Unknown(u8),
}

impl RunHeaderStatus {
    /// Pending status byte.
    pub const PENDING: Self = Self(0);
    /// Accepted status byte.
    pub const ACCEPTED: Self = Self(1);
    /// Active status byte.
    pub const ACTIVE: Self = Self(2);
    /// Finished status byte.
    pub const FINISHED: Self = Self(3);

    /// Builds a lossless status value from its persisted byte.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Self {
        Self(byte)
    }

    /// Returns the exact byte persisted on disk.
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        self.0
    }

    /// Returns the known status, or a typed unknown-byte error.
    pub const fn known(self) -> Result<KnownRunHeaderStatus, UnknownRunHeaderStatus> {
        match KnownRunHeaderStatus::try_from_byte(self.0) {
            Some(status) => Ok(status),
            None => Err(UnknownRunHeaderStatus { byte: self.0 }),
        }
    }

    /// Classifies the byte without losing unknown values.
    #[must_use]
    pub const fn classify(self) -> RunHeaderStatusClass {
        match KnownRunHeaderStatus::try_from_byte(self.0) {
            Some(status) => RunHeaderStatusClass::Known(status),
            None => RunHeaderStatusClass::Unknown(self.0),
        }
    }
}

impl KnownRunHeaderStatus {
    /// Returns the persisted byte for this known status.
    #[must_use]
    pub const fn as_byte(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Accepted => 1,
            Self::Active => 2,
            Self::Finished => 3,
        }
    }

    const fn try_from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Pending),
            1 => Some(Self::Accepted),
            2 => Some(Self::Active),
            3 => Some(Self::Finished),
            _ => None,
        }
    }
}

impl UnknownRunHeaderStatus {
    /// Returns the unknown persisted byte.
    #[must_use]
    pub const fn byte(self) -> u8 {
        self.byte
    }
}

impl From<KnownRunHeaderStatus> for RunHeaderStatus {
    fn from(status: KnownRunHeaderStatus) -> Self {
        Self(status.as_byte())
    }
}

impl From<RunHeaderStatus> for u8 {
    fn from(status: RunHeaderStatus) -> Self {
        status.as_byte()
    }
}

impl TryFrom<u8> for KnownRunHeaderStatus {
    type Error = UnknownRunHeaderStatus;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        Self::try_from_byte(byte).ok_or(UnknownRunHeaderStatus { byte })
    }
}

#[cfg(test)]
mod run_header_status_tests {
    use super::{
        KnownRunHeaderStatus, RunHeaderStatus, RunHeaderStatusClass, UnknownRunHeaderStatus,
    };

    #[test]
    fn run_header_status_known_bytes_classify_as_known_statuses() {
        let cases = [
            (0, KnownRunHeaderStatus::Pending),
            (1, KnownRunHeaderStatus::Accepted),
            (2, KnownRunHeaderStatus::Active),
            (3, KnownRunHeaderStatus::Finished),
        ];

        for (byte, expected) in cases {
            let status = RunHeaderStatus::from_byte(byte);

            assert_eq!(status.as_byte(), byte);
            assert_eq!(status.known(), Ok(expected));
            assert_eq!(status.classify(), RunHeaderStatusClass::Known(expected));
            assert_eq!(RunHeaderStatus::from(expected).as_byte(), byte);
        }
    }

    #[test]
    fn run_header_status_unknown_byte_returns_typed_error_and_lossless_unknown() {
        let status = RunHeaderStatus::from_byte(255);

        assert_eq!(status.known(), Err(UnknownRunHeaderStatus { byte: 255 }));
        assert_eq!(status.classify(), RunHeaderStatusClass::Unknown(255));
        assert_eq!(status.as_byte(), 255);
    }

    #[test]
    fn run_header_status_known_try_from_rejects_unknown_byte() {
        assert_eq!(
            KnownRunHeaderStatus::try_from(9),
            Err(UnknownRunHeaderStatus { byte: 9 })
        );
    }

    #[test]
    fn unknown_status_byte_is_preserved_by_byte_and_u8_accessors() {
        let unknown = match RunHeaderStatus::from_byte(7).known() {
            Err(unknown) => unknown,
            Ok(_) => panic!("status byte 7 must be unknown"),
        };
        assert_eq!(unknown.byte(), 7);
        assert_eq!(u8::from(RunHeaderStatus::from_byte(7)), 7);
    }

    #[test]
    fn run_header_record_setter_round_trips_typed_status() {
        let mut record = super::super::RunHeaderRecord {
            run: vb_core::RunId::new(7),
            workflow_id: vb_core::WorkflowId::new(3),
            compiled_digest: vb_core::WorkflowDigest::from_bytes(
                [0xAB; crate::constants::DIGEST_BYTES],
            ),
            status: 0,
            accepted_at_ms: 100,
        };
        record.set_run_header_status(RunHeaderStatus::from_byte(2));
        assert_eq!(record.status, 2);
        assert_eq!(
            record.run_header_status().known(),
            Ok(KnownRunHeaderStatus::Active)
        );
    }
}
