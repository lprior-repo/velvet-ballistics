//! Error types for argument parsing.
#![forbid(unsafe_code)]

use super::VALID_COMMANDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ParseError {
    MissingArgument(&'static str),
    UnknownEmitTarget(String),
    UnknownDurability(String),
    UnknownProfile(String),
    UnknownCommand(String),
    UnknownServerMode(String),
    UnknownEventStatus(String),
    InvalidAgentContextArgument(String),
    InvalidTraceArgument(String),
    InvalidStatusArgument(String),
    InvalidSystemStatusArgument(String),
    UnknownActionCommand(String),
    UnknownActionRegistry(String),
    MissingActionRegistryValue,
    UnknownActionListFlag(String),
    UnexpectedActionListArgument(String),
    InvalidActionListArgument(String),
    UnknownActionInspectFlag(String),
    UnexpectedActionInspectArgument(String),
    InvalidActionInspectArgument(String),
    InvalidActionId(String),
    InvalidActionName(String),
    UnknownFlag { command: &'static str, flag: String },
    InvalidArgument(String),
    NoCommand,
    InvalidStep(String),
    ReasonTooLong,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingArgument(name) => write!(formatter, "missing argument: {name}"),
            Self::UnknownEmitTarget(target) => {
                write!(
                    formatter,
                    "unknown emit target: {target} (expected: ir, yaml, postcard)"
                )
            }
            Self::UnknownDurability(mode) => {
                write!(
                    formatter,
                    "unknown durability mode: {mode} (expected: strict, journaled, none)"
                )
            }
            Self::UnknownProfile(profile) => {
                write!(
                    formatter,
                    "unknown verify profile: {profile} (expected: quick, standard, full)"
                )
            }
            Self::UnknownCommand(cmd) => {
                write!(
                    formatter,
                    "unknown command: {cmd} (expected one of: {VALID_COMMANDS})"
                )
            }
            Self::UnknownServerMode(mode) => {
                write!(
                    formatter,
                    "unknown server mode: {mode} (expected: none; strict and journaled require a backend probe that is not implemented)"
                )
            }
            Self::UnknownEventStatus(status) => {
                write!(formatter, "unknown event status: {status}")
            }
            Self::InvalidAgentContextArgument(reason) => {
                write!(formatter, "invalid agent-context argument: {reason}")
            }
            Self::InvalidTraceArgument(reason) => {
                write!(formatter, "invalid trace argument: {reason}")
            }
            Self::InvalidStatusArgument(reason) => {
                write!(formatter, "invalid status argument: {reason}")
            }
            Self::InvalidSystemStatusArgument(reason) => {
                write!(formatter, "invalid system status argument: {reason}")
            }
            Self::UnknownActionCommand(cmd) => {
                write!(
                    formatter,
                    "unknown action command: {cmd} (expected: list, inspect)"
                )
            }
            Self::UnknownActionRegistry(registry) => {
                write!(
                    formatter,
                    "unknown action registry: {registry} (expected: registered, empty, uninitialized)"
                )
            }
            Self::MissingActionRegistryValue => write!(
                formatter,
                "missing action-args value for --registry (expected: registered, empty, uninitialized)"
            ),
            Self::UnknownActionListFlag(flag) => {
                write!(formatter, "unknown action list flag: {flag}")
            }
            Self::UnexpectedActionListArgument(argument) => {
                write!(formatter, "unexpected action list argument: {argument}")
            }
            Self::InvalidActionListArgument(reason) => {
                write!(formatter, "invalid action list argument: {reason}")
            }
            Self::UnknownActionInspectFlag(flag) => {
                write!(formatter, "unknown action inspect flag: {flag}")
            }
            Self::UnexpectedActionInspectArgument(argument) => {
                write!(formatter, "unexpected action inspect argument: {argument}")
            }
            Self::InvalidActionInspectArgument(reason) => {
                write!(formatter, "invalid action inspect argument: {reason}")
            }
            Self::InvalidActionId(action_id) => {
                write!(formatter, "invalid action id: {action_id}")
            }
            Self::InvalidActionName(name) => {
                write!(formatter, "invalid action name: {name}")
            }
            Self::UnknownFlag { command, flag } => {
                write!(formatter, "unknown flag for {command}: {flag}")
            }
            Self::InvalidArgument(reason) => {
                write!(formatter, "invalid argument: {reason}")
            }
            Self::NoCommand => write!(formatter, "no command provided"),
            Self::InvalidStep(step) => write!(formatter, "invalid step: {step}"),
            Self::ReasonTooLong => {
                write!(formatter, "reason exceeds maximum length of 256 characters")
            }
        }
    }
}
