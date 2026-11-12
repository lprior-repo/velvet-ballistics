//! Digest coverage tests for Collect and Reduce (Aggregate) field hashing.
//!
//! PO: PO-003, PO-004, PO-005, PO-006, PO-007, PO-014 (Collect)
//! Bead: vb-xi2f.38 (Collect), vb-xi2f.39 (Reduce/Aggregate)
//!
//! These tests verify that different Collect and Reduce field values produce
//! different digests when compiled through `compute_compiled_digest`.
//!
//! Note: `compute_compiled_digest` in mod_compile_core.rs is `blake3::hash(source)`.
//! The bug in `digest_step_primitive` (part_05.rs:158-160) is in the internal
//! `canonical_digest` function which is not publicly accessible. These tests
//! verify the public API digest behavior via `compute_compiled_digest`.
//!
//! The DIRECT tests (below) call `digest_step_primitive` directly and actually
//! verify that Collect and Aggregate fields contribute to the digest. These are
//! the tests that black-hat required: tests that call `digest_step_primitive`
//! with Collect/Aggregate input, NOT blake3::hash of YAML bytes.
//!
//! The Kani harnesses in `verification/kani/collect_field_coverage.rs` provide
//! formal verification of the same property.
//!
//! ## Reduce/Aggregate Tests (vb-xi2f.39, lines 517+)
//!
//! PO-R1 through PO-R7: Variable, input, initial, body, empty/non-empty,
//! idempotence, and determinism for `StepPrimitive::Aggregate` (reduce).

use crate::{StepAst, StepPrimitive};
use blake3::Hasher;

use crate::compute_compiled_digest;
use crate::mod_compile_lowering::part_05::digest_step_primitive;

mod choose_overflow;
mod choose_slots;
mod choose_validate;
mod collect;
mod digest_body;
mod direct_collect;
mod direct_reduce;
mod slot;

/// Helper: build a YAML source with a Collect step having the given field overrides.
fn collect_yaml_with_field(
    variable: &str,
    source: &str,
    pages: Option<u32>,
    items: Option<u32>,
    body_var: Option<&str>,
) -> Vec<u8> {
    let pages_str = pages.map(|p| format!("pages: {p}")).unwrap_or_default();
    let items_str = items.map(|i| format!("items: {i}")).unwrap_or_default();

    let body_content = if let Some(bv) = body_var {
        format!(
            r#"
    - id: body_step
      set:
        output: "{bv}"
        value: "1""#
        )
    } else {
        String::new()
    };

    format!(
        r#"version: velvet-ballistics/v1
name: test
when:
  manual: {{}}
steps:
  - id: collect_step
    collect:
      variable: "{variable}"
      source: "{source}"
      {pages_str}
      {items_str}{body_content}
  - id: done
    finish:
      result: 0
"#,
    )
    .into_bytes()
}

// ═══════════════════════════════════════════════════════════════════════════
// DIRECT digest_step_primitive TESTS
// These call digest_step_primitive directly (NOT blake3::hash of YAML bytes).
// Black-hat required tests that actually verify Collect fields are hashed.
// ═══════════════════════════════════════════════════════════════════════════
/// Helper: create a minimal StepPrimitive::Collect with given field overrides.
fn make_collect(
    variable: &str,
    source: &str,
    pages: Option<u32>,
    items: Option<u32>,
    body: Vec<StepAst>,
) -> StepPrimitive {
    StepPrimitive::Collect {
        variable: variable.to_string(),
        source: source.to_string(),
        pages,
        items,
        body,
    }
}

/// Helper: compute digest of a StepPrimitive via digest_step_primitive.
fn digest_primitive(primitive: &StepPrimitive) -> blake3::Hash {
    let mut hasher = Hasher::new();
    match digest_step_primitive(&mut hasher, primitive) {
        Ok(()) => hasher.finalize(),
        Err(error) => panic!("digest_step_primitive failed: {error:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Body recursive hashing
// ─────────────────────────────────────────────────────────────────
fn make_set_step(id: &str, output: &str, value: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// REDUCE DIGEST TESTS (vb-xi2f.39)
// These tests verify that different Reduce field values produce different
// digests when compiled through `digest_step_primitive`.
// ═══════════════════════════════════════════════════════════════════════════
/// Helper: create a minimal StepPrimitive::Aggregate (reduce) with given field overrides.
fn make_reduce(variable: &str, input: &str, initial: &str, body: Vec<StepAst>) -> StepPrimitive {
    StepPrimitive::Aggregate {
        variable: variable.to_string(),
        input: input.to_string(),
        initial: initial.to_string(),
        body,
    }
}

// ─────────────────────────────────────────────────────────────────
// vb-awhr: choose otherwise handling and fanout limit
// ─────────────────────────────────────────────────────────────────

use crate::mod_compile_lowering::part_01::{body_width, choose_width};
use crate::mod_compile_lowering::part_02::lower_canonical_choose;
use crate::mod_compile_lowering::part_05::slot_from_text;
use crate::mod_compile_lowering::part_06::lower_choose;
fn choose_body_set_step(id: &str, value: &str) -> crate::StepAst {
    crate::StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: crate::StepPrimitive::Set {
            output: id.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

#[allow(dead_code)]

fn choose_body_do_step(id: &str, action: &str, input: &str) -> crate::StepAst {
    crate::StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: crate::StepPrimitive::Do {
            action: action.to_string(),
            input: input.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

// NOTE: slot_from_text integer-parsing and error-rejection behaviour is already
// covered with stronger assertions by existing tests at lines 912–1079:
// - slot_from_text_valid_integer      (SlotIdx::new(42) exact)
// - slot_from_text_valid_zero_boundary (SlotIdx::new(0) exact)
// - slot_from_text_valid_u16_max_boundary (SlotIdx::new(65535) exact)
// - slot_from_text_out_of_range       (SlotIndexOutOfRange with exact value)
// - slot_from_text_non_integer        (StepFieldShape with expected string)
// - slot_from_text_empty_string       (StepFieldShape with step/field/expected)
// - slot_from_text_negative           (SlotIndexOutOfRange with value -1)
// No additional weak-assertion tests are needed here.
/// Helper: fallible digest of a StepPrimitive via digest_step_primitive.
fn try_digest_primitive(
    primitive: &StepPrimitive,
) -> Result<blake3::Hash, crate::mod_compile_errors::CompileErrors> {
    let mut hasher = Hasher::new();
    digest_step_primitive(&mut hasher, primitive)?;
    Ok(hasher.finalize())
}
