//! Source-of-truth #4: `run_from_env` dispatcher arm count.
//!
//! The `run_from_env` function is defined at
//! `crates/vb_cli/src/dispatcher.rs:49-159` and contains a `match` over the
//! `Command` enum with one arm per variant. The `Command` enum has 30
//! variants (see `source_command_enum`), so the dispatcher has 30 arms.
//! This module encodes 30 as a `pub const` and provides a
//! `dispatch_arm_count` function.

#![forbid(unsafe_code)]

/// Number of `match` arms in `run_from_env`'s dispatcher.
///
/// Source of truth: `crates/vb_cli/src/dispatcher.rs:49-159` (one arm per
/// `Command` variant; `Command` has 30 variants per
/// `crates/vb_cli/src/args/types.rs:69-218`).
pub const DISPATCH_ARM_COUNT: usize = 30;

/// Returns the constant 30.
#[must_use]
pub const fn dispatch_arm_count() -> usize {
    DISPATCH_ARM_COUNT
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::matrix::source_command_enum;

    #[test]
    fn dispatch_arm_count_matches_command_variant_count() {
        assert_eq!(dispatch_arm_count(), 30);
        assert_eq!(
            dispatch_arm_count(),
            source_command_enum::variant_count(),
            "run_from_env dispatch arm count must equal Command variant count"
        );
    }
}
