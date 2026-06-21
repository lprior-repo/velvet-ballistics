#![forbid(unsafe_code)]
//! Validated IPC command flag set.
//!
//! Contract §2.1: every IPC frame carries a 16-bit flag word. Bits 8..=15
//! (mask `0xFF00`) are reserved for the IPC envelope and must always be
//! zero. The low byte is command-specific and constrained per command.
//!
//! This module provides [`CommandFlags::validate`] which checks a raw
//! `u16` flag word against the reserved-global mask and the
//! command-specific valid mask and returns either a validated
//! [`CommandFlags`] or an [`IpcError`](crate::IpcError).

use crate::commands::IpcCommand;
use crate::error::IpcError;

/// Global reserved flag mask: bits 8..=15 of the flag word.
///
/// Contract §2.1 INV-RESERVED: ∀ C, raw & 0xFF00 == 0.
pub const RESERVED_GLOBAL_MASK: u16 = 0xFF00;

/// Validated, command-specific flag set carried in the IPC frame header.
///
/// The inner `u16` is guaranteed by [`Self::validate`] to satisfy:
///
/// - `raw & RESERVED_GLOBAL_MASK == 0`
/// - `raw & !valid_mask(command) == 0`
///
/// Construction outside [`Self::validate`] is restricted to `pub(crate)`
/// to prevent bypassing validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandFlags(u16);

impl CommandFlags {
    /// Wraps an already-validated raw flag value.
    ///
    /// Marked `pub(crate)` to prevent external callers from bypassing
    /// [`Self::validate`]. Use [`Self::validate`] for external
    /// construction.
    #[must_use]
    pub(crate) const fn from_validated_raw(raw: u16) -> Self {
        Self(raw)
    }

    /// Returns the validated raw flag value as a `u16`.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.0
    }

    /// Returns the per-command valid flag mask (the upper-bound mask).
    ///
    /// Production values may use subset masks but the upper bound
    /// documented here is the contract maximum. Contract §2.2.
    #[must_use]
    pub const fn valid_mask(command: IpcCommand) -> u16 {
        match command {
            IpcCommand::SubmitRun => 0x00FF,
            IpcCommand::SubmitRunInline => 0x00FF,
            IpcCommand::CancelRun => 0x0000,
            IpcCommand::InspectRun => 0x0003,
            IpcCommand::ListEvents => 0x00FF,
            IpcCommand::AnswerAsk => 0x0000,
            IpcCommand::CompleteAction => 0x0000,
            IpcCommand::FailAction => 0x0000,
            IpcCommand::DrainTrace => 0x0007,
            IpcCommand::Health => 0x0000,
            IpcCommand::Shutdown => 0x0000,
            IpcCommand::UnknownCommand(_) => 0x0000,
        }
    }

    /// Validates a raw flag word against the command's contract.
    ///
    /// Validation order (contract §4):
    /// 1. Reserved bits check: if `raw & RESERVED_GLOBAL_MASK != 0`,
    ///    return [`IpcError::ReservedBitsSet`].
    /// 2. Command-specific mask: if `raw & !valid_mask(command) != 0`,
    ///    return [`IpcError::InvalidCommandFlags`].
    /// 3. Otherwise return `Ok(CommandFlags(raw))`.
    pub const fn validate(command: IpcCommand, raw: u16) -> Result<Self, IpcError> {
        if (raw & RESERVED_GLOBAL_MASK) != 0 {
            return Err(IpcError::ReservedBitsSet {
                command,
                actual: raw,
                reserved_mask: RESERVED_GLOBAL_MASK,
            });
        }
        let mask = Self::valid_mask(command);
        if (raw & !mask) != 0 {
            return Err(IpcError::InvalidCommandFlags {
                command,
                flags: raw,
            });
        }
        Ok(Self(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_flags_validate_for_every_command() {
        for cmd in [
            IpcCommand::SubmitRun,
            IpcCommand::SubmitRunInline,
            IpcCommand::CancelRun,
            IpcCommand::InspectRun,
            IpcCommand::ListEvents,
            IpcCommand::AnswerAsk,
            IpcCommand::CompleteAction,
            IpcCommand::FailAction,
            IpcCommand::DrainTrace,
            IpcCommand::Health,
            IpcCommand::Shutdown,
        ] {
            let result = CommandFlags::validate(cmd, 0x0000);
            assert!(result.is_ok(), "zero flags must validate for {cmd:?}");
            let flags = result.unwrap_or(CommandFlags::from_validated_raw(0));
            assert_eq!(flags.as_u16(), 0);
        }
    }

    #[test]
    fn reserved_bits_beat_command_specific_mask() {
        // 0xFF01 has both reserved bits AND out-of-mask bits for
        // Health (mask=0). Reserved check must fire first.
        let result = CommandFlags::validate(IpcCommand::Health, 0xFF01);
        assert_eq!(
            result,
            Err(IpcError::ReservedBitsSet {
                command: IpcCommand::Health,
                actual: 0xFF01,
                reserved_mask: RESERVED_GLOBAL_MASK,
            })
        );
    }

    #[test]
    fn invalid_low_byte_for_zero_mask_command_rejects() {
        let result = CommandFlags::validate(IpcCommand::Health, 0x0001);
        assert_eq!(
            result,
            Err(IpcError::InvalidCommandFlags {
                command: IpcCommand::Health,
                flags: 0x0001,
            })
        );
    }

    #[test]
    fn valid_mask_per_command_upper_bound_is_disjoint_from_reserved() {
        for cmd in [
            IpcCommand::SubmitRun,
            IpcCommand::SubmitRunInline,
            IpcCommand::CancelRun,
            IpcCommand::InspectRun,
            IpcCommand::ListEvents,
            IpcCommand::AnswerAsk,
            IpcCommand::CompleteAction,
            IpcCommand::FailAction,
            IpcCommand::DrainTrace,
            IpcCommand::Health,
            IpcCommand::Shutdown,
        ] {
            let mask = CommandFlags::valid_mask(cmd);
            assert_eq!(
                mask & RESERVED_GLOBAL_MASK,
                0,
                "valid_mask({cmd:?})={mask:#06x} must be disjoint from RESERVED_GLOBAL_MASK",
            );
        }
    }

    #[test]
    fn as_u16_returns_raw_value() {
        let flags = CommandFlags::validate(IpcCommand::SubmitRunInline, 0x00FF)
            .expect("SubmitRunInline accepts 0x00FF");
        assert_eq!(flags.as_u16(), 0x00FF);
        let zero = CommandFlags::validate(IpcCommand::Health, 0).expect("Health accepts 0");
        assert_eq!(zero.as_u16(), 0);
    }
}
