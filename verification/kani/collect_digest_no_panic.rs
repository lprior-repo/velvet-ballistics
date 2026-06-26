// Verification artifact: collect_digest_no_panic.rs
// PO: PO-013 (CC-DIGEST-006: No panic on Collect digest)
// Bead: vb-8mdp.7
// Verifier: Kani
// Command: cargo kani --package vb_compile --harness kani_collect_digest_no_panic
//
// GOD RULE 1: Calls digest_step_primitive with arbitrary Collect input
// GOD RULE 2: Binds to actual Rust digest_step_primitive implementation (part_05.rs:194)
// GOD RULE 3: No hardcoded dummy data
//
// This harness proves that digest_step_primitive does not panic for
// any valid StepPrimitive containing a Collect variant with bounded fields.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_compile::mod_compile_lowering::part_05::digest_step_primitive;
use vb_compile::{StepAst, StepPrimitive};

// ─────────────────────────────────────────────────────────────────
// Bounded string helpers (consistent with collect_field_coverage.rs)
// ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct BoundedString {
    value: [u8; 64],
    len: usize,
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
    fn as_str(&self) -> &str {
        let valid = &self.value[..self.len.min(64)];
        std::str::from_utf8(valid).unwrap_or("")
    }
}

/// Bounded body: 0..8 child steps, each with a bounded id.
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

/// Construct an arbitrary Collect step using kani::any() for every field.
fn any_collect_step() -> StepPrimitive {
    let variable = kani::any::<BoundedString>().as_str().to_string();
    let source = kani::any::<BoundedString>().as_str().to_string();
    let pages: Option<u32> = kani::any();
    let items: Option<u32> = kani::any();
    let body_len: usize = kani::any();
    let body = bounded_body(body_len % 9);

    StepPrimitive::Collect {
        variable,
        source,
        pages,
        items,
        body,
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-013: No panic on Collect digest
// ─────────────────────────────────────────────────────────────────

/// Prove that digest_step_primitive does not panic when called with
/// an arbitrary Collect variant. The function returns Option<()> —
/// we assert only that no panic occurs, regardless of return value.
#[kani::proof]
#[kani::unwind(16)]
fn kani_collect_digest_no_panic() {
    let primitive = any_collect_step();
    let mut hasher = blake3::Hasher::new();

    // digest_step_primitive returns Option<()> — we verify no panic,
    // not success. The function may return Err for invalid inputs,
    // but must never panic.
    let result = digest_step_primitive(&mut hasher, &primitive);

    // Reached this point without panicking — PO-013 satisfied.
    kani::assert(
        true,
        "PO-013: digest_step_primitive reached without panic for arbitrary Collect",
    );

    // Ensure result is consumed (avoids unused-result warning)
    let _ = result;
}

/// Second harness: verify no panic specifically when pages and items
/// are extreme values (None, Some(0), Some(u32::MAX)).
#[kani::proof]
#[kani::unwind(16)]
fn kani_collect_digest_no_panic_extreme() {
    let variable = kani::any::<BoundedString>().as_str().to_string();
    let source = kani::any::<BoundedString>().as_str().to_string();
    // Cycle through edge cases for pages and items
    let pages_selector: u8 = kani::any();
    let pages = match pages_selector % 4 {
        0 => None,
        1 => Some(0),
        2 => Some(1),
        _ => Some(u32::MAX),
    };
    let items_selector: u8 = kani::any();
    let items = match items_selector % 4 {
        0 => None,
        1 => Some(0),
        2 => Some(1),
        _ => Some(u32::MAX),
    };
    let body_len: usize = kani::any();
    let body = bounded_body(body_len % 9);

    let primitive = StepPrimitive::Collect {
        variable,
        source,
        pages,
        items,
        body,
    };

    let mut hasher = blake3::Hasher::new();
    let result = digest_step_primitive(&mut hasher, &primitive);

    kani::assert(
        true,
        "PO-013: digest_step_primitive reached without panic for Collect with extreme pages/items",
    );
    let _ = result;
}
