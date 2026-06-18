#![forbid(unsafe_code)]
//! Reference-parsing helpers for workflow reference validation.
//!
//! Provides string-level extraction functions used by the validation engine
//! to deconstruct `$`-prefixed reference paths into roots, step IDs, and
//! field names.

use vb_core::ids::StepIdx;

/// Sentinel [`SymbolId`] for the canonical
/// "output" field of a step result.
///
/// The validator does not have access to the workflow's symbol table,
/// so we record the missing field as a fixed symbol id. Downstream
/// consumers that know the workflow's symbol table can re-resolve
/// the symbol id back to the field name "output" via the registry.
pub const OUTPUT_FIELD_SYMBOL: vb_core::ids::SymbolId = vb_core::ids::SymbolId::new(0);

/// Parses a namespace-prefixed step reference of the form
/// `$step.<step_id>.<field>` or `$steps.<step_id>.<field>`.
///
/// Returns `Some((step_id, field))` if the reference is a valid step reference,
/// or `None` if the reference is not a step reference.
pub fn parse_step_reference(reference: &str) -> Option<(&str, &str)> {
    let body = reference.strip_prefix('$')?;
    let (root, tail) = body.split_once('.')?;
    if !matches!(root, "step" | "steps") {
        return None;
    }
    let (step_id, field) = tail.split_once('.')?;
    Some((step_id, field))
}

/// Extracts the first name component from a dotted tail.
///
/// For `"user.profile"` returns `"user"`.
/// For `"output"` returns `"output"` (no dot present).
pub(super) fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}

/// Returns `true` when the requested step field is the canonical
/// "output" slot.
///
/// The input `field_tail` is either the text after a direct step root
/// (e.g. `Some("output.value")` for `$build.output.value`) or after the
/// step id in a `$steps` reference (e.g. `Some("output")` for
/// `$steps.build.output`). `None` means the reference named a step without
/// a field.
pub(super) fn step_field_is_output(field_tail: Option<&str>) -> bool {
    match field_tail {
        Some(tail) => reference_name(tail) == "output",
        None => false,
    }
}

/// Converts a [`usize`] workflow step index into the bounded
/// [`StepIdx`] newtype used by [`crate::ValidationError`] variants.
///
/// [`StepIdx`] is a `u16` newtype. Workflows that exceed `u16::MAX`
/// steps cannot be represented; in that case we saturate to
/// `StepIdx::MAX` so the validator can still emit a typed diagnostic
/// instead of panicking on the conversion. The error variant the
/// caller is constructing is `ResultReferenceMissing`, so the saturating
/// behavior keeps the validation pipeline total and avoids surfacing
/// a misleading `UnknownReference` when the real failure is the
/// missing output slot.
pub(super) fn step_index_to_step_idx(step_idx: usize) -> StepIdx {
    match u16::try_from(step_idx) {
        Ok(value) => StepIdx::new(value),
        Err(_) => StepIdx::new(u16::MAX),
    }
}
