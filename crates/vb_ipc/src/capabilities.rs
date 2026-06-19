#![forbid(unsafe_code)]
//! Caller capabilities envelope for the IPC boundary.
//!
//! Every IPC frame must carry a non-zero [`CallerCapabilities`] bitmap in the
//! previously reserved slot of the v1 wire header. Zero is a magic value meaning
//! "missing capability" and is rejected with
//! [`IpcError::PermissionDenied`](crate::IpcError::PermissionDenied).
//!
//! On Unix-like platforms, callers from outside the current user identity are
//! additionally rejected by the peer-credentials check at `accept()` time.

use serde::{Deserialize, Serialize};

/// Bit-position of the `ROOT` capability. The ROOT capability is held by callers
/// that have already passed the OS peer-credentials check; it must be present in
/// every valid frame so that the absence-of-capabilities check is unambiguous.
pub const ROOT_CAPABILITY_BIT: u16 = 0x0001;
/// Bit-position of the `OPERATOR` capability. Required for commands that mutate
/// runtime state (`Shutdown`, `CancelRun`).
pub const OPERATOR_CAPABILITY_BIT: u16 = 0x0002;
/// Bit-position of the `OBSERVER` capability. Required for read-only commands
/// (`Health`, `InspectRun`, `ListEvents`).
pub const OBSERVER_CAPABILITY_BIT: u16 = 0x0004;
/// Bit-position of the `SUBMITTER` capability. Required for commands that
/// start a new run (`SubmitRun`, `SubmitRunInline`).
pub const SUBMITTER_CAPABILITY_BIT: u16 = 0x0008;
/// Bit-position of the `ACTION_HANDLER` capability. Required for commands that
/// resolve external action/ask tickets (`AnswerAsk`, `CompleteAction`,
/// `FailAction`).
pub const ACTION_HANDLER_CAPABILITY_BIT: u16 = 0x0010;

/// Caller capability bitmap carried in every IPC frame.
///
/// This is the SEC-01 envelope. A zero value is reserved to mean "no capability
/// provided" and is rejected at decode time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct CallerCapabilities(u16);

impl CallerCapabilities {
    /// Empty (no-capabilities) bit set. Frames carrying this value are rejected
    /// at decode time.
    pub const EMPTY: Self = Self(0);
    /// The minimum set of capabilities required to land a frame: caller has
    /// passed the OS peer-credentials check.
    pub const ROOT: Self = Self(ROOT_CAPABILITY_BIT);
    /// Capabilities granted to operator clients (shutdown, cancel).
    pub const OPERATOR: Self = Self(ROOT_CAPABILITY_BIT | OPERATOR_CAPABILITY_BIT);
    /// Capabilities granted to read-only clients.
    pub const OBSERVER: Self = Self(ROOT_CAPABILITY_BIT | OBSERVER_CAPABILITY_BIT);
    /// Capabilities granted to clients that submit runs.
    pub const SUBMITTER: Self = Self(ROOT_CAPABILITY_BIT | SUBMITTER_CAPABILITY_BIT);
    /// Capabilities granted to clients that answer or fail action tickets.
    pub const ACTION_HANDLER: Self = Self(ROOT_CAPABILITY_BIT | ACTION_HANDLER_CAPABILITY_BIT);

    /// Wraps a raw wire value. Does not validate non-zero — use [`Self::from_wire`]
    /// or [`Self::require_nonzero`] for that.
    #[must_use]
    pub const fn from_raw(bits: u16) -> Self {
        Self(bits)
    }

    /// Reads a wire value, returning [`None`] for the no-capability sentinel.
    #[must_use]
    pub const fn from_wire(bits: u16) -> Option<Self> {
        if bits == 0 { None } else { Some(Self(bits)) }
    }

    /// Returns the raw wire value.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Returns true if the bit set is non-empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns true if `self` is a superset of `other`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns true if `self` carries the `ROOT` capability bit.
    #[must_use]
    pub const fn has_root(self) -> bool {
        (self.0 & ROOT_CAPABILITY_BIT) != 0
    }

    /// Bitwise OR.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Bitwise AND.
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl Default for CallerCapabilities {
    /// Default is [`CallerCapabilities::ROOT`] so that the existing four-argument
    /// `IpcFrameHeader::new` constructor produces valid envelopes.
    fn default() -> Self {
        Self::ROOT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_wire_zero_is_none() {
        assert_eq!(CallerCapabilities::from_wire(0), None);
    }

    #[test]
    fn from_wire_root_is_some() {
        assert_eq!(
            CallerCapabilities::from_wire(ROOT_CAPABILITY_BIT),
            Some(CallerCapabilities::ROOT)
        );
    }

    #[test]
    fn empty_is_empty() {
        assert!(CallerCapabilities::EMPTY.is_empty());
        assert!(!CallerCapabilities::ROOT.is_empty());
    }

    #[test]
    fn operator_contains_root() {
        assert!(CallerCapabilities::OPERATOR.contains(CallerCapabilities::ROOT));
    }

    #[test]
    fn union_or_combines_bits() {
        let combined = CallerCapabilities::SUBMITTER.union(CallerCapabilities::OBSERVER);
        assert!(combined.contains(CallerCapabilities::SUBMITTER));
        assert!(combined.contains(CallerCapabilities::OBSERVER));
    }

    #[test]
    fn intersection_and_extracts_common_bits() {
        let common = CallerCapabilities::OPERATOR.intersection(CallerCapabilities::ROOT);
        assert_eq!(common, CallerCapabilities::ROOT);
    }

    #[test]
    fn default_is_root() {
        assert_eq!(CallerCapabilities::default(), CallerCapabilities::ROOT);
    }

    /// Edge case (tier-a-6-011 deep validation): the named capability bits
    /// must all round-trip through `from_wire` as `Some(...)`. `from_wire(0)`
    /// is the only sentinel; every named bit is non-zero and must survive.
    #[test]
    fn from_wire_round_trips_all_named_capability_bits() {
        const BITS: [u16; 5] = [
            ROOT_CAPABILITY_BIT,
            OPERATOR_CAPABILITY_BIT,
            OBSERVER_CAPABILITY_BIT,
            SUBMITTER_CAPABILITY_BIT,
            ACTION_HANDLER_CAPABILITY_BIT,
        ];
        for &bit in &BITS {
            let caps = CallerCapabilities::from_wire(bit);
            assert!(caps.is_some(), "from_wire({bit}) must be Some");
            let unwrapped = caps.unwrap_or(CallerCapabilities::EMPTY);
            assert_eq!(unwrapped.bits(), bit);
        }
    }

    /// Edge case: the bitmap is a raw `u16` with no reserved/undefined mask.
    /// Bits outside the named capability set must round-trip verbatim
    /// rather than being silently masked off.
    #[test]
    fn raw_bits_preserve_undefined_high_bit_positions() {
        const RAW: u16 = 0xFFFF;
        let caps = CallerCapabilities::from_wire(RAW);
        assert_eq!(caps.map(CallerCapabilities::bits), Some(RAW));
        let unwrapped = caps.unwrap_or(CallerCapabilities::EMPTY);
        assert!(unwrapped.has_root());
        // Bits beyond the named capability set survive the round-trip.
        assert_eq!(unwrapped.bits(), RAW);
    }

    /// Edge case: `contains` is a strict subset check. A superset contains
    /// its subset; a subset does not contain its superset; equal sets
    /// contain each other; the empty set contains only itself.
    #[test]
    fn contains_strict_subset_semantics() {
        let operator = CallerCapabilities::OPERATOR;
        let root = CallerCapabilities::ROOT;
        assert!(operator.contains(root));
        assert!(!root.contains(operator));
        assert!(root.contains(root));
        assert!(CallerCapabilities::EMPTY.contains(CallerCapabilities::EMPTY));
        assert!(!CallerCapabilities::EMPTY.contains(CallerCapabilities::ROOT));
    }

    /// Edge case: union and intersection form a lattice on the bit-set.
    /// `a | b` is the join; `a & b` is the meet; intersection of a set with
    /// itself equals itself; intersection of disjoint sets is empty.
    #[test]
    fn union_intersection_lattice_holds() {
        let a = CallerCapabilities::OPERATOR;
        let b = CallerCapabilities::OBSERVER;
        let joined = a.union(b);
        assert!(joined.contains(a));
        assert!(joined.contains(b));
        let met = a.intersection(b);
        // OPERATOR and OBSERVER share only ROOT.
        assert_eq!(met, CallerCapabilities::ROOT);
        let disjoint = CallerCapabilities::ROOT
            .intersection(CallerCapabilities::SUBMITTER);
        assert_eq!(disjoint, CallerCapabilities::ROOT);
        let none = CallerCapabilities::ROOT.intersection(CallerCapabilities::EMPTY);
        assert_eq!(none, CallerCapabilities::EMPTY);
    }
}
