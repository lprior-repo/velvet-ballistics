// Verification artifact: collect_field_coverage.rs
// PO: PO-002 (Collect field coverage — post-fix correctness)
// PO: PO-020 (GOD RULE — no hardcoded harness data)
// Bead: vb-8mdp.7 / vb-xi2f.38
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_collect_field_coverage
//
// GOD RULE 1: kani::any::<StepPrimitive::Collect>() generates all valid field combinations.
// GOD RULE 2: Binds to actual Rust digest_step_primitive implementation — never copies it.
// GOD RULE 3: No hardcoded dummy data — every harness uses kani::any() for the struct.
//
// The production fix (part_05.rs:263-299) now hashes all Collect fields:
//   variable, source, pages, items, body.
// This harness proves that two Collect instances differing in any field
// produce different digest contributions (post-fix), and that the digest
// function never panics on arbitrary Collect input.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_05::digest_step_primitive;
use vb_yaml::ast::{StepAst, StepPrimitive};

// ─────────────────────────────────────────────────────────────────
// Bounded string helpers for kani::Arbitrary
// ─────────────────────────────────────────────────────────────────

/// Bounded string up to 64 bytes — avoids unbounded blowup in Kani.
#[derive(Debug, Clone)]
pub struct BoundedString {
    pub value: [u8; 64],
    pub len: usize,
}

impl kani::Arbitrary for BoundedString {
    fn any() -> Self {
        let mut value = [0u8; 64];
        let len: usize = kani::any();
        let len = len % 65; // 0..64
        for i in 0..len {
            value[i] = kani::any();
        }
        BoundedString { value, len }
    }
}

impl BoundedString {
    pub fn as_str(&self) -> &str {
        let valid = &self.value[..self.len.min(64)];
        std::str::from_utf8(valid).unwrap_or("")
    }
}

/// Bounded body: 0..8 child steps, each with a bounded id.
/// Every child step uses Finish(Integer(0)) so we don't recurse infinitely.
fn bounded_body(len: usize) -> Vec<StepAst> {
    let len = len.min(8);
    (0..len)
        .map(|_| {
            let id = kani::any::<BoundedString>();
            StepAst {
                id: id.as_str().to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Finish {
                    result: vb_yaml::ast::ScalarValue::Integer(0),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }
        })
        .collect()
}

/// Arbitrary for Collect — GOD RULE compliant: every field from kani::any().
impl kani::Arbitrary for StepPrimitive::Collect {
    fn any() -> Self {
        let variable = kani::any::<BoundedString>().as_str().to_string();
        let source = kani::any::<BoundedString>().as_str().to_string();
        let pages: Option<u32> = kani::any();
        let items: Option<u32> = kani::any();
        let body_len: usize = kani::any();
        let body = bounded_body(body_len);

        StepPrimitive::Collect {
            variable,
            source,
            pages,
            items,
            body,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-002: Collect field coverage — all fields contribute to digest.
//
// Strategy: generate two arbitrary Collect instances and assert
// that if they differ structurally, their digests differ.
//
// POST-FIX: digest_step_primitive hashes variable/source/pages/items/body.
// PRE-FIX (bug): only hashed primitive name "collect".
// ─────────────────────────────────────────────────────────────────

/// Generate two Collect instances where exactly one field differs,
/// and verify the resulting digest contributions differ (post-fix).
///
/// This harness uses kani::any() structurally — no hardcoded field values.
fn compute_digest_of(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    // We call the actual production function — GOD RULE 2.
    let _ = digest_step_primitive(&mut hasher, primitive);
    hasher.finalize().into()
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_collect_field_coverage_variable() {
    let base = kani::any::<StepPrimitive::Collect>();
    // Clone and change the variable field.
    let mut other = base.clone();
    other.variable = format!("{}_diff", kani::any::<BoundedString>().as_str());

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    // Different variable → different digest (post-fix).
    kani::assert(
        digest_a != digest_b,
        "PO-002: different variable field must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_collect_field_coverage_source() {
    let base = kani::any::<StepPrimitive::Collect>();
    let mut other = base.clone();
    other.source = format!("{}_diff", kani::any::<BoundedString>().as_str());

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-002: different source field must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_collect_field_coverage_pages() {
    let base = kani::any::<StepPrimitive::Collect>();
    let mut other = base.clone();
    // Flip pages: if None→Some(1), if Some(p)→None
    other.pages = match base.pages {
        None => Some(1),
        Some(p) => Some(p.wrapping_add(1)),
    };

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-002: different pages field must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_collect_field_coverage_items() {
    let base = kani::any::<StepPrimitive::Collect>();
    let mut other = base.clone();
    other.items = match base.items {
        None => Some(1),
        Some(i) => Some(i.wrapping_add(1)),
    };

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-002: different items field must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_collect_field_coverage_body() {
    let base = kani::any::<StepPrimitive::Collect>();
    let mut other = base.clone();
    // Append a step to body — always produces different body.
    let extra_id = kani::any::<BoundedString>();
    other.body.push(StepAst {
        id: extra_id.as_str().to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: vb_yaml::ast::ScalarValue::Integer(0),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    });

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    // Note: empty body vs non-empty body should differ.
    // If base.body was at max bound (8), adding makes 9 — still different.
    kani::assert(
        digest_a != digest_b,
        "PO-002: different body must produce different digest",
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-020: GOD RULE — all harnesses use kani::any().
// This meta-harness proves the Arbitrary impl is non-trivial.
// ─────────────────────────────────────────────────────────────────

#[kani::proof]
#[kani::unwind(4)]
fn kani_god_rule_collect_uses_any() {
    // Prove that kani::Arbitrary produces structurally valid Collect
    // and that no field is constrained to a single hardcoded value.
    let c1 = kani::any::<StepPrimitive::Collect>();
    let c2 = kani::any::<StepPrimitive::Collect>();

    // At least one field might differ between two any() calls.
    // kani::cover! is an existence proof — if Kani can find a
    // trace where fields differ, the Arbitrary impl is non-trivial.
    let fields_differ = c1.variable != c2.variable
        || c1.source != c2.source
        || c1.pages != c2.pages
        || c1.items != c2.items
        || c1.body.len() != c2.body.len();

    kani::cover!(
        fields_differ,
        "PO-020 GOD RULE: kani::any() generates non-identical Collect instances"
    );
}
