#![forbid(unsafe_code)]
//! Trigger parsing logic.

use crate::{YamlError, YamlResult};

use super::parse::{lookup, mapping, reject_unknown_fields};
use super::types::*;

/// Parse the trigger declaration from a workflow node.
pub(super) fn parse_trigger(node: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    if let Some(when_val) = lookup(node, "when") {
        return parse_when_trigger(when_val);
    }
    Err(YamlError::MissingField {
        span: None,
        field: "when",
    })
}

fn parse_when_trigger(when_val: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    let map = mapping(when_val, "when")?;
    if map.len() != 1 {
        return Err(YamlError::FieldShape {
            span: None,
            field: "when",
            expected: "exactly one trigger",
        });
    }
    let Some((key, body)) = map.iter().next() else {
        return Err(YamlError::FieldShape {
            span: None,
            field: "when",
            expected: "exactly one trigger",
        });
    };
    let Some(kind) = key.as_str() else {
        return Err(YamlError::FieldShape {
            span: None,
            field: "when key",
            expected: "string",
        });
    };
    match kind {
        "manual" => empty_body(body, "when.manual").map(|()| TriggerAst::Manual),
        "webhook" => empty_body(body, "when.webhook").map(|()| TriggerAst::Webhook),
        "schedule" => parse_schedule(body),
        "event" => parse_event(body),
        "ipc" => Err(YamlError::UnsupportedTrigger {
            span: None,
            trigger: "ipc",
        }),
        "http" => Err(YamlError::UnsupportedTrigger {
            span: None,
            trigger: "http",
        }),
        other => Err(YamlError::UnknownField {
            span: None,
            field: other.into(),
        }),
    }
}

fn empty_body(body: &saphyr::Yaml<'_>, field: &'static str) -> YamlResult<()> {
    let map = mapping(body, field)?;
    if map.is_empty() {
        Ok(())
    } else {
        Err(YamlError::FieldShape {
            span: None,
            field,
            expected: "empty mapping",
        })
    }
}

fn parse_schedule(body: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    reject_unknown_fields(body, &["cron"])?;
    let Some(cron) = lookup(body, "cron").and_then(saphyr::Yaml::as_str) else {
        return Err(YamlError::MissingField {
            span: None,
            field: "when.schedule.cron",
        });
    };
    if cron.is_empty() {
        return Err(YamlError::FieldShape {
            span: None,
            field: "when.schedule.cron",
            expected: "non-empty string",
        });
    }
    Ok(TriggerAst::Schedule {
        cron: cron.to_string(),
    })
}

fn parse_event(body: &saphyr::Yaml<'_>) -> YamlResult<TriggerAst> {
    reject_unknown_fields(body, &["type"])?;
    let Some(event_name) = lookup(body, "type").and_then(saphyr::Yaml::as_str) else {
        return Err(YamlError::MissingField {
<<<<<<< HEAD
            field: "when.event.type",
=======
            span: None,
            field: "when.event.name",
>>>>>>> landing/vb-xi2f.9
        });
    };
    if event_name.is_empty() {
        return Err(YamlError::FieldShape {
            span: None,
            field: "when.event.name",
            expected: "non-empty string",
        });
    }
    Ok(TriggerAst::Event {
        event_type: event_name.to_string(),
    })
}
