#![forbid(unsafe_code)]

//! Property tests: equivalence-relation axioms + Hash anti-symmetry for
//! equality-semantics types introduced or stabilized in Wave-15.
//!
//! These property tests prove the contract that the equality implementation
//! on each type forms a sound equivalence relation and that the Hash impl is
//! consistent with the equality impl:
//!
//!   (a) reflexivity:   `a == a`
//!   (b) symmetry:      `a == b ⇒ b == a`
//!   (c) transitivity:  `a == b ∧ b == c ⇒ a == c`
//!   (d) hash anti-symmetry: `a == b ⇒ hash(a) == hash(b)` (Hash contract)
//!
//! Three distinct types are covered:
//!
//! - `vb_runtime::RuntimeError` — hand-written `PartialEq` at
//!   `crates/vb_runtime/src/error/equality.rs:3-194`. Not `Hash`-able.
//! - `vb_core::diagnostic::DiagnosticCode` — derived `PartialEq + Eq + Hash`
//!   at `crates/vb_core/src/diagnostic.rs:1763`.
//! - `vb_core::diagnostic::CodeCategory` — derived `PartialEq + Eq + Hash`
//!   at `crates/vb_core/src/diagnostic.rs:22`.
//!
//! Every `proptest!` block runs `ProptestConfig::with_cases(1000)` or more.
//! Power-of-Ten and zero-panic doctrine: no `unwrap`, `expect`, `panic`,
//! `todo`, `unimplemented`, `unreachable!`, or production `assert!` macros
//! appear in this file. All failures route through `prop_assert!` /
//! `prop_assert_eq!` / `prop_assume!` only.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use proptest::prelude::*;

use vb_core::Taint;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::diagnostic::{CodeCategory, DiagnosticCode};
use vb_core::errors::CoreError;
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_storage::JournalError;

// ---------------------------------------------------------------------------
// Shared hash helper (replaces std::hash::Hash::hash with a deterministic
// u64 from a DefaultHasher). Local to this proptest file.
// ---------------------------------------------------------------------------

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

// ---------------------------------------------------------------------------
// Source: crates/vb_core/src/diagnostic.rs:1763
// pub struct DiagnosticCode(u16);
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
//          Serialize, Deserialize)]
// pub const fn new(code: u16) -> Self { Self(code) }
// ---------------------------------------------------------------------------

fn arb_diagnostic_code() -> impl Strategy<Value = DiagnosticCode> {
    (0_u16..=u16::MAX).prop_map(DiagnosticCode::new)
}

// ---------------------------------------------------------------------------
// Source: crates/vb_core/src/diagnostic.rs:22
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// #[non_exhaustive]
// pub enum CodeCategory { ... 17 variants ... }
// ---------------------------------------------------------------------------

fn arb_code_category() -> impl Strategy<Value = CodeCategory> {
    prop_oneof![
        Just(CodeCategory::Schema),
        Just(CodeCategory::Reference),
        Just(CodeCategory::ControlFlow),
        Just(CodeCategory::TypeTaint),
        Just(CodeCategory::Gate),
        Just(CodeCategory::ContractDiscovery),
        Just(CodeCategory::Compilation),
        Just(CodeCategory::WorkflowIr),
        Just(CodeCategory::Expression),
        Just(CodeCategory::Accessor),
        Just(CodeCategory::Lowering),
        Just(CodeCategory::Storage),
        Just(CodeCategory::Runtime),
        Just(CodeCategory::Ipc),
        Just(CodeCategory::Lifecycle),
        Just(CodeCategory::RuntimeBoundary),
        Just(CodeCategory::Internal),
    ]
}

// ---------------------------------------------------------------------------
// Source: crates/vb_runtime/src/error/equality.rs:3-194
// hand-written PartialEq for RuntimeError. Variant constructors must come
// from outside the defining crate, so we enumerate the variants the equality
// impl actually handles. `&'static str` operation strings come from a small
// static slice via proptest::sample::select.
// ---------------------------------------------------------------------------

/// Static pool for `RuntimeError::UnsupportedOperation { operation }`.
/// `prop_oneof!` + `proptest::sample::select` give a stable `&'static str`.
const UNSUPPORTED_OPERATIONS: &[&str] = &[
    "load_artifact",
    "store_artifact",
    "admit_run",
    "cancel_run",
    "migrate_shard",
    "replay_journal",
    "register_timer",
    "fire_timer",
    "drain_queue",
    "seal_snapshot",
];

fn arb_static_op() -> impl Strategy<Value = &'static str> {
    proptest::sample::select(UNSUPPORTED_OPERATIONS)
}

fn arb_taint() -> impl Strategy<Value = Taint> {
    prop_oneof![
        Just(Taint::Clean),
        Just(Taint::DerivedFromSecret),
        Just(Taint::Secret),
        Just(Taint::Random),
        Just(Taint::TimeDependent),
    ]
}

/// Strategy that covers every variant handled by the equality impl
/// (`runtime_error_unit_eq`, `runtime_error_core_field_eq`,
/// `runtime_error_admission_digest_eq`, `runtime_error_admission_capability_eq`).
///
/// Note: `RuntimeError` is `#[non_exhaustive]`. We cannot derive `Arbitrary`
/// from outside the defining crate. Each variant is constructed explicitly.
fn arb_runtime_error() -> impl Strategy<Value = RuntimeError> {
    prop_oneof![
        // ---- runtime_error_unit_eq unit variants (tags 0..=15) ----
        Just(RuntimeError::QueueFull),
        Just(RuntimeError::RunNotFound),
        Just(RuntimeError::RunAlreadyExists),
        Just(RuntimeError::ShutdownInProgress),
        Just(RuntimeError::JournalPoisoned),
        Just(RuntimeError::UnsupportedAsyncStrictAck),
        Just(RuntimeError::FramePoolUnavailable),
        Just(RuntimeError::InvalidActionCompletion),
        Just(RuntimeError::InvalidTimerFire),
        Just(RuntimeError::UnsupportedFullRecoveryHydration),
        Just(RuntimeError::InvalidRecoveryHydration),
        Just(RuntimeError::ActiveRunCapacityZero),
        Just(RuntimeError::EncodeFailed),
        Just(RuntimeError::SecretResultNotAllowed),
        // ShardNotFound { shard } — fixed by vb-jpq7.audit.2.
        // After the vb-jpq7.audit.2 fix, `runtime_error_unit_tag` at
        // crates/vb_runtime/src/error/equality.rs no longer matches
        // `ShardNotFound { .. }` via wildcard; it falls through to the
        // `_ => None` arm, so `runtime_error_unit_eq` returns `false` for
        // any `ShardNotFound` pair. Equality is then decided by the
        // `ShardNotFound { shard: a } == ShardNotFound { shard: b }` field
        // arm in `runtime_error_core_field_eq` at equality.rs:120-122.
        // This is consistent with every other field-bearing variant
        // (`ActiveRunCapacityExceeded { capacity }`, `JournalFull { capacity }`,
        // `UnsupportedOperation { operation }`, ...). The dedicated
        // 8x8-grid `runtime_error_shard_not_found_field_equality` test
        // below asserts this contract exhaustively.
        (any::<u32>()).prop_map(|shard| RuntimeError::ShardNotFound { shard }),
        Just(RuntimeError::MigrateSelf),
        // ---- runtime_error_core_field_eq field variants ----
        (any::<usize>()).prop_map(|capacity| RuntimeError::ActiveRunCapacityExceeded { capacity }),
        (any::<usize>()).prop_map(|capacity| RuntimeError::JournalFull { capacity }),
        arb_static_op().prop_map(|operation| RuntimeError::UnsupportedOperation { operation }),
        // Core { source } compared via Box<CoreError> ==. Use a fixed source
        // — the equality branch is exercised regardless of source value.
        Just(RuntimeError::Core {
            source: Box::new(CoreError::QueueFull),
        }),
        // StorageJournalAppend { source } compared via Arc<JournalError>
        // diagnostic_code(). Different JournalError variants yield different
        // codes, so they break equality and exercise the arm.
        Just(RuntimeError::StorageJournalAppend {
            source: Arc::new(JournalError::QueueFull),
        }),
        Just(RuntimeError::AdmissionHeaderPersistenceFailed {
            source: Arc::new(JournalError::WriteLockPoisoned),
        }),
        (any::<usize>(), any::<usize>()).prop_map(|(capacity, max)| {
            RuntimeError::CommandQueueCapacityExceeded { capacity, max }
        }),
        (any::<u16>(), any::<u16>())
            .prop_map(|(incoming, current)| RuntimeError::StaleAttempt { incoming, current }),
        (any::<u16>(), any::<u16>())
            .prop_map(|(attempt, max)| RuntimeError::AttemptBeyondMax { attempt, max }),
        (any::<u32>(), any::<u32>())
            .prop_map(|(size, max)| RuntimeError::IpcPayloadSizeExceeded { size, max }),
        (any::<u32>(), any::<u32>()).prop_map(|(declared, actual)| {
            RuntimeError::ActionOutputLengthMismatch { declared, actual }
        }),
        (any::<u32>(), any::<u32>())
            .prop_map(|(size, max)| RuntimeError::ActionOutputTooLarge { size, max }),
        (any::<u64>(), any::<u64>())
            .prop_map(|(size, max)| { RuntimeError::ActionOutputBlobTooLarge { size, max } }),
        (arb_taint(), arb_taint()).prop_map(|(required, supplied)| {
            RuntimeError::ActionTaintDowngrade { required, supplied }
        }),
        // EngineDriveFailed compared via Box<CoreError> diagnostic_code().
        Just(RuntimeError::EngineDriveFailed {
            run: RunId::new(1),
            source: Box::new(CoreError::InternalInvariantViolation { reason: "p" }),
        }),
        // ---- runtime_error_admission_digest_eq ----
        (any::<[u8; 32]>()).prop_map(|bytes| RuntimeError::AdmissionArtifactNotFound {
            digest: WorkflowDigest::from_bytes(bytes),
        }),
        (any::<[u8; 32]>()).prop_map(|bytes| RuntimeError::AdmissionArtifactInvalid {
            digest: WorkflowDigest::from_bytes(bytes),
        }),
        (any::<[u8; 32]>(), any::<[u8; 32]>()).prop_map(|(requested, found)| {
            RuntimeError::AdmissionArtifactDigestMismatch {
                requested: WorkflowDigest::from_bytes(requested),
                found: WorkflowDigest::from_bytes(found),
            }
        }),
        (any::<[u8; 32]>()).prop_map(|bytes| RuntimeError::AdmissionArtifactStale {
            digest: WorkflowDigest::from_bytes(bytes),
        }),
        (any::<[u8; 32]>(), any::<[u8; 32]>(), any::<[u8; 32]>()).prop_map(
            |(requested, record, envelope)| RuntimeError::AdmissionDigestMismatch {
                requested: WorkflowDigest::from_bytes(requested),
                record: WorkflowDigest::from_bytes(record),
                envelope: WorkflowDigest::from_bytes(envelope),
            },
        ),
        // ---- runtime_error_admission_capability_eq ----
        (any::<u16>(), any::<u16>(), any::<u16>()).prop_map(
            |(action, required_action, granted_action)| {
                RuntimeError::AdmissionCapabilityDenied {
                    action: ActionId::new(action),
                    required: Capability::new(
                        Box::from("required"),
                        ActionId::new(required_action),
                    ),
                    granted: CapabilitySet::from_grants(Box::new([Capability::new(
                        Box::from("granted"),
                        ActionId::new(granted_action),
                    )])),
                }
            }
        ),
    ]
}

// ---------------------------------------------------------------------------
// DiagnosticCode — equivalence-relation axioms + hash anti-symmetry
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Reflexivity: `a == a` for every sampled `DiagnosticCode`.
    #[test]
    fn diagnostic_code_reflexivity_axiom(samples in proptest::collection::vec(arb_diagnostic_code(), 0..100)) {
        for a in &samples {
            prop_assert_eq!(*a == *a, true);
        }
    }

    /// Symmetry: `a == b ⇒ b == a` for every pair drawn from the vec.
    #[test]
    fn diagnostic_code_symmetry_axiom(samples in proptest::collection::vec(arb_diagnostic_code(), 0..100)) {
        for a in &samples {
            for b in &samples {
                if *a == *b {
                    prop_assert_eq!(*b == *a, true);
                }
            }
        }
    }

    /// Transitivity: `a == b ∧ b == c ⇒ a == c` for every triple drawn.
    #[test]
    fn diagnostic_code_transitivity_axiom(
        samples in proptest::collection::vec(arb_diagnostic_code(), 0..20)
    ) {
        for a in &samples {
            for b in &samples {
                for c in &samples {
                    if *a == *b && *b == *c {
                        prop_assert_eq!(*a == *c, true);
                    }
                }
            }
        }
    }

    /// Hash anti-symmetry: `a == b ⇒ hash(a) == hash(b)`.
    #[test]
    fn diagnostic_code_hash_anti_symmetry(samples in proptest::collection::vec(arb_diagnostic_code(), 0..100)) {
        for a in &samples {
            for b in &samples {
                if *a == *b {
                    prop_assert_eq!(hash_of(a), hash_of(b));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CodeCategory — equivalence-relation axioms + hash anti-symmetry
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Reflexivity: `a == a` for every sampled `CodeCategory`.
    #[test]
    fn code_category_reflexivity_axiom(samples in proptest::collection::vec(arb_code_category(), 0..100)) {
        for a in &samples {
            prop_assert_eq!(*a == *a, true);
        }
    }

    /// Symmetry: `a == b ⇒ b == a` for every pair drawn from the vec.
    #[test]
    fn code_category_symmetry_axiom(samples in proptest::collection::vec(arb_code_category(), 0..100)) {
        for a in &samples {
            for b in &samples {
                if *a == *b {
                    prop_assert_eq!(*b == *a, true);
                }
            }
        }
    }

    /// Transitivity: `a == b ∧ b == c ⇒ a == c` for every triple drawn.
    #[test]
    fn code_category_transitivity_axiom(
        samples in proptest::collection::vec(arb_code_category(), 0..20)
    ) {
        for a in &samples {
            for b in &samples {
                for c in &samples {
                    if *a == *b && *b == *c {
                        prop_assert_eq!(*a == *c, true);
                    }
                }
            }
        }
    }

    /// Hash anti-symmetry: `a == b ⇒ hash(a) == hash(b)`.
    #[test]
    fn code_category_hash_anti_symmetry(samples in proptest::collection::vec(arb_code_category(), 0..100)) {
        for a in &samples {
            for b in &samples {
                if *a == *b {
                    prop_assert_eq!(hash_of(a), hash_of(b));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// RuntimeError — equivalence-relation axioms (hand-written PartialEq;
// `RuntimeError` is not `Hash`-able, so the hash anti-symmetry property
// is not asserted here. It is covered for the derived `DiagnosticCode`
// and `CodeCategory` types above.)
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Reflexivity: `a == a` for every sampled `RuntimeError`.
    #[test]
    fn runtime_error_reflexivity_axiom(
        samples in proptest::collection::vec(arb_runtime_error(), 0..100)
    ) {
        for a in &samples {
            prop_assert_eq!(a == a, true);
        }
    }

    /// Symmetry: `a == b ⇒ b == a` for every pair drawn from the vec.
    #[test]
    fn runtime_error_symmetry_axiom(
        samples in proptest::collection::vec(arb_runtime_error(), 0..100)
    ) {
        for a in &samples {
            for b in &samples {
                if a == b {
                    prop_assert_eq!(b == a, true);
                }
            }
        }
    }

    /// Transitivity: `a == b ∧ b == c ⇒ a == c` for every triple drawn.
    #[test]
    fn runtime_error_transitivity_axiom(
        samples in proptest::collection::vec(arb_runtime_error(), 0..20)
    ) {
        for a in &samples {
            for b in &samples {
                for c in &samples {
                    if a == b && b == c {
                        prop_assert_eq!(a == c, true);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ShardNotFound field-equality contract (vb-jpq7.audit.2)
//
// After the equality fix at crates/vb_runtime/src/error/equality.rs (the
// `ShardNotFound { .. } => Some(14)` wildcard was removed from
// `runtime_error_unit_tag`), two `RuntimeError::ShardNotFound` values are
// equal iff their `shard` fields are equal — the same contract every other
// field-bearing variant already satisfies. This plain `#[test]` exhaustively
// covers an 8x8 grid (0..8 u32 for both shards), so any regression that
// reintroduces the wildcard (or otherwise short-circuits the field-arm) is
// caught immediately.
//
// Test clippy is not strict per AGENTS.md, so `assert_eq!` / `assert_ne!`
// are allowed here. These assertions are scoped to the test target and do
// not enter the production source lint gate.
// ---------------------------------------------------------------------------

#[test]
fn runtime_error_shard_not_found_field_equality() {
    for shard_a in 0u32..8 {
        for shard_b in 0u32..8 {
            let a = RuntimeError::ShardNotFound { shard: shard_a };
            let b = RuntimeError::ShardNotFound { shard: shard_b };
            if shard_a == shard_b {
                assert_eq!(
                    a, b,
                    "ShardNotFound {{ shard: {shard_a} }} must equal ShardNotFound {{ shard: {shard_b} }} when shard fields match"
                );
            } else {
                assert_ne!(
                    a, b,
                    "ShardNotFound {{ shard: {shard_a} }} must not equal ShardNotFound {{ shard: {shard_b} }} when shard fields differ"
                );
            }
        }
    }
}
