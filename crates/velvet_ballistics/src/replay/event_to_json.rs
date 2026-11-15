//! Convert journal events to the shared replay and events JSON projection.
#![forbid(unsafe_code)]

use serde_json::{Value, json};
use vb_storage::JournalEvent;

mod action;
mod authority;
mod lifecycle;
mod step;

/// Retains common journal identity alongside each event's complete payload.
pub(crate) fn event_to_json(event: &JournalEvent) -> Value {
    let fields = step::fields(event)
        .or_else(|| action::fields(event))
        .or_else(|| lifecycle::fields(event))
        .or_else(|| authority::fields(event));
    let mut value = match fields {
        Some(value) => value,
        None => {
            json!({"type":"Unknown", "note":"future non-exhaustive variant — add explicit match arm"})
        }
    };
    append_identity(&mut value, event);
    value
}

fn append_identity(value: &mut Value, event: &JournalEvent) {
    if let Value::Object(fields) = value {
        fields.extend([
            ("run".into(), json!(event.run_id().get())),
            ("seq".into(), json!(event.seq().get())),
        ]);
        if let Some(attempt) = event.attempt() {
            fields.extend([("attempt".into(), json!(attempt))]);
        }
    }
}

#[cfg(test)]
mod continuation_tests;
#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod tests;
