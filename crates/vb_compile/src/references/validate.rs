//! Compile-specific reference validation routing.
//!
//! Validates a reference from the compiler AST, handling compile-specific
//! references (`$slot.*`, `$attempt.*`, accessor paths) locally and delegating
//! everything else to `vb_validate::references::validate_single_reference_with_context`.
//!
//! `in_repeat_body` lifts the `$attempt.*` scope guard for references that
//! appear inside a `Repeat` body. The flag is propagated from
//! `collect_references_from_repeat_body` and is `false` for top-level
//! references (where `$attempt.*` is rejected with `InvalidVariableScope`).
//!
//! Master §65 idempotency-key determinism gate lives here: the function
//! [`validate_idempotency_key_determinism`] rejects random / time / wall-clock
//! references wherever a YAML idempotency-key surface is encountered. The
//! webhook trigger `unique` field is the only current YAML surface that
//! materialises an idempotency-key string; future per-action idempotency-key
//! fields will route through the same function.

use super::errors::map_validation_error;
use crate::errors::NonDeterministicKind;
use crate::{CompileError, SourceMark};
use vb_validate::references::{RefTables, validate_single_reference_with_context};

pub(super) fn validate_compile_reference(
    reference: &str,
    tables: &RefTables,
    step_index: Option<usize>,
    in_repeat_body: bool,
) -> Result<(), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let Some((root, tail)) = body.split_once('.') else {
        // Bare reference -- delegate to shared validation. Inside a repeat
        // body the bare `$attempt` is still illegal (it is only meaningful
        // with the `.number` accessor), so the scope guard does not lift for
        // bare references.
        return validate_single_reference_with_context(reference, tables, step_index, false, false)
            .map_err(|e| map_validation_error(reference, &e));
    };
    if root == "attempt" {
        if in_repeat_body {
            return Ok(());
        }
        return Err(reject_attempt_scope(reference));
    }
    // Compile-specific: slot references are not in the standalone validator
    if matches!(root, "slot" | "slots") {
        return validate_slot_reference(reference, root, tail);
    }
    // Compile-specific: reject accessor paths after declared names
    // (e.g., $vars.data.field is unsupported because the compiler
    // does not support accessor traversal on vars/inputs/secrets)
    if let Some(error) = check_accessor_path(reference, root, tail, tables) {
        return Err(error);
    }
    validate_single_reference_with_context(reference, tables, step_index, false, false)
        .map_err(|e| map_validation_error(reference, &e))
}

/// Rejects a `$attempt.*` reference observed outside a `Repeat` body.
///
/// Scope guard: `$attempt.*` is only legal inside a `Repeat` body step.
/// Architectural invariant: the cold AST (master spec §45) drops
/// `StepKindAst::Repeat` body expressions at construction. Any
/// `$attempt.*` reference that reaches this validator is therefore
/// by definition outside a `Repeat` body — there is no per-step
/// "in a Repeat body" flag on `RefTables` (only declared name
/// sets), and the cold-AST `Repeat` variant carries no body to
/// inspect. The blanket reject is correct under the cold-AST
/// invariant. When canonical lowering adds body retention (master
/// §45 follow-up), this guard will need a `repeat_step_indices`
/// set threaded through `RefTables` to support the legal
/// use case (see `references_scope_guard_tests.rs` for the
/// architectural note).
fn reject_attempt_scope(reference: &str) -> CompileError {
    CompileError::InvalidVariableScope {
        reference: Box::from(reference),
        context: "outside repeat body",
        allowed: Box::from(["repeat_attempt.body", "repeat_check"].as_slice()),
        mark: SourceMark::unavailable(),
    }
}

/// Validates a `$slot.*` reference (compile-specific).
fn validate_slot_reference(reference: &str, root: &str, tail: &str) -> Result<(), CompileError> {
    let (slot, path) = match tail.split_once('.') {
        Some((slot, path)) => (slot, Some(path)),
        None => (tail, None),
    };
    if slot.parse::<u16>().is_err() {
        return Err(CompileError::UnknownReferenceName {
            kind: "slot",
            reference: Box::from(reference),
            name: Box::from(slot),
        });
    }
    if let Some(path) = path {
        if numeric_accessor_path(path) {
            return Ok(());
        }
        let accessor_root = format!("{root}.{slot}");
        return Err(CompileError::UnsupportedAccessorReference {
            reference: Box::from(reference),
            root: Box::from(accessor_root),
            path: Box::from(path),
        });
    }
    Ok(())
}

fn numeric_accessor_path(path: &str) -> bool {
    let mut saw_segment = false;
    for segment in path.split('.') {
        // Reject empty segments (e.g., from "$slot.1..0") and non-numeric segments.
        if segment.is_empty() {
            return false;
        }
        if segment.parse::<u32>().is_err() {
            return false;
        }
        saw_segment = true;
    }
    saw_segment
}

/// Checks for unsupported accessor paths after declared names.
///
/// For example, `$vars.data.field` has an accessor path `field` after the
/// declared name `data`, which the compiler does not support.
fn check_accessor_path(
    reference: &str,
    root: &str,
    tail: &str,
    tables: &RefTables,
) -> Option<CompileError> {
    // Only check accessor paths for name-rooted references
    #[allow(clippy::question_mark)]
    let Some((name, path)) = tail.split_once('.') else {
        return None;
    };
    // Check if the root+name is declared; if so, the trailing path is unsupported
    let is_declared = match root {
        "input" | "inputs" => tables.contains_input(name),
        "var" | "vars" => tables.contains_var(name),
        "secrets" => tables.contains_secret(name),
        _ => return None,
    };
    if is_declared {
        let accessor_root = format!("{root}.{name}");
        return Some(CompileError::UnsupportedAccessorReference {
            reference: Box::from(reference),
            root: Box::from(accessor_root),
            path: Box::from(path),
        });
    }
    None
}

// ── Master §65 idempotency-key determinism gate ─────────────────────────

/// Validates that a list of references feeding an idempotency key are
/// deterministic (master plan §65).
///
/// Idempotency keys must be reproducible across retries and replay so that
/// the same logical action request always produces the same
/// `ActionTicket.idempotency_key`. References rooted at `random`,
/// `time`, `now`, `runtime`, `wall_clock`, `wallclock`, or `clock` denote
/// non-reproducible state and are rejected with
/// [`CompileError::IdempotencyKeyNotDeterministic`].
///
/// This is the compile-time companion to the runtime check in
/// `vb_core::action::validate::validate_idempotency_key_ingredients`. The
/// runtime check inspects the actual `Taint` of slots feeding the key and
/// cannot see whether a slot was sourced from a non-deterministic reference
/// (deterministic taint is permitted for derived values). The compile-time
/// check sits in front of the runtime and rejects the source reference before
/// any slot is materialised.
///
/// # Signature
///
/// The function takes a borrowed slice of reference strings (typically
/// `$root.path...`) extracted from the YAML idempotency-key surface. The
/// first non-deterministic reference is reported and short-circuits the
/// check; the remaining references are not inspected.
pub(super) fn validate_idempotency_key_determinism(
    references: &[&str],
) -> Result<(), CompileError> {
    let mut index = 0;
    while index < references.len() {
        let Some(reference) = references.get(index) else {
            break;
        };
        reject_non_deterministic_reference(reference)?;
        index = match index.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    Ok(())
}

/// Scans a YAML idempotency-key surface string for `$...` references and
/// rejects any reference rooted at a non-deterministic source.
///
/// Used to extract references from free-form key strings (the webhook
/// trigger `unique` field today, and any future per-action
/// `idempotency.key` field). The scanner is conservative: a `$` not
/// followed by an ASCII identifier is left alone, so plain text like
/// "USD" or "v2" is not misread as a reference.
pub(super) fn scan_idempotency_key_references(text: &str, out: &mut Vec<Box<str>>) {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let Some(dollar_offset) = find_dollar(bytes, cursor) else {
            break;
        };
        let Some(reference_start) = dollar_offset.checked_add(1) else {
            break;
        };
        let Some(first_byte) = bytes.get(reference_start).copied() else {
            break;
        };
        if !is_reference_start(first_byte) {
            cursor = reference_start;
            continue;
        }
        let (reference_end, after_end) = scan_reference(bytes, reference_start);
        // `scan_reference` returns a `[reference_start..reference_end]` slice
        // bounded by `bytes.len()` (see `checked_add` usage there). Use
        // `bytes.get(...)` so the bound is checked at runtime; clippy
        // requires this because `scan_reference` is not inlined here and
        // cannot prove the bound across the call.
        let reference_text = match bytes
            .get(reference_start..reference_end)
            .and_then(|slice| std::str::from_utf8(slice).ok())
        {
            Some(text) => text,
            None => {
                cursor = after_end;
                continue;
            }
        };
        let mut reference = String::with_capacity(reference_text.len().saturating_add(1));
        reference.push('$');
        reference.push_str(reference_text);
        out.push(reference.into_boxed_str());
        cursor = after_end;
    }
}

fn reject_non_deterministic_reference(reference: &str) -> Result<(), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Ok(());
    };
    let root = match body.split_once('.') {
        Some((root, _)) => root,
        None => body,
    };
    match root {
        "random" => Err(CompileError::IdempotencyKeyNotDeterministic {
            reference: Box::from(reference),
            kind: NonDeterministicKind::Random,
        }),
        "time" | "now" => Err(CompileError::IdempotencyKeyNotDeterministic {
            reference: Box::from(reference),
            kind: NonDeterministicKind::Time,
        }),
        "runtime" => Err(CompileError::IdempotencyKeyNotDeterministic {
            reference: Box::from(reference),
            kind: NonDeterministicKind::Time,
        }),
        "wall_clock" | "wallclock" | "clock" => Err(CompileError::IdempotencyKeyNotDeterministic {
            reference: Box::from(reference),
            kind: NonDeterministicKind::WallClock,
        }),
        _ => Ok(()),
    }
}

fn find_dollar(bytes: &[u8], from: usize) -> Option<usize> {
    let mut cursor = from;
    while cursor < bytes.len() {
        if bytes.get(cursor).copied()? == b'$' {
            return Some(cursor);
        }
        cursor = cursor.checked_add(1)?;
    }
    None
}

fn is_reference_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_reference_continuation(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

fn scan_reference(bytes: &[u8], start: usize) -> (usize, usize) {
    let mut end = start;
    while let Some(byte) = bytes.get(end).copied() {
        if !is_reference_continuation(byte) {
            break;
        }
        end = match end.checked_add(1) {
            Some(next) => next,
            None => break,
        };
    }
    let mut path_end = end;
    while let Some(byte) = bytes.get(path_end).copied() {
        if byte == b'.' {
            let segment_start = match path_end.checked_add(1) {
                Some(next) => next,
                None => break,
            };
            match bytes.get(segment_start).copied() {
                Some(next_byte) if is_reference_continuation(next_byte) => {
                    path_end = segment_start;
                    while let Some(byte) = bytes.get(path_end).copied() {
                        if !is_reference_continuation(byte) {
                            break;
                        }
                        path_end = match path_end.checked_add(1) {
                            Some(next) => next,
                            None => break,
                        };
                    }
                }
                _ => break,
            }
        } else {
            break;
        }
    }
    (path_end, path_end)
}
