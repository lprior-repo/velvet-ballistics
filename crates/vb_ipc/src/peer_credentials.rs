#![forbid(unsafe_code)]
//! SEC-02: Unix-domain socket peer-credentials check at accept time.
//!
//! The IPC server MUST reject frames whose peer identity does not match the
//! configured allow-list before the frame is dispatched. On Unix-like targets
//! the implementation uses `getpeereid`/`SO_PEERCRED` to read the peer's
//! effective user id; the check itself runs in safe Rust with no `unsafe`
//! blocks. On non-Unix targets the check is a no-op (the OS does not provide
//! peer credentials for the IPC transport).

use crate::error::IpcError;

/// Platform-agnostic peer identity recorded by the IPC accept path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerIdentity {
    /// Effective user id. `u32::MAX` when the platform cannot supply one
    /// (Windows, WASI).
    pub euid: u32,
    /// Effective group id. `u32::MAX` when the platform cannot supply one.
    pub egid: u32,
    /// Process id. `u32::MAX` when the platform cannot supply one.
    pub pid: u32,
}

impl PeerIdentity {
    /// Same-user placeholder used when the platform cannot supply a peer
    /// identity. Callers must apply additional transport-level authentication
    /// before relying on this identity.
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            euid: u32::MAX,
            egid: u32::MAX,
            pid: u32::MAX,
        }
    }

    /// Returns true if the peer identity is the platform "unknown" sentinel.
    #[must_use]
    pub const fn is_unknown(self) -> bool {
        self.euid == u32::MAX && self.egid == u32::MAX && self.pid == u32::MAX
    }
}

/// Identity of the local server process. Used to compare against
/// [`PeerIdentity::euid`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ServerIdentity {
    /// Effective user id of the running server.
    pub euid: u32,
}

impl ServerIdentity {
    /// Constructs a server identity from a known uid. Used by the server's
    /// `bind()`/`accept()` glue to compare against the peer's `euid`.
    #[must_use]
    pub const fn new(euid: u32) -> Self {
        Self { euid }
    }
}

/// Outcome of the SEC-02 peer-credentials check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerCheckOutcome {
    /// The peer identity is acceptable for the configured policy.
    Accept,
    /// The peer identity is unacceptable and must be rejected.
    Reject {
        /// Static reason describing why the peer was rejected.
        reason: &'static str,
    },
}

impl PeerCheckOutcome {
    /// Lifts a [`PeerCheckOutcome`] into an [`IpcError::PeerCredentialsFailed`]
    /// for the reject case, returning `Ok(())` for the accept case.
    pub fn into_ipc_error(self) -> Result<(), IpcError> {
        match self {
            Self::Accept => Ok(()),
            Self::Reject { reason } => Err(IpcError::PeerCredentialsFailed(reason)),
        }
    }
}

/// Policy that decides whether a [`PeerIdentity`] is allowed to dispatch IPC
/// commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerPolicy {
    /// Allow any caller. Used only when the IPC server is intentionally
    /// public (e.g., behind an external reverse proxy that already
    /// authenticated the caller).
    AllowAll,
    /// Allow only callers whose effective user id matches the local server's.
    SameUser,
    /// Allow only callers whose effective user id appears in the allow list.
    AllowList {
        /// Allowed effective user ids, in ascending order.
        allowed: &'static [u32],
    },
}

impl PeerPolicy {
    /// Evaluates the policy against a concrete peer identity.
    #[must_use]
    pub fn evaluate(self, peer: PeerIdentity, server: ServerIdentity) -> PeerCheckOutcome {
        match self {
            Self::AllowAll => PeerCheckOutcome::Accept,
            Self::SameUser => {
                if peer.is_unknown() {
                    PeerCheckOutcome::Reject {
                        reason: "peer identity unavailable on this platform",
                    }
                } else if peer.euid == server.euid {
                    PeerCheckOutcome::Accept
                } else {
                    PeerCheckOutcome::Reject {
                        reason: "peer euid does not match server euid",
                    }
                }
            }
            Self::AllowList { allowed } => {
                if peer.is_unknown() {
                    PeerCheckOutcome::Reject {
                        reason: "peer identity unavailable on this platform",
                    }
                } else {
                    let mut idx = 0_usize;
                    let mut accepted = false;
                    while let Some(candidate) = allowed.get(idx) {
                        if *candidate == peer.euid {
                            accepted = true;
                            break;
                        }
                        idx = idx.saturating_add(1);
                    }
                    if accepted {
                        PeerCheckOutcome::Accept
                    } else {
                        PeerCheckOutcome::Reject {
                            reason: "peer euid not in allow list",
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_identity_unknown_is_detected() {
        assert!(PeerIdentity::unknown().is_unknown());
        assert!(
            !PeerIdentity {
                euid: 1000,
                egid: 1000,
                pid: 1
            }
            .is_unknown()
        );
    }

    #[test]
    fn allow_all_accepts_any_peer() {
        let server = ServerIdentity::new(1000);
        let policy = PeerPolicy::AllowAll;
        assert_eq!(
            policy.evaluate(PeerIdentity::unknown(), server),
            PeerCheckOutcome::Accept
        );
        assert_eq!(
            policy.evaluate(
                PeerIdentity {
                    euid: 999,
                    egid: 999,
                    pid: 1
                },
                server
            ),
            PeerCheckOutcome::Accept
        );
    }

    #[test]
    fn same_user_rejects_other_euid() {
        let server = ServerIdentity::new(1000);
        let policy = PeerPolicy::SameUser;
        assert_eq!(
            policy.evaluate(
                PeerIdentity {
                    euid: 1000,
                    egid: 1000,
                    pid: 1
                },
                server
            ),
            PeerCheckOutcome::Accept
        );
        match policy.evaluate(
            PeerIdentity {
                euid: 999,
                egid: 999,
                pid: 1,
            },
            server,
        ) {
            PeerCheckOutcome::Reject { reason } => {
                assert_eq!(reason, "peer euid does not match server euid");
            }
            other => {
                assert!(
                    matches!(other, PeerCheckOutcome::Reject { .. }),
                    "expected rejection, got {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn same_user_rejects_unknown_peer() {
        let server = ServerIdentity::new(1000);
        let policy = PeerPolicy::SameUser;
        match policy.evaluate(PeerIdentity::unknown(), server) {
            PeerCheckOutcome::Reject { reason } => {
                assert_eq!(reason, "peer identity unavailable on this platform");
            }
            other => {
                assert!(
                    matches!(other, PeerCheckOutcome::Reject { .. }),
                    "expected rejection, got {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn allow_list_accepts_listed_euid() {
        let server = ServerIdentity::new(1000);
        let policy = PeerPolicy::AllowList {
            allowed: &[1000, 1234],
        };
        assert_eq!(
            policy.evaluate(
                PeerIdentity {
                    euid: 1234,
                    egid: 1234,
                    pid: 7
                },
                server
            ),
            PeerCheckOutcome::Accept
        );
        match policy.evaluate(
            PeerIdentity {
                euid: 9999,
                egid: 9999,
                pid: 7,
            },
            server,
        ) {
            PeerCheckOutcome::Reject { reason } => {
                assert_eq!(reason, "peer euid not in allow list");
            }
            other => {
                assert!(
                    matches!(other, PeerCheckOutcome::Reject { .. }),
                    "expected rejection, got {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn into_ipc_error_lifts_reject() {
        let outcome: PeerCheckOutcome = PeerCheckOutcome::Reject {
            reason: "peer euid not in allow list",
        };
        let result = outcome.into_ipc_error();
        assert!(
            result.is_err(),
            "reject must lift to error: got {:?}",
            result
        );
        let Err(err) = result else {
            // Unreachable: the assertion above guarantees the Reject lifts to an error.
            return;
        };
        match err {
            IpcError::PeerCredentialsFailed(reason) => {
                assert_eq!(reason, "peer euid not in allow list");
            }
            other => {
                assert!(
                    matches!(other, IpcError::PeerCredentialsFailed(_)),
                    "unexpected error variant: {:?}",
                    other
                );
            }
        }
    }

    #[test]
    fn into_ipc_error_returns_ok_for_accept() {
        let outcome = PeerCheckOutcome::Accept;
        let result = outcome.into_ipc_error();
        assert!(result.is_ok(), "accept must lift to ok: got {:?}", result);
    }
}
