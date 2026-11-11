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

include!("tests/collect_reduce_digest.rs");
include!("tests/choose_lowering.rs");
