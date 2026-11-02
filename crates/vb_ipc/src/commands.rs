#![forbid(unsafe_code)]
//! IPC command identifiers.

use serde::{Deserialize, Serialize};

use crate::error::IpcError;

/// Binary IPC command identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
#[non_exhaustive]
pub enum IpcCommand {
    /// Submit a run using a previously compiled workflow artifact.
    SubmitRun = 1,
    /// Submit a run with inline validated runtime inputs.
    SubmitRunInline = 2,
    /// Cancel an active or queued run.
    CancelRun = 3,
    /// Inspect run state.
    InspectRun = 4,
    /// List persisted events for a run.
    ListEvents = 5,
    /// Answer a suspended ask.
    AnswerAsk = 6,
    /// Complete an external action ticket.
    CompleteAction = 7,
    /// Fail an external action ticket.
    FailAction = 8,
    /// Drain bounded trace records.
    DrainTrace = 9,
    /// Probe runtime health.
    Health = 10,
    /// Request graceful shutdown.
    Shutdown = 11,
    /// Unknown command identifier (reserved for wire compatibility).
    UnknownCommand(u16) = 12,
}

/// Validated command flags for a single IPC command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandFlags {
    command: IpcCommand,
    raw: u16,
}

impl CommandFlags {
    /// Bits no command may set.
    #[must_use]
    pub const fn reserved_global_mask() -> u16 {
        0xFF00
    }

    /// Bits a command may set.
    #[must_use]
    pub const fn valid_mask(command: IpcCommand) -> u16 {
        match command {
            IpcCommand::SubmitRun | IpcCommand::SubmitRunInline | IpcCommand::ListEvents => 0x00FF,
            IpcCommand::InspectRun => 0x0003,
            IpcCommand::DrainTrace => 0x0007,
            IpcCommand::CancelRun
            | IpcCommand::AnswerAsk
            | IpcCommand::CompleteAction
            | IpcCommand::FailAction
            | IpcCommand::Health
            | IpcCommand::Shutdown
            | IpcCommand::UnknownCommand(_) => 0x0000,
        }
    }

    /// Validates raw flags for a command, checking reserved bits before
    /// the command-specific mask.
    pub fn validate(command: IpcCommand, raw: u16) -> Result<Self, IpcError> {
        if (raw & Self::reserved_global_mask()) != 0 {
            return Err(IpcError::ReservedBitsSet {
                command,
                actual: raw,
                reserved_mask: Self::reserved_global_mask(),
            });
        }
        if (raw & !Self::valid_mask(command)) != 0 {
            return Err(IpcError::InvalidCommandFlags {
                command,
                flags: raw,
            });
        }
        Ok(Self { command, raw })
    }

    /// Returns the command these flags were validated for.
    #[must_use]
    pub const fn command(self) -> IpcCommand {
        self.command
    }

    /// Returns the validated raw flags value.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        self.raw
    }
}

impl IpcCommand {
    /// Parses a wire command identifier.
    pub fn from_u16(value: u16) -> Result<Self, IpcError> {
        match value {
            1 => Ok(Self::SubmitRun),
            2 => Ok(Self::SubmitRunInline),
            3 => Ok(Self::CancelRun),
            4 => Ok(Self::InspectRun),
            5 => Ok(Self::ListEvents),
            6 => Ok(Self::AnswerAsk),
            7 => Ok(Self::CompleteAction),
            8 => Ok(Self::FailAction),
            9 => Ok(Self::DrainTrace),
            10 => Ok(Self::Health),
            11 => Ok(Self::Shutdown),
            other => Ok(Self::UnknownCommand(other)),
        }
    }

    /// Returns the wire command identifier.
    #[must_use]
    pub const fn as_u16(self) -> u16 {
        match self {
            Self::SubmitRun => 1,
            Self::SubmitRunInline => 2,
            Self::CancelRun => 3,
            Self::InspectRun => 4,
            Self::ListEvents => 5,
            Self::AnswerAsk => 6,
            Self::CompleteAction => 7,
            Self::FailAction => 8,
            Self::DrainTrace => 9,
            Self::Health => 10,
            Self::Shutdown => 11,
            Self::UnknownCommand(id) => id,
        }
    }
}
