use super::VALID_COMMANDS;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ParseError {
    MissingArgument(&'static str),
    UnknownEmitTarget(String),
    UnknownDurability(String),
    UnknownProfile(String),
    UnknownCommand(String),
    InvalidStatusArgument(String),
    UnknownActionCommand(String),
    UnknownActionRegistry(String),
    MissingActionRegistryValue,
    UnknownActionListFlag(String),
    UnexpectedActionListArgument(String),
    UnknownActionInspectFlag(String),
    UnexpectedActionInspectArgument(String),
    InvalidActionId(String),
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
            Self::InvalidStatusArgument(reason) => {
                write!(formatter, "invalid status argument: {reason}")
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
            Self::UnknownActionInspectFlag(flag) => {
                write!(formatter, "unknown action inspect flag: {flag}")
            }
            Self::UnexpectedActionInspectArgument(argument) => {
                write!(formatter, "unexpected action inspect argument: {argument}")
            }
            Self::InvalidActionId(action_id) => {
                write!(formatter, "invalid action id: {action_id}")
            }
            Self::NoCommand => write!(formatter, "no command provided"),
            Self::InvalidStep(step) => write!(formatter, "invalid step: {step}"),
            Self::ReasonTooLong => {
                write!(formatter, "reason exceeds maximum length of 256 characters")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_display_missing_argument() {
        assert_eq!(
            ParseError::MissingArgument("workflow").to_string(),
            "missing argument: workflow"
        );
    }

    #[test]
    fn parse_error_display_unknown_emit_target() {
        assert_eq!(
            ParseError::UnknownEmitTarget("json".into()).to_string(),
            "unknown emit target: json (expected: ir, yaml, postcard)"
        );
    }

    #[test]
    fn parse_error_display_unknown_durability() {
        assert_eq!(
            ParseError::UnknownDurability("fast".into()).to_string(),
            "unknown durability mode: fast (expected: strict, journaled, none)"
        );
    }

    #[test]
    fn parse_error_display_unknown_profile() {
        assert_eq!(
            ParseError::UnknownProfile("deep".into()).to_string(),
            "unknown verify profile: deep (expected: quick, standard, full)"
        );
    }

    #[test]
    fn parse_error_display_unknown_command() {
        let display = ParseError::UnknownCommand("foo".into()).to_string();
        assert!(display.contains("unknown command: foo"));
    }

    #[test]
    fn parse_error_display_no_command() {
        assert_eq!(ParseError::NoCommand.to_string(), "no command provided");
    }

    #[test]
    fn parse_error_display_invalid_step() {
        assert_eq!(
            ParseError::InvalidStep("abc".into()).to_string(),
            "invalid step: abc"
        );
    }

    #[test]
    fn parse_error_display_reason_too_long() {
        assert_eq!(
            ParseError::ReasonTooLong.to_string(),
            "reason exceeds maximum length of 256 characters"
        );
    }

    #[test]
    fn parse_error_display_unknown_action_command() {
        assert_eq!(
            ParseError::UnknownActionCommand("delete".into()).to_string(),
            "unknown action command: delete (expected: list, inspect)"
        );
    }

    #[test]
    fn parse_error_all_variants_are_exhaustive() {
        let errors = [
            ParseError::MissingArgument("test"),
            ParseError::UnknownEmitTarget("test".into()),
            ParseError::UnknownDurability("test".into()),
            ParseError::UnknownProfile("test".into()),
            ParseError::UnknownCommand("test".into()),
            ParseError::InvalidStatusArgument("test".into()),
            ParseError::UnknownActionCommand("test".into()),
            ParseError::UnknownActionRegistry("test".into()),
            ParseError::MissingActionRegistryValue,
            ParseError::UnknownActionListFlag("test".into()),
            ParseError::UnexpectedActionListArgument("test".into()),
            ParseError::UnknownActionInspectFlag("test".into()),
            ParseError::UnexpectedActionInspectArgument("test".into()),
            ParseError::InvalidActionId("test".into()),
            ParseError::NoCommand,
            ParseError::InvalidStep("test".into()),
            ParseError::ReasonTooLong,
        ];
        for err in &errors {
            let s = err.to_string();
            assert!(!s.is_empty(), "empty display for {:?}", err);
        }
    }
}
