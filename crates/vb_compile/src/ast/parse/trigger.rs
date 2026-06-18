#![forbid(unsafe_code)]
//! Trigger parsing for workflow documents.
//!
//! Validates that exactly one trigger kind is present and dispatches
//! to the appropriate trigger-specific parser.

use crate::ast::types::TriggerAst;
use crate::CompileError;
use crate::SourceMark;
use saphyr::Yaml;

use super::field::optional_str;

/// Parse a trigger mapping node into a `TriggerAst`.
///
/// Validates that exactly one trigger kind key is present.
pub(crate) fn parse_trigger(
    mapping: &saphyr::Mapping<'_>,
    marks: &crate::ast::marks::AstMarks,
) -> Result<TriggerAst, CompileError> {
    if mapping.len() != 1 {
        return Err(CompileError::InvalidTriggerCount {
            count: mapping.len(),
        });
    }
    let Some((key, value)) = mapping.iter().next() else {
        return Err(CompileError::InvalidTriggerCount { count: 0 });
    };
    let kind = key.as_str().ok_or_else(crate::non_string_key_error)?;
    let mark = marks.trigger(kind);
    match kind {
        "manual" => Ok(TriggerAst::Manual { mark }),
        "webhook" => parse_webhook_trigger(value, mark),
        "schedule" => parse_schedule_trigger(value, mark),
        "event" => parse_event_trigger(value, mark),
        other => Err(CompileError::UnknownTriggerKind {
            trigger: other.into(),
        }),
    }
}

/// Parse a webhook trigger body.
fn parse_webhook_trigger(
    value: &Yaml<'_>,
    mark: Option<SourceMark>,
) -> Result<TriggerAst, CompileError> {
    Ok(TriggerAst::Webhook {
        path: optional_str(value, "path").map(|s| s.into()),
        method: optional_str(value, "method").map(|s| s.into()),
        unique: optional_str(value, "unique").map(Box::<str>::from),
        mark,
    })
}

/// Parse a schedule trigger body.
fn parse_schedule_trigger(
    value: &Yaml<'_>,
    mark: Option<SourceMark>,
) -> Result<TriggerAst, CompileError> {
    Ok(TriggerAst::Schedule {
        cron: trigger_str(value, "schedule", "cron")?.into(),
        timezone: optional_str(value, "timezone").map(Box::<str>::from),
        mark,
    })
}

/// Parse an event trigger body.
fn parse_event_trigger(
    value: &Yaml<'_>,
    mark: Option<SourceMark>,
) -> Result<TriggerAst, CompileError> {
    Ok(TriggerAst::Event {
        name: trigger_str(value, "event", "type")?.into(),
        mark,
    })
}

/// Extract a required string field from a trigger body.
fn trigger_str<'a>(
    value: &'a Yaml<'a>,
    trigger: &'static str,
    field: &'static str,
) -> Result<&'a str, CompileError> {
    value
        .as_mapping_get(field)
        .ok_or(CompileError::MissingTriggerField { trigger, field })?
        .as_str()
        .ok_or(CompileError::InvalidTriggerField {
            trigger,
            field,
            expected: "a string",
        })
}
