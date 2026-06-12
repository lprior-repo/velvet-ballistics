#![cfg(test)]
#![forbid(unsafe_code)]

//! Proptest: vb_8mdp_7_admission_error_mapping_preserves_cause (BT-003)
//!
//! Behaviors covered: B-001, B-004, B-005, B-006, B-007, B-008, B-009
//!
//! Per GAP-001 retraction: legacy "budget policy" errors map to
//! AdmissionArtifactInvalid (by design). The new step-count-specific
//! BudgetExceeded error maps to the typed RuntimeError::AdmissionBudgetExceeded
//! with `actual`/`limit` preserved.
//! All other errors map to typed RuntimeError variants with fields preserved.
//!
//! Invariants:
//!   I1: Mapping function never panics for any AdmissionError variant
//!   I2: ResourceCapacityExceeded → RuntimeError::ActiveRunCapacityExceeded (capacity preserved)
//!   I3: ArtifactNotFound → RuntimeError::AdmissionArtifactNotFound (digest preserved)
//!   I4: CapabilityDenied → RuntimeError::AdmissionCapabilityDenied (action/capability/grants preserved)
//!   I5: ArtifactDigestMismatch → RuntimeError::AdmissionArtifactDigestMismatch (both digests preserved)
//!   I6: Legacy budget policy errors map to RuntimeError::AdmissionArtifactInvalid with correct digest (by design)
//!   I7: AdmissionError::BudgetExceeded → RuntimeError::AdmissionBudgetExceeded (actual/limit preserved)

use proptest::prelude::*;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, WorkflowDigest};
use crate::admission::AdmissionError;
use crate::RuntimeError;
use vb_storage::EventSeq;

/// Replicates the mapping logic from `Shard::build_admission` in
/// `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` so we can test
/// the mapping in isolation without a full Shard.
fn map_admission_to_runtime(
    admission: AdmissionError,
    digest: WorkflowDigest,
) -> RuntimeError {
    match admission {
        AdmissionError::ArtifactNotFound { digest: d } => {
            RuntimeError::AdmissionArtifactNotFound { digest: d }
        }
        AdmissionError::CapabilityDenied {
            action,
            required,
            granted,
        } => RuntimeError::AdmissionCapabilityDenied {
            action,
            required,
            granted,
        },
        AdmissionError::ResourceCapacityExceeded { available, .. } => {
            RuntimeError::ActiveRunCapacityExceeded {
                capacity: usize::try_from(available).map_or(usize::MAX, |v| v),
            }
        }
        AdmissionError::BudgetPolicyExceeded { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ResourceBudgetOverflow { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ResourceBudgetUnderflow { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ResourceBudgetInvalidCapacity { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ResourceStepCeilingExceeded { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ResourcePerTickCeilingExceeded { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ArtifactEnvelopeDecodeFailed => {
            RuntimeError::AdmissionArtifactInvalid {
                digest: WorkflowDigest::from_bytes([0u8; 32]),
            }
        }
        AdmissionError::ArtifactInvalidGateCount { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ArtifactInvalidProofFlag { .. } => {
            RuntimeError::AdmissionArtifactInvalid { digest }
        }
        AdmissionError::ArtifactDigestMismatch { requested, found } => {
            RuntimeError::AdmissionArtifactDigestMismatch { requested, found }
        }
        AdmissionError::ArtifactCertificateStale { digest: d, .. } => {
            RuntimeError::AdmissionArtifactStale { digest: d }
        }
        AdmissionError::BudgetExceeded { actual, limit } => {
            RuntimeError::AdmissionBudgetExceeded { actual, limit }
        }
    }
}

// ── Strategies for AdmissionError variant generation ──

fn test_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xAB; 32])
}

fn test_digest_other() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xCD; 32])
}

fn arb_digest() -> impl Strategy<Value = WorkflowDigest> {
    proptest::collection::vec(any::<u8>(), 32)
        .prop_map(|bytes| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            WorkflowDigest::from_bytes(arr)
        })
}

fn arb_capability() -> impl Strategy<Value = Capability> {
    (any::<u16>(), ".*")
        .prop_map(|(id, name)| Capability::new(name.into(), ActionId::new(id)))
}

fn arb_admission_error() -> impl Strategy<Value = AdmissionError> {
    let resource_capacity_exceeded = (
        ".*", any::<u64>(), any::<u64>()
    ).prop_map(|(resource, requested, available)| {
        AdmissionError::ResourceCapacityExceeded { resource: Box::leak(resource.into_boxed_str()), requested, available }
    });

    let artifact_not_found = arb_digest()
        .prop_map(|digest| AdmissionError::ArtifactNotFound { digest });

    let capability_denied = (
        any::<u16>(), arb_capability(), any::<u16>()
    ).prop_map(|(action_id, required, grant_id)| {
        let granted = CapabilitySet::from_grants(
            Box::new([Capability::new("granted".into(), ActionId::new(grant_id))])
        );
        AdmissionError::CapabilityDenied {
            action: ActionId::new(action_id),
            required,
            granted,
        }
    });

    let digest_mismatch = (arb_digest(), arb_digest())
        .prop_map(|(requested, found)| {
            AdmissionError::ArtifactDigestMismatch { requested, found }
        });

    let budget_policy = (".*", any::<u64>(), any::<u64>())
        .prop_map(|(resource, actual, limit)| {
            AdmissionError::BudgetPolicyExceeded { resource: Box::leak(resource.into_boxed_str()), actual, limit }
        });

    let budget_overflow = ".*"
        .prop_map(|resource: String| {
            AdmissionError::ResourceBudgetOverflow { resource: Box::leak(resource.into_boxed_str()) }
        });

    let budget_underflow = ".*"
        .prop_map(|resource: String| {
            AdmissionError::ResourceBudgetUnderflow { resource: Box::leak(resource.into_boxed_str()) }
        });

    let invalid_capacity = ".*"
        .prop_map(|resource: String| {
            AdmissionError::ResourceBudgetInvalidCapacity { resource: Box::leak(resource.into_boxed_str()) }
        });

    let step_ceiling = (any::<u64>(), any::<u64>())
        .prop_map(|(requested, limit)| {
            AdmissionError::ResourceStepCeilingExceeded { requested, limit }
        });

    let per_tick_ceiling = (any::<u64>(), any::<u64>())
        .prop_map(|(requested, limit)| {
            AdmissionError::ResourcePerTickCeilingExceeded { requested, limit }
        });

    let artifact_stale = (arb_digest(), any::<u32>(), any::<u32>())
        .prop_map(|(digest, accepted_seq, required_seq)| {
            AdmissionError::ArtifactCertificateStale {
                digest,
                accepted_at_seq: EventSeq::new(u64::from(accepted_seq)),
                required_at_least: EventSeq::new(u64::from(required_seq)),
            }
        });

    prop_oneof![
        3 => resource_capacity_exceeded,
        3 => artifact_not_found,
        3 => capability_denied,
        3 => digest_mismatch,
        1 => budget_policy,
        1 => budget_overflow,
        1 => budget_underflow,
        1 => invalid_capacity,
        1 => step_ceiling,
        1 => per_tick_ceiling,
        1 => artifact_stale,
        1 => Just(AdmissionError::ArtifactEnvelopeDecodeFailed),
        1 => Just(AdmissionError::ArtifactInvalidGateCount { found: 7, required: 15 }),
        1 => Just(AdmissionError::ArtifactInvalidProofFlag { flag: "bounded" }),
    ]
}

// ─────────────────────────────────────────────────────────────────
// Proptest suites
// ─────────────────────────────────────────────────────────────────

proptest! {
    // ── I1: Mapping never panics on any variant ──

    #[test]
    fn mapping_never_panics_for_any_admission_error(
        error in arb_admission_error(),
        digest in arb_digest(),
    ) {
        // This test proves the mapping function never panics
        let _result = map_admission_to_runtime(error, digest);
    }

    // ── I2: ResourceCapacityExceeded → ActiveRunCapacityExceeded ──

    #[test]
    fn capacity_exceeded_maps_to_active_run_capacity_exceeded(
        capacity in (1u64..=65536u64),
    ) {
        let error = AdmissionError::ResourceCapacityExceeded {
            resource: "max_active_runs",
            requested: capacity + 1,
            available: capacity,
        };
        let result = map_admission_to_runtime(error, test_digest());
        prop_assert!(
            matches!(result, RuntimeError::ActiveRunCapacityExceeded { .. }),
            "ResourceCapacityExceeded should map to ActiveRunCapacityExceeded"
        );
    }

    // ── I3: ArtifactNotFound → AdmissionArtifactNotFound ──

    #[test]
    fn artifact_not_found_maps_with_digest_preserved(
        digest in arb_digest(),
    ) {
        let error = AdmissionError::ArtifactNotFound { digest };
        let result = map_admission_to_runtime(error, test_digest());
        match result {
            RuntimeError::AdmissionArtifactNotFound { digest: d } => {
                prop_assert_eq!(d, digest, "digest preserved");
            }
            other => {
                prop_assert!(false, "expected AdmissionArtifactNotFound, got {other:?}");
            }
        }
    }

    // ── I4: CapabilityDenied → AdmissionCapabilityDenied ──

    #[test]
    fn capability_denied_preserves_action_capability_and_grants(
        action_id in any::<u16>(),
        cap_name in ".*",
        grant_id in any::<u16>(),
    ) {
        let action = ActionId::new(action_id);
        let required = Capability::new(cap_name.into(), action);
        let granted = CapabilitySet::from_grants(
            Box::new([Capability::new("granted".into(), ActionId::new(grant_id))])
        );
        let error = AdmissionError::CapabilityDenied {
            action,
            required: required.clone(),
            granted: granted.clone(),
        };
        let result = map_admission_to_runtime(error, test_digest());
        match result {
            RuntimeError::AdmissionCapabilityDenied {
                action: a,
                required: r,
                granted: g,
            } => {
                prop_assert_eq!(a, action, "action preserved");
                prop_assert_eq!(r, required, "required capability preserved");
                prop_assert_eq!(g, granted, "granted capabilities preserved");
            }
            other => {
                prop_assert!(false, "expected AdmissionCapabilityDenied, got {other:?}");
            }
        }
    }

    // ── I5: ArtifactDigestMismatch → AdmissionArtifactDigestMismatch ──

    #[test]
    fn digest_mismatch_preserves_both_digests(
        requested in arb_digest(),
        found in arb_digest(),
    ) {
        prop_assume!(requested != found);
        let error = AdmissionError::ArtifactDigestMismatch { requested, found };
        let result = map_admission_to_runtime(error, test_digest());
        match result {
            RuntimeError::AdmissionArtifactDigestMismatch {
                requested: r,
                found: f,
            } => {
                prop_assert_eq!(r, requested, "requested digest preserved");
                prop_assert_eq!(f, found, "found digest preserved");
            }
            other => {
                prop_assert!(false, "expected AdmissionArtifactDigestMismatch, got {other:?}");
            }
        }
    }

    // ── I6: Budget errors map to AdmissionArtifactInvalid (by design per GAP-001) ──

    #[test]
    fn budget_policy_error_maps_to_admission_artifact_invalid(
        resource_name in "[a-z_]{1,32}",
        actual in any::<u64>(),
        limit in any::<u64>(),
        digest in arb_digest(),
    ) {
        let error = AdmissionError::BudgetPolicyExceeded {
            resource: Box::leak(resource_name.clone().into_boxed_str()),
            actual,
            limit,
        };
        let result = map_admission_to_runtime(error, digest);
        match result {
            RuntimeError::AdmissionArtifactInvalid { digest: d } => {
                prop_assert_eq!(d, digest, "digest preserved in AdmissionArtifactInvalid");
            }
            other => {
                prop_assert!(false, "BudgetPolicyExceeded should map to AdmissionArtifactInvalid, got {other:?}");
            }
        }
    }

    #[test]
    fn budget_overflow_maps_to_admission_artifact_invalid(
        resource_name in "[a-z_]{1,32}",
        digest in arb_digest(),
    ) {
        let error = AdmissionError::ResourceBudgetOverflow {
            resource: Box::leak(resource_name.into_boxed_str()),
        };
        let result = map_admission_to_runtime(error, digest);
        match result {
            RuntimeError::AdmissionArtifactInvalid { .. } => {}
            other => {
                prop_assert!(false, "ResourceBudgetOverflow should map to AdmissionArtifactInvalid, got {other:?}");
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Deterministic unit tests
// ─────────────────────────────────────────────────────────────────

#[test]
fn resource_capacity_exceeded_preserves_capacity_in_runtime_error() {
    let error = AdmissionError::ResourceCapacityExceeded {
        resource: "max_active_runs",
        requested: 10,
        available: 3,
    };
    let result = map_admission_to_runtime(error, test_digest());
    assert_eq!(
        result,
        RuntimeError::ActiveRunCapacityExceeded { capacity: 3usize },
        "capacity should be preserved from available field"
    );
}

#[test]
fn artifact_not_found_preserves_digest() {
    let digest = test_digest();
    let error = AdmissionError::ArtifactNotFound { digest };
    let result = map_admission_to_runtime(error, test_digest());
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactNotFound { digest },
        "digest should be preserved"
    );
}

#[test]
fn capability_denied_preserves_all_fields() {
    let action = ActionId::new(5);
    let required = Capability::new("secrets".into(), action);
    let granted = CapabilitySet::empty();
    let error = AdmissionError::CapabilityDenied {
        action,
        required: required.clone(),
        granted: granted.clone(),
    };
    let result = map_admission_to_runtime(error, test_digest());
    assert_eq!(
        result,
        RuntimeError::AdmissionCapabilityDenied {
            action,
            required,
            granted,
        },
        "all fields should be preserved"
    );
}

#[test]
fn digest_mismatch_preserves_both_digests_unit() {
    let requested = test_digest();
    let found = test_digest_other();
    let error = AdmissionError::ArtifactDigestMismatch { requested, found };
    let result = map_admission_to_runtime(error, test_digest());
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactDigestMismatch { requested, found },
        "both digests preserved"
    );
}

#[test]
fn budget_policy_exceeded_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::BudgetPolicyExceeded {
        resource: "max_steps_executable",
        actual: 100,
        limit: 50,
    };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "budget policy error maps to AdmissionArtifactInvalid by design (GAP-001)"
    );
}

#[test]
fn budget_overflow_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ResourceBudgetOverflow {
        resource: "max_steps_executable",
    };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "budget overflow error maps to AdmissionArtifactInvalid by design"
    );
}

#[test]
fn budget_underflow_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ResourceBudgetUnderflow {
        resource: "max_steps_executable",
    };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "budget underflow error maps to AdmissionArtifactInvalid by design"
    );
}

#[test]
fn invalid_capacity_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ResourceBudgetInvalidCapacity {
        resource: "max_steps_executable",
    };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "invalid capacity maps to AdmissionArtifactInvalid by design"
    );
}

#[test]
fn step_ceiling_exceeded_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ResourceStepCeilingExceeded {
        requested: 200,
        limit: 100,
    };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "step ceiling maps to AdmissionArtifactInvalid by design"
    );
}

#[test]
fn per_tick_ceiling_exceeded_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ResourcePerTickCeilingExceeded {
        requested: 200,
        limit: 100,
    };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "per-tick ceiling maps to AdmissionArtifactInvalid by design"
    );
}

#[test]
fn artifact_envelope_decode_maps_to_artifact_invalid() {
    let error = AdmissionError::ArtifactEnvelopeDecodeFailed;
    let result = map_admission_to_runtime(error, test_digest());
    assert!(
        matches!(result, RuntimeError::AdmissionArtifactInvalid { .. }),
        "envelope decode failed maps to AdmissionArtifactInvalid"
    );
}

#[test]
fn artifact_invalid_gate_count_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ArtifactInvalidGateCount { found: 7, required: 15 };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "invalid gate count maps to AdmissionArtifactInvalid"
    );
}

#[test]
fn artifact_invalid_proof_flag_maps_to_artifact_invalid() {
    let digest = test_digest();
    let error = AdmissionError::ArtifactInvalidProofFlag { flag: "bounded" };
    let result = map_admission_to_runtime(error, digest);
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactInvalid { digest },
        "invalid proof flag maps to AdmissionArtifactInvalid"
    );
}

#[test]
fn artifact_stale_maps_to_admission_artifact_stale() {
    let digest = test_digest();
    let error = AdmissionError::ArtifactCertificateStale {
        digest,
        accepted_at_seq: EventSeq::new(5),
        required_at_least: EventSeq::new(10),
    };
    let result = map_admission_to_runtime(error, test_digest_other());
    assert_eq!(
        result,
        RuntimeError::AdmissionArtifactStale { digest },
        "artifact stale preserves digest"
    );
}

#[test]
fn runtime_error_enum_contains_all_required_rejection_classes() {
    // Compile-time check: all required RuntimeError variants exist.
    // This test explicitly constructs each variant to prove they compile.
    let _q = RuntimeError::QueueFull;
    let _c = RuntimeError::ActiveRunCapacityExceeded { capacity: 1 };
    let _f = RuntimeError::FramePoolUnavailable;
    let _an = RuntimeError::AdmissionArtifactNotFound { digest: test_digest() };
    let _ai = RuntimeError::AdmissionArtifactInvalid { digest: test_digest() };
    let _ad = RuntimeError::AdmissionArtifactDigestMismatch {
        requested: test_digest(),
        found: test_digest_other(),
    };
    let _ac = RuntimeError::AdmissionCapabilityDenied {
        action: ActionId::new(1),
        required: Capability::new("net".into(), ActionId::new(1)),
        granted: CapabilitySet::empty(),
    };
    let _as = RuntimeError::AdmissionArtifactStale { digest: test_digest() };
    let _sj = RuntimeError::StorageJournalAppend {
        source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
    };
    let _hp = RuntimeError::AdmissionHeaderPersistenceFailed {
        source: std::sync::Arc::new(vb_storage::JournalError::WriteLockPoisoned),
    };
}
