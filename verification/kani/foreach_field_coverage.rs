// Verification artifact: foreach_field_coverage.rs
// PO: PO-015 (ForEach field hashing — post-fix correctness)
// Bead: vb-8mdp.7 / vb-xi2f.38
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_foreach_field_coverage
//
// GOD RULE 1: kani::any::<StepPrimitive::ForEach>() generates well-formed ForEach.
// GOD RULE 2: Binds to actual Rust digest_step_primitive implementation.
// GOD RULE 3: No hardcoded dummy data — every harness uses kani::any() for the struct.
//
// The production fix hashes all ForEach fields: variable, input, at_once, body.
// Pre-fix only hashed the primitive name "for_each".

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_05::digest_step_primitive;
use vb_compile::{StepAst, StepPrimitive};

// ─────────────────────────────────────────────────────────────────
// Bounded string helpers (shared with collect_field_coverage.rs)
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BoundedString {
    pub value: [u8; 64],
    pub len: usize,
}

impl kani::Arbitrary for BoundedString {
    fn any() -> Self {
        let mut value = [0u8; 64];
        let len: usize = kani::any();
        let len = len % 65;
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
                    result: vb_compile::ScalarValue::Integer(0),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }
        })
        .collect()
}

impl kani::Arbitrary for StepPrimitive::ForEach {
    fn any() -> Self {
        let variable = kani::any::<BoundedString>().as_str().to_string();
        let input = kani::any::<BoundedString>().as_str().to_string();
        let at_once: Option<u32> = kani::any();
        let body_len: usize = kani::any();
        let body = bounded_body(body_len);

        StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-015: ForEach field coverage
// ─────────────────────────────────────────────────────────────────

fn compute_digest_of(primitive: &StepPrimitive) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher, primitive);
    hasher.finalize().into()
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_foreach_field_coverage_variable() {
    let base = kani::any::<StepPrimitive::ForEach>();
    let mut other = base.clone();
    other.variable = format!("{}_diff", kani::any::<BoundedString>().as_str());

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-015: different ForEach variable must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_foreach_field_coverage_input() {
    let base = kani::any::<StepPrimitive::ForEach>();
    let mut other = base.clone();
    other.input = format!("{}_diff", kani::any::<BoundedString>().as_str());

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-015: different ForEach input must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_foreach_field_coverage_at_once() {
    let base = kani::any::<StepPrimitive::ForEach>();
    let mut other = base.clone();
    other.at_once = match base.at_once {
        None => Some(1),
        Some(a) => Some(a.wrapping_add(1)),
    };

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-015: different ForEach at_once must produce different digest",
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_foreach_field_coverage_body() {
    let base = kani::any::<StepPrimitive::ForEach>();
    let mut other = base.clone();
    let extra_id = kani::any::<BoundedString>();
    other.body.push(StepAst {
        id: extra_id.as_str().to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: vb_compile::ScalarValue::Integer(0),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    });

    let digest_a = compute_digest_of(&base);
    let digest_b = compute_digest_of(&other);

    kani::assert(
        digest_a != digest_b,
        "PO-015: different ForEach body must produce different digest",
    );
}
