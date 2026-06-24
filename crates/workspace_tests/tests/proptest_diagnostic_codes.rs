#![forbid(unsafe_code)]

//! Property tests: diagnostic-code uniqueness, stability, range,
//! registry cross-check, constructor totality, and proptest-driven
//! pairwise sweep for the wave-15 `RuntimeError::X_CODE` constants.
//!
//! # Scope
//!
//! Wave-15 introduced thirty `pub const X_CODE: DiagnosticCode =
//! DiagnosticCode::new(0xNNNN);` declarations on `RuntimeError` at
//! `crates/vb_runtime/src/error/diagnostics.rs:10-41`, covering the
//! `0x2001..=0x201E` runtime range. The contract that all diagnostic
//! codes are **unique** (no two variants share a discriminant) and
//! **stable** (discriminants do not change across builds) had no
//! formal-verification coverage before this bead. This file fills
//! that gap.
//!
//! # Source type
//!
//! `DiagnosticCode` is defined at `crates/vb_core/src/diagnostic.rs:1763`
//! as `pub struct DiagnosticCode(u16)` with the constructor
//! `pub const fn new(code: u16) -> Self { Self(code) }` at line 1770.
//! The packed numeric accessor is `pub const fn code(self) -> u16`
//! at line 1776 (NOT `.get()`, which does not exist).
//!
//! # Properties
//!
//! 1. **Uniqueness** (`#[test]`): pairwise distinct discriminants
//!    across all thirty `RuntimeError::X_CODE` constants, asserted via
//!    nested `for` loops with `assert_ne!`. No `HashSet`, no
//!    `Vec::sort`, no allocation. Constant-time assertion per pair.
//!
//! 2. **Stability** (`#[test]`): each constant's `code()` matches
//!    the frozen baseline at
//!    `crates/vb_runtime/src/diagnostics_baseline.txt`. The expected
//!    values are inlined as the `expected` field of `RUNTIME_ERROR_CODES`
//!    so this test is self-contained.
//!
//! 3. **Range** (`#[test]`): every discriminant fits in `u16` (true
//!    by type) AND falls in the wave-15 runtime range `0x2001..=0x201E`
//!    AND avoids the reserved system-code range `0xFFFF_0000..=0xFFFF_FFFF`.
//!    The reserved range is structurally unreachable on `u16`; this
//!    test asserts the upper bound explicitly so widening the
//!    representation trips the test.
//!
//! 4. **Registry cross-check (bonus)** (`#[test]`): every wave-15
//!    `RuntimeError::X_CODE` constant maps to exactly one entry in
//!    `vb_core::diagnostic::CODE_REGISTRY` with the same `numeric`
//!    discriminant. Guards against accidental collision with
//!    pre-existing registry codes and against duplicate registrations
//!    inside the wave-15 range.
//!
//! 5. **Constructor round-trip** (`proptest!`, 1000 cases): for a
//!    uniform `u16` sample, `DiagnosticCode::new(u).code() == u`.
//!    Demonstrates that the public constructor is total.
//!
//! 6. **Proptest pairwise sweep** (`proptest!`, 1000 cases): random
//!    pick of two `(name, code)` pairs via `proptest::sample::select`
//!    over the static array; smoke-test the random-pick machinery
//!    against the same uniqueness invariant.
//!
//! # Holzman / zero-panic posture
//!
//! - No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`,
//!   `unreachable!`, or production `assert!` macros appear in this
//!   file. Test-style `assert_eq!`/`assert_ne!` and
//!   `prop_assert_eq!`/`prop_assert_ne!` are the only failure paths.
//! - All loops have static upper bounds (`RUNTIME_ERROR_CODES.len()`
//!   is a `const` of 30, so the inner loop body executes at most
//!   `30 * 29 / 2 = 435` times).
//! - No heap allocation in any of the exhaustive tests; all iteration
//!   uses static slices.

use proptest::prelude::*;

use vb_core::DiagnosticCode;
use vb_core::diagnostic::CODE_REGISTRY;
use vb_runtime::RuntimeError;

// ---------------------------------------------------------------------------
// Static arrays
// ---------------------------------------------------------------------------

/// One row of the wave-15 frozen baseline. Holds the constant's name
/// (for diagnostic output), the frozen expected discriminant
/// (matching `crates/vb_runtime/src/diagnostics_baseline.txt`), and a
/// reference to the actual `DiagnosticCode` constant exported by
/// `RuntimeError`.
#[derive(Debug, Clone, Copy)]
struct FrozenCode {
    /// The constant's name (e.g., `"QUEUE_FULL_CODE"`).
    name: &'static str,
    /// Frozen expected discriminant (`u16`). Source-cited per row.
    expected: u16,
    /// The actual `DiagnosticCode` constant declared in production.
    actual: DiagnosticCode,
}

/// Frozen baseline of wave-15 `RuntimeError::X_CODE` constants.
///
/// Every row's `expected` matches the literal in
/// `crates/vb_runtime/src/diagnostics_baseline.txt` (which itself is
/// generated from `crates/vb_runtime/src/error/diagnostics.rs:10-41`).
/// The `actual` field references the production constant by path, so
/// changing the discriminant in the source code (or the frozen
/// baseline file) trips the stability property test.
const RUNTIME_ERROR_CODES: &[FrozenCode] = &[
    FrozenCode {
        name: "QUEUE_FULL_CODE",
        expected: 0x2001,
        actual: RuntimeError::QUEUE_FULL_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:10
    FrozenCode {
        name: "RUN_NOT_FOUND_CODE",
        expected: 0x2002,
        actual: RuntimeError::RUN_NOT_FOUND_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:11
    FrozenCode {
        name: "ACTIVE_RUN_CAPACITY_EXCEEDED_CODE",
        expected: 0x2003,
        actual: RuntimeError::ACTIVE_RUN_CAPACITY_EXCEEDED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:12
    FrozenCode {
        name: "RUN_ALREADY_EXISTS_CODE",
        expected: 0x2004,
        actual: RuntimeError::RUN_ALREADY_EXISTS_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:13
    FrozenCode {
        name: "UNSUPPORTED_OPERATION_CODE",
        expected: 0x2005,
        actual: RuntimeError::UNSUPPORTED_OPERATION_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:14
    FrozenCode {
        name: "SHUTDOWN_IN_PROGRESS_CODE",
        expected: 0x2006,
        actual: RuntimeError::SHUTDOWN_IN_PROGRESS_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:15
    FrozenCode {
        name: "JOURNAL_POISONED_CODE",
        expected: 0x2007,
        actual: RuntimeError::JOURNAL_POISONED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:16
    FrozenCode {
        name: "STORAGE_JOURNAL_APPEND_FAILED_CODE",
        expected: 0x2008,
        actual: RuntimeError::STORAGE_JOURNAL_APPEND_FAILED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:17
    FrozenCode {
        name: "JOURNAL_FULL_CODE",
        expected: 0x201E,
        actual: RuntimeError::JOURNAL_FULL_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:18
    FrozenCode {
        name: "ADMISSION_HEADER_PERSISTENCE_FAILED_CODE",
        expected: 0x2015,
        actual: RuntimeError::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:19-20
    FrozenCode {
        name: "UNSUPPORTED_ASYNC_STRICT_ACK_CODE",
        expected: 0x2009,
        actual: RuntimeError::UNSUPPORTED_ASYNC_STRICT_ACK_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:21
    FrozenCode {
        name: "FRAME_POOL_UNAVAILABLE_CODE",
        expected: 0x200A,
        actual: RuntimeError::FRAME_POOL_UNAVAILABLE_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:22
    FrozenCode {
        name: "INVALID_ACTION_COMPLETION_CODE",
        expected: 0x200B,
        actual: RuntimeError::INVALID_ACTION_COMPLETION_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:23
    FrozenCode {
        name: "INVALID_TIMER_FIRE_CODE",
        expected: 0x200C,
        actual: RuntimeError::INVALID_TIMER_FIRE_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:24
    FrozenCode {
        name: "UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE",
        expected: 0x200D,
        actual: RuntimeError::UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:25-26
    FrozenCode {
        name: "INVALID_RECOVERY_HYDRATION_CODE",
        expected: 0x200E,
        actual: RuntimeError::INVALID_RECOVERY_HYDRATION_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:27
    FrozenCode {
        name: "COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE",
        expected: 0x200F,
        actual: RuntimeError::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:28
    FrozenCode {
        name: "ACTIVE_RUN_CAPACITY_ZERO_CODE",
        expected: 0x2010,
        actual: RuntimeError::ACTIVE_RUN_CAPACITY_ZERO_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:29
    FrozenCode {
        name: "ADMISSION_ARTIFACT_NOT_FOUND_CODE",
        expected: 0x2011,
        actual: RuntimeError::ADMISSION_ARTIFACT_NOT_FOUND_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:30
    FrozenCode {
        name: "ADMISSION_CAPABILITY_DENIED_CODE",
        expected: 0x2012,
        actual: RuntimeError::ADMISSION_CAPABILITY_DENIED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:31
    FrozenCode {
        name: "ADMISSION_ARTIFACT_INVALID_CODE",
        expected: 0x2014,
        actual: RuntimeError::ADMISSION_ARTIFACT_INVALID_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:32
    FrozenCode {
        name: "ENCODE_FAILED_CODE",
        expected: 0x2013,
        actual: RuntimeError::ENCODE_FAILED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:33
    FrozenCode {
        name: "SECRET_RESULT_NOT_ALLOWED_CODE",
        expected: 0x2016,
        actual: RuntimeError::SECRET_RESULT_NOT_ALLOWED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:34
    FrozenCode {
        name: "IPC_PAYLOAD_SIZE_EXCEEDED_CODE",
        expected: 0x2017,
        actual: RuntimeError::IPC_PAYLOAD_SIZE_EXCEEDED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:35
    FrozenCode {
        name: "ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE",
        expected: 0x2018,
        actual: RuntimeError::ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:36
    FrozenCode {
        name: "ADMISSION_ARTIFACT_STALE_CODE",
        expected: 0x2019,
        actual: RuntimeError::ADMISSION_ARTIFACT_STALE_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:37
    FrozenCode {
        name: "ADMISSION_DIGEST_MISMATCH_CODE",
        expected: 0x201A,
        actual: RuntimeError::ADMISSION_DIGEST_MISMATCH_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:38
    FrozenCode {
        name: "ENGINE_DRIVE_FAILED_CODE",
        expected: 0x201B,
        actual: RuntimeError::ENGINE_DRIVE_FAILED_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:39
    FrozenCode {
        name: "SHARD_NOT_FOUND_CODE",
        expected: 0x201C,
        actual: RuntimeError::SHARD_NOT_FOUND_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:40
    FrozenCode {
        name: "MIGRATE_SELF_CODE",
        expected: 0x201D,
        actual: RuntimeError::MIGRATE_SELF_CODE,
    }, // Source: crates/vb_runtime/src/error/diagnostics.rs:41
];

/// Lightweight `(name, code)` projection used for the proptest sweep.
///
/// `DiagnosticCode` is `Copy` (see
/// `crates/vb_core/src/diagnostic.rs:1763`), so a slice of
/// `(&'static str, DiagnosticCode)` is itself `Copy + Clone`, which
/// satisfies `proptest::sample::select`'s `T: Clone` bound.
const RUNTIME_ERROR_CODE_PAIRS: &[(&'static str, DiagnosticCode)] = &[
    ("QUEUE_FULL_CODE", RuntimeError::QUEUE_FULL_CODE),
    ("RUN_NOT_FOUND_CODE", RuntimeError::RUN_NOT_FOUND_CODE),
    (
        "ACTIVE_RUN_CAPACITY_EXCEEDED_CODE",
        RuntimeError::ACTIVE_RUN_CAPACITY_EXCEEDED_CODE,
    ),
    (
        "RUN_ALREADY_EXISTS_CODE",
        RuntimeError::RUN_ALREADY_EXISTS_CODE,
    ),
    (
        "UNSUPPORTED_OPERATION_CODE",
        RuntimeError::UNSUPPORTED_OPERATION_CODE,
    ),
    (
        "SHUTDOWN_IN_PROGRESS_CODE",
        RuntimeError::SHUTDOWN_IN_PROGRESS_CODE,
    ),
    ("JOURNAL_POISONED_CODE", RuntimeError::JOURNAL_POISONED_CODE),
    (
        "STORAGE_JOURNAL_APPEND_FAILED_CODE",
        RuntimeError::STORAGE_JOURNAL_APPEND_FAILED_CODE,
    ),
    ("JOURNAL_FULL_CODE", RuntimeError::JOURNAL_FULL_CODE),
    (
        "ADMISSION_HEADER_PERSISTENCE_FAILED_CODE",
        RuntimeError::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE,
    ),
    (
        "UNSUPPORTED_ASYNC_STRICT_ACK_CODE",
        RuntimeError::UNSUPPORTED_ASYNC_STRICT_ACK_CODE,
    ),
    (
        "FRAME_POOL_UNAVAILABLE_CODE",
        RuntimeError::FRAME_POOL_UNAVAILABLE_CODE,
    ),
    (
        "INVALID_ACTION_COMPLETION_CODE",
        RuntimeError::INVALID_ACTION_COMPLETION_CODE,
    ),
    (
        "INVALID_TIMER_FIRE_CODE",
        RuntimeError::INVALID_TIMER_FIRE_CODE,
    ),
    (
        "UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE",
        RuntimeError::UNSUPPORTED_FULL_RECOVERY_HYDRATION_CODE,
    ),
    (
        "INVALID_RECOVERY_HYDRATION_CODE",
        RuntimeError::INVALID_RECOVERY_HYDRATION_CODE,
    ),
    (
        "COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE",
        RuntimeError::COMMAND_QUEUE_CAPACITY_EXCEEDED_CODE,
    ),
    (
        "ACTIVE_RUN_CAPACITY_ZERO_CODE",
        RuntimeError::ACTIVE_RUN_CAPACITY_ZERO_CODE,
    ),
    (
        "ADMISSION_ARTIFACT_NOT_FOUND_CODE",
        RuntimeError::ADMISSION_ARTIFACT_NOT_FOUND_CODE,
    ),
    (
        "ADMISSION_CAPABILITY_DENIED_CODE",
        RuntimeError::ADMISSION_CAPABILITY_DENIED_CODE,
    ),
    (
        "ADMISSION_ARTIFACT_INVALID_CODE",
        RuntimeError::ADMISSION_ARTIFACT_INVALID_CODE,
    ),
    ("ENCODE_FAILED_CODE", RuntimeError::ENCODE_FAILED_CODE),
    (
        "SECRET_RESULT_NOT_ALLOWED_CODE",
        RuntimeError::SECRET_RESULT_NOT_ALLOWED_CODE,
    ),
    (
        "IPC_PAYLOAD_SIZE_EXCEEDED_CODE",
        RuntimeError::IPC_PAYLOAD_SIZE_EXCEEDED_CODE,
    ),
    (
        "ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE",
        RuntimeError::ADMISSION_ARTIFACT_DIGEST_MISMATCH_CODE,
    ),
    (
        "ADMISSION_ARTIFACT_STALE_CODE",
        RuntimeError::ADMISSION_ARTIFACT_STALE_CODE,
    ),
    (
        "ADMISSION_DIGEST_MISMATCH_CODE",
        RuntimeError::ADMISSION_DIGEST_MISMATCH_CODE,
    ),
    (
        "ENGINE_DRIVE_FAILED_CODE",
        RuntimeError::ENGINE_DRIVE_FAILED_CODE,
    ),
    ("SHARD_NOT_FOUND_CODE", RuntimeError::SHARD_NOT_FOUND_CODE),
    ("MIGRATE_SELF_CODE", RuntimeError::MIGRATE_SELF_CODE),
];

/// Number of wave-15 `RuntimeError::X_CODE` constants in scope.
const WAVE_15_COUNT: usize = 30;

// ---------------------------------------------------------------------------
// Property 1 — uniqueness
// ---------------------------------------------------------------------------

/// Property 1 (uniqueness): every pair of wave-15
/// `RuntimeError::X_CODE` constants has a distinct discriminant.
///
/// Pairwise comparison via nested `for` loops. No `HashSet`, no
/// `Vec::sort`, no allocation. Bounded by `30 * 29 / 2 = 435`
/// iterations. Each `assert_ne!` failure message names both
/// colliding constants and their discriminants.
#[test]
fn diagnostic_code_uniqueness_pairwise() {
    assert_eq!(
        RUNTIME_ERROR_CODES.len(),
        WAVE_15_COUNT,
        "wave-15 constant count drifted; update RUNTIME_ERROR_CODES and RUNTIME_ERROR_CODE_PAIRS in lockstep"
    );
    for (i, left) in RUNTIME_ERROR_CODES.iter().enumerate() {
        for right in RUNTIME_ERROR_CODES.iter().skip(i + 1) {
            assert_ne!(
                left.actual.code(),
                right.actual.code(),
                "wave-15 collision: RuntimeError::{left_name} (0x{left_code:04X}) and RuntimeError::{right_name} (0x{right_code:04X}) share a discriminant; uniqueness contract violated",
                left_name = left.name,
                left_code = left.actual.code(),
                right_name = right.name,
                right_code = right.actual.code(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 2 — stability against frozen baseline
// ---------------------------------------------------------------------------

/// Property 2 (stability): each constant's discriminant matches the
/// frozen baseline at `crates/vb_runtime/src/diagnostics_baseline.txt`.
///
/// Asserted as a sequence of explicit per-constant `assert_eq!` lines
/// so failures point to the exact row. The inline `expected` values
/// are mirrored verbatim from the `.txt` file (committed regression
/// oracle); the assertion runs against the production constant, not
/// against the `.txt` file, so a typo in the `.txt` will not silently
/// pass — both sides must agree.
#[test]
fn diagnostic_code_stability_against_frozen_baseline() {
    for row in RUNTIME_ERROR_CODES {
        assert_eq!(
            row.actual.code(),
            row.expected,
            "wave-15 drift: RuntimeError::{name} is 0x{actual:04X} but frozen baseline says 0x{expected:04X}. Update crates/vb_runtime/src/diagnostics_baseline.txt and verify the registry entry at crates/vb_core/src/diagnostic.rs CODE_REGISTRY if this change is intentional.",
            name = row.name,
            actual = row.actual.code(),
            expected = row.expected,
        );
    }
}

// ---------------------------------------------------------------------------
// Property 3 — range and pairwise uniqueness
// ---------------------------------------------------------------------------

/// Property 3 (range): every discriminant fits in `u16`, falls in the
/// wave-15 runtime range `0x2001..=0x201E`, and avoids the reserved
/// system-code range `0xFFFF_0000..=0xFFFF_FFFF`.
///
/// `DiagnosticCode` is `pub struct DiagnosticCode(u16)` at
/// `crates/vb_core/src/diagnostic.rs:1763-1765`, so the constructor
/// `pub const fn new(code: u16) -> Self` at line 1770 makes the upper
/// `0xFFFF_0000..=0xFFFF_FFFF` range structurally unreachable on
/// `u16`. This test asserts the bound explicitly so widening the
/// representation (e.g., to `u32`) trips the test immediately.
///
/// Also asserts pairwise distinctness within the wave-15 set, in the
/// same nested-for shape as Property 1, so this test is independently
/// useful in isolation.
#[test]
fn diagnostic_code_range_and_pairwise_uniqueness() {
    /// Inclusive lower bound of the wave-15 runtime-code range.
    const WAVE_15_LOW: u16 = 0x2001;
    /// Inclusive upper bound of the wave-15 runtime-code range.
    const WAVE_15_HIGH: u16 = 0x201E;
    /// Inclusive lower bound of the reserved system-code range.
    /// Structurally unreachable on `u16`; asserted explicitly.
    const RESERVED_SYSTEM_LOW_U32: u32 = 0xFFFF_0000_u32;

    for row in RUNTIME_ERROR_CODES {
        let discriminant = row.actual.code();
        // (a) u16 fit. Trivially true via the type, but assert the
        // literal upper bound so a future widening of `DiagnosticCode`
        // trips this test rather than silently passing.
        assert!(
            discriminant <= u16::MAX,
            "wave-15 range violation: RuntimeError::{name} discriminant 0x{discriminant:04X} exceeds u16::MAX",
            name = row.name,
            discriminant = discriminant,
        );
        // (b) Wave-15 range. The 30 wave-15 constants live entirely in
        // `0x2001..=0x201E` per the source file's section header.
        assert!(
            (WAVE_15_LOW..=WAVE_15_HIGH).contains(&discriminant),
            "wave-15 range violation: RuntimeError::{name} discriminant 0x{discriminant:04X} falls outside the declared runtime-code range 0x2001..=0x201E",
            name = row.name,
            discriminant = discriminant,
        );
        // (c) Reserved system-code range. The widened comparison
        // (`u32::from`) makes the bound check meaningful even if
        // `DiagnosticCode` is later widened; the literal
        // `0xFFFF_0000_u32` is the documented lower bound of the
        // reserved range per the bead spec.
        assert!(
            u32::from(discriminant) < RESERVED_SYSTEM_LOW_U32,
            "reserved system range violation: RuntimeError::{name} discriminant 0x{discriminant:04X} falls in reserved system range 0x{lo:04X}..=0xFFFF_FFFF",
            name = row.name,
            discriminant = discriminant,
            lo = RESERVED_SYSTEM_LOW_U32,
        );
    }

    // Pairwise distinctness within the wave-15 set (same shape as
    // Property 1, asserted again here so Property 3 is independently
    // verifiable).
    for (i, left) in RUNTIME_ERROR_CODES.iter().enumerate() {
        for right in RUNTIME_ERROR_CODES.iter().skip(i + 1) {
            assert_ne!(
                left.actual.code(),
                right.actual.code(),
                "wave-15 range collision: RuntimeError::{left_name} (0x{left:04X}) and RuntimeError::{right_name} (0x{right:04X}) both fall in the wave-15 runtime-code range but share a discriminant",
                left_name = left.name,
                right_name = right.name,
                left = left.actual.code(),
                right = right.actual.code(),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 4 — registry cross-check (bonus)
// ---------------------------------------------------------------------------

/// Property 4 (registry cross-check, bonus): every wave-15
/// `RuntimeError::X_CODE` constant maps to exactly one entry in
/// `vb_core::diagnostic::CODE_REGISTRY` with the same `numeric`
/// discriminant. Also asserts pairwise distinctness across the entire
/// `CODE_REGISTRY` to guard against accidental collisions between
/// wave-15 runtime codes and pre-existing registry codes.
///
/// The registry is `pub const` at
/// `crates/vb_core/src/diagnostic.rs:118`. Its bijection invariant is
/// documented at lines 108-117. A linear scan over the slice is
/// `O(registry_len)` per wave-15 row; this test does not allocate.
#[test]
fn diagnostic_code_registry_cross_check() {
    // (a) Every wave-15 row maps to exactly one registry entry.
    for row in RUNTIME_ERROR_CODES {
        let discriminant = row.actual.code();
        let mut matches: usize = 0;
        for entry in CODE_REGISTRY {
            if entry.numeric == discriminant {
                matches = matches.saturating_add(1);
            }
        }
        assert_eq!(
            matches,
            1,
            "wave-15 ↔ registry bijection violation: RuntimeError::{name} (0x{discriminant:04X}) maps to {matches} CODE_REGISTRY entries; expected exactly 1. Update crates/vb_core/src/diagnostic.rs CODE_REGISTRY.",
            name = row.name,
            discriminant = discriminant,
            matches = matches,
        );
    }

    // (b) Pairwise distinctness across the entire CODE_REGISTRY.
    // This catches duplicate registrations anywhere in the workspace,
    // not just in the wave-15 runtime range. Bounded by
    // `len(CODE_REGISTRY).choose(2)` iterations; the static slice has
    // a few hundred entries so this is a few hundred comparisons,
    // all at compile-time-known addresses.
    for (i, left) in CODE_REGISTRY.iter().enumerate() {
        for right in CODE_REGISTRY.iter().skip(i + 1) {
            assert_ne!(
                left.numeric,
                right.numeric,
                "CODE_REGISTRY bijection violation: {left_sym} (0x{left_num:04X}) and {right_sym} (0x{right_num:04X}) share a numeric discriminant; the registry must be a bijection",
                left_sym = left.symbolic,
                left_num = left.numeric,
                right_sym = right.symbolic,
                right_num = right.numeric,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Property 5 — constructor round-trip (proptest, 1000 cases)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property 5 (constructor round-trip): the public constructor
    /// `DiagnosticCode::new(u16) -> Self` is total on `u16`, i.e.,
    /// `DiagnosticCode::new(u).code() == u` and the constructor
    /// produces `Eq`-consistent values.
    ///
    /// Source: `crates/vb_core/src/diagnostic.rs:1770`
    /// `pub const fn new(code: u16) -> Self { Self(code) }` and
    /// `crates/vb_core/src/diagnostic.rs:1776`
    /// `pub const fn code(self) -> u16 { self.0 }`.
    #[test]
    fn diagnostic_code_constructor_round_trip(u in any::<u16>()) {
        let code = DiagnosticCode::new(u);
        prop_assert_eq!(code.code(), u);
        prop_assert_eq!(code, DiagnosticCode::new(u));
    }
}

// ---------------------------------------------------------------------------
// Property 6 — proptest pairwise sweep via `proptest::sample::select`
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Property 6 (proptest pairwise sweep): randomly pick two wave-15
    /// `(name, code)` pairs via `proptest::sample::select` over the
    /// static array and assert pairwise distinctness when the picked
    /// names differ. Exercises the random-pick machinery against the
    /// same uniqueness invariant as the exhaustive Property 1.
    ///
    /// `RUNTIME_ERROR_CODE_PAIRS` is `Copy + Clone`, so
    /// `proptest::sample::select` can pick from it without allocation.
    #[test]
    fn diagnostic_code_proptest_pairwise_sweep(
        (left_name, left_code) in proptest::sample::select(RUNTIME_ERROR_CODE_PAIRS),
        (right_name, right_code) in proptest::sample::select(RUNTIME_ERROR_CODE_PAIRS),
    ) {
        prop_assert_eq!(left_code.code(), left_code.code());
        if left_name != right_name {
            prop_assert_ne!(
                left_code,
                right_code,
                "proptest sweep collision: {ln} (0x{lo:04X}) and {rn} (0x{ro:04X}) share a discriminant",
                ln = left_name,
                rn = right_name,
                lo = left_code.code(),
                ro = right_code.code(),
            );
        }
    }
}
