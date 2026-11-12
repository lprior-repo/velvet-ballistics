#![forbid(unsafe_code)]

//! Shard-local fencing tokens for live action invocations.
//!
//! A token is minted for every action ticket the shard issues and travels with
//! the ticket, so a completion can be matched against the run's current token.
//! A completion carrying an older token belongs to a superseded invocation and
//! must not apply, even when its ticket fields happen to line up.
//!
//! Zero means "unknown": records written before the token existed decode with
//! zero, and recovery restores tokens from the journal. Fencing therefore never
//! refuses a completion just because the shard restarted.

use std::collections::HashMap;

use vb_core::RunId;

/// Monotonic per-shard fencing counter.
pub(crate) type FencingToken = u64;

/// Tracks the token whose completions the shard currently accepts, per run.
#[derive(Debug, Default)]
pub(crate) struct FencingTokens {
    current: HashMap<RunId, FencingToken>,
    next: FencingToken,
}

#[cfg(test)]
impl FencingTokens {
    pub(crate) fn recovery_snapshot(&self) -> (HashMap<RunId, FencingToken>, FencingToken) {
        (self.current.clone(), self.next)
    }
}

impl FencingTokens {
    /// Creates an empty tracker.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Mints the next token for `run` and records it as the accepted token.
    pub(crate) fn mint(&mut self, run: RunId) -> FencingToken {
        self.next = self.next.saturating_add(1);
        let token = self.next;
        self.current.insert(run, token);
        token
    }

    /// Records a token read back from the journal during recovery.
    ///
    /// Zero is ignored: it carries no ordering information, and overwriting a
    /// live token with it would refuse legitimate completions.
    pub(crate) fn restore(&mut self, run: RunId, token: FencingToken) {
        if token == 0 {
            return;
        }
        self.next = self.next.max(token);
        self.current.insert(run, token);
    }

    /// Whether a completion carrying `token` may apply to `run`.
    ///
    /// A zero token and a run with no recorded token are accepted; the durable
    /// ticket identity still rejects wrong attempts, and refusing unknown
    /// tokens would strand runs across an upgrade or a restart.
    pub(crate) fn accepts(&self, run: RunId, token: FencingToken) -> bool {
        if token == 0 {
            return true;
        }
        self.current
            .get(&run)
            .is_none_or(|current| *current == token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN: RunId = RunId::new(7);

    #[test]
    fn minted_tokens_increase_and_the_latest_is_accepted() {
        let mut tokens = FencingTokens::new();
        let first = tokens.mint(RUN);
        assert!(tokens.accepts(RUN, first));
        let second = tokens.mint(RUN);
        assert!(second > first);
        assert!(tokens.accepts(RUN, second));
        assert!(!tokens.accepts(RUN, first), "superseded token accepted");
    }

    #[test]
    fn zero_token_is_always_accepted() {
        let mut tokens = FencingTokens::new();
        let minted = tokens.mint(RUN);
        assert!(tokens.accepts(RUN, 0));
        assert!(tokens.accepts(RunId::new(99), 0));
        assert!(!tokens.accepts(RUN, minted.saturating_add(1)));
    }

    #[test]
    fn unknown_run_is_accepted_after_recovery() {
        let tokens = FencingTokens::new();
        assert!(tokens.accepts(RunId::new(1234), 42));
    }

    #[test]
    fn restore_keeps_the_journaled_token_authoritative() {
        let mut tokens = FencingTokens::new();
        tokens.restore(RUN, 40);
        assert!(tokens.accepts(RUN, 40));
        assert!(!tokens.accepts(RUN, 39));
        let next = tokens.mint(RUN);
        assert!(next > 40, "mint must not reuse a restored token");
    }

    #[test]
    fn restore_ignores_zero() {
        let mut tokens = FencingTokens::new();
        let minted = tokens.mint(RUN);
        tokens.restore(RUN, 0);
        assert!(tokens.accepts(RUN, minted));
    }
}
