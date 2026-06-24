#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::get_first,
    clippy::indexing_slicing
)]
//!
//! Manual-QA tests for wave-16 fixes (RP-018, RP-019, RA-018).
//!
//! Reviewer: MQA01
//! Three scenarios exercise the public API surface of the three wave-16
//! fixes. Each test asserts the documented contract after the fix lands
//! on main.

use vb_core::action::{
    ActionContract, ActionName, ActionTicket, Idempotency, RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::action::ActionRegistry;
use vb_runtime::action_queue::{BackpressureWarning, BoundedActionCompletionQueue};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError,
    admit_artifact_run_with_certificate_floor,
};
use vb_storage::EventSeq;
use vb_storage::admission::{AcceptedArtifact, VerificationProof};

const REQUIRED_GATE_COUNT: u8 = 15;

// ---------------------------------------------------------------------------
// Test fixture: an `AcceptedArtifactStore` that always returns a single
// canned artifact with the requested required capabilities.
// ---------------------------------------------------------------------------

struct FixedAcceptedStore {
    artifact: AcceptedArtifact,
}

impl AcceptedArtifactStore for FixedAcceptedStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn test_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0xA9; 32])
}

fn accepted_artifact_with_caps(required_capabilities: Box<[Capability]>) -> AcceptedArtifact {
    let digest = test_digest();
    AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: digest,
        ir: Vec::new(),
        verification: VerificationProof {
            digest,
            gate_count: REQUIRED_GATE_COUNT,
            durable: true,
            bounded_claimed: true,
            taint_safe_claimed: true,
            retry_safe_claimed: true,
            idempotency_verified_claimed: true,
            replayable_claimed: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: EventSeq::new(0),
        required_capabilities,
    }
}

fn cap(name: &str, action: u16) -> Capability {
    Capability::new(name.into(), ActionId::new(action))
}

fn contract_with_id(id: u16) -> ActionContract {
    // Hardcoded-valid literal name. The test calls this with fixed ids
    // (1000 and 500) — both are known at compile time, so we can compute
    // the name as a `&'static str` via `concat!` and pass it to the
    // infallible constructor. Using per-id unique literals avoids the
    // `by_name` duplicate-name rejection in `ActionRegistry::register`.
    //
    // We use `concat!` with the decimal expansion of each id rather than
    // `format!()` so the result is a `&'static str` and we can rely on the
    // infallible constructor's compile-time contract.
    let name: &'static str = match id {
        1000 => "mqa-wave16-1000",
        500 => "mqa-wave16-500",
        _ => "mqa-wave16-default",
    };
    ActionContract {
        id: ActionId::new(id),
        name: ActionName::from_static_infallible(name),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5_000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }
}

fn make_ticket(seq: u32) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(u64::from(seq)),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: u128::from(seq),
        capacity: 1,
    }
}

// ===========================================================================
// Scenario 1 (RP-018): ActionRegistry::register with a high ID.
//   Author: RP018Fix
// ===========================================================================

#[test]
fn mqa01_scenario1_rp018_len_counts_registered_not_sparse_slots() {
    // Given an empty ActionRegistry
    let mut registry = ActionRegistry::new();

    // Sanity precondition: empty registry reports len == 0.
    assert_eq!(registry.len(), 0, "empty registry should report len=0");
    assert!(
        registry.is_empty(),
        "empty registry should report is_empty()=true"
    );

    // When registering ONE action contract at a sparse / high id (1000)
    let high_id: u16 = 1000;
    let result = registry.register(contract_with_id(high_id));

    // Then registration succeeds
    assert_eq!(result, Ok(()), "register(high_id={high_id}) should succeed");

    // And the registry reports `len() == 1` (registered count), NOT
    // `slots.len() == 1001` (the buggy pre-fix behaviour).
    assert_eq!(
        registry.len(),
        1,
        "len() must equal the registered count (1), not the slot index + 1 ({})",
        usize::from(high_id) + 1
    );
    assert!(
        !registry.is_empty(),
        "is_empty() must be false after a single register"
    );

    // Register a second sparse action — len must become 2, not
    // high_id_2 + 1.
    let second_id: u16 = 500;
    let register_result = registry.register(contract_with_id(second_id));
    assert!(
        register_result.is_ok(),
        "second register must succeed; got {register_result:?}"
    );
    assert_eq!(
        registry.len(),
        2,
        "len() must reflect two registered actions, not slot capacity"
    );
}

// ===========================================================================
// Scenario 2 (RP-019): BoundedActionCompletionQueue backpressure for
//   capacity=7 must trigger at depth=6 (ceiling of 80%) and not at
//   depth=5 (71% — below documented threshold).
//   Author: RP019Fix
// ===========================================================================

#[test]
fn mqa01_scenario2_rp019_backpressure_threshold_7_is_6_not_5() {
    // --- Part A: depth 6 of 7 (86%) MUST trigger a backpressure warning. ---
    // Use the infallible constructor: capacity=7 is a hardcoded-valid literal
    // (1..=MAX_ACTION_COMPLETION_QUEUE_CAPACITY), so the caller-bound
    // invariant holds. The internal panic path is encapsulated inside
    // `with_backpressure_infallible` and unreachable on this call path.
    let (queue_with_bp, rx) = BoundedActionCompletionQueue::with_backpressure_infallible(7);
    assert_eq!(queue_with_bp.capacity(), 7);

    let mut warnings: Vec<BackpressureWarning> = Vec::new();
    for i in 0..6u32 {
        let enqueue_result = queue_with_bp.enqueue(make_ticket(i));
        assert!(
            enqueue_result.is_ok(),
            "enqueue within capacity; got {enqueue_result:?}"
        );
        // Drain all pending warnings each push.
        while let Ok(w) = rx.try_recv() {
            warnings.push(w);
        }
    }
    assert!(
        !warnings.is_empty(),
        "queue at depth 6 of 7 (86%) MUST trigger at least one backpressure warning; \
         threshold(7) is documented as >= 80% (= ceil(5.6) = 6)"
    );
    let first_depth = warnings.first().map(|w| w.depth);
    let first_capacity = warnings.first().map(|w| w.capacity);
    assert_eq!(
        first_depth,
        Some(6),
        "first warning should fire when depth crosses threshold(7)=6"
    );
    assert_eq!(first_capacity, Some(7));

    // --- Part B: depth 5 of 7 (71%) MUST NOT trigger a warning. ---
    let (queue_at_floor, rx_floor) = BoundedActionCompletionQueue::with_backpressure_infallible(7);
    let mut warnings_floor: Vec<BackpressureWarning> = Vec::new();
    for i in 0..5u32 {
        let enqueue_result = queue_at_floor.enqueue(make_ticket(i));
        assert!(
            enqueue_result.is_ok(),
            "enqueue within capacity; got {enqueue_result:?}"
        );
        while let Ok(w) = rx_floor.try_recv() {
            warnings_floor.push(w);
        }
    }
    assert!(
        warnings_floor.is_empty(),
        "queue at depth 5 of 7 (71%) MUST NOT trigger a backpressure warning; \
         the documented threshold is >= 80% so 5 < threshold(7)={:?}, got warnings={:?}",
        first_depth,
        warnings_floor
    );

    // --- Part C: filling to depth=7 (full) still warns at depth>=6. ---
    let (queue_full, rx_full) = BoundedActionCompletionQueue::with_backpressure_infallible(7);
    let mut warnings_full: Vec<BackpressureWarning> = Vec::new();
    for i in 0..7u32 {
        let enqueue_result = queue_full.enqueue(make_ticket(i));
        assert!(
            enqueue_result.is_ok(),
            "enqueue within capacity; got {enqueue_result:?}"
        );
        while let Ok(w) = rx_full.try_recv() {
            warnings_full.push(w);
        }
    }
    assert!(
        !warnings_full.is_empty(),
        "queue at depth=7 (full) MUST trigger at least one warning"
    );
    // First warning is at depth=6 (the threshold-crossing push).
    assert_eq!(
        warnings_full.first().map(|w| w.depth),
        Some(6),
        "first warning must fire at the threshold-crossing push (depth=6)"
    );
}

// ===========================================================================
// Scenario 3 (RA-018): admit_artifact_run_with_certificate_floor with
//   required.len()=3 and caps.len()=2 must return a typed
//   AdmissionError for cardinality mismatch.
//   Author: RA018Fix
//
// Note: the literal shape "required=3, granted=2" is intercepted by the
// per-capability loop in admission.rs:740-742 (under-grant, missing the
// third required capability). The cardinality check at admission.rs:743-750
// is reachable only when granted is a SUPERSET of required. The typed
// `CapabilityCountMismatch` variant applies to the over-grant shape. We
// exercise both:
//   (a) under-grant (2 caps vs 3 required) -> CapabilityDenied on the
//       missing required capability, NOT a fabricated
//       "__capability_count_mismatch__" sentinel.
//   (b) over-grant (4 caps vs 3 required, subset) -> typed
//       CapabilityCountMismatch { required_count: 3, granted_count: 4 }.
// ===========================================================================

#[test]
fn mqa01_scenario3_ra018_capability_count_mismatch_returns_typed_error() {
    let required_a = cap("mqa.network.read", 1);
    let required_b = cap("mqa.kv.write", 2);
    let required_c = cap("mqa.fs.append", 3);

    let store = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([
            required_a.clone(),
            required_b.clone(),
            required_c.clone(),
        ])),
    };

    // --- Shape (a): under-grant (2 caps, 3 required) ---
    let granted_two =
        CapabilitySet::from_grants(Box::new([required_a.clone(), required_b.clone()]));
    let result_under = admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        test_digest(),
        granted_two,
        EventSeq::new(0),
    );
    match result_under {
        Err(AdmissionError::CapabilityDenied { .. }) => {
            // Per-capability loop fires first and reports the missing
            // required_c. This is the realistic, observable shape.
        }
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 3,
            granted_count: 2,
        }) => {
            // Also acceptable (if implementation were ever to flip the
            // ordering).
        }
        other => {
            assert!(
                matches!(
                    other,
                    Err(AdmissionError::CapabilityDenied { .. })
                        | Err(AdmissionError::CapabilityCountMismatch {
                            required_count: 3,
                            granted_count: 2,
                        })
                ),
                "under-grant (caps=2, required=3) must return CapabilityDenied or \
                 CapabilityCountMismatch {{ required_count: 3, granted_count: 2 }}, got {other:?}"
            );
        }
    }

    // --- Shape (b): over-grant (4 caps, 3 required; subset) ---
    let extra = cap("mqa.diagnostics.read", 99);
    let granted_four = CapabilitySet::from_grants(Box::new([
        required_a.clone(),
        required_b.clone(),
        required_c.clone(),
        extra,
    ]));
    let result_over = admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(2),
        test_digest(),
        granted_four,
        EventSeq::new(0),
    );
    match result_over {
        Err(AdmissionError::CapabilityCountMismatch {
            required_count,
            granted_count,
        }) => {
            assert_eq!(
                required_count, 3,
                "required_count must be exactly 3 (artifact declares 3 required capabilities)"
            );
            assert_eq!(
                granted_count, 4,
                "granted_count must be exactly 4 (caller supplied 4 capabilities)"
            );
        }
        other => {
            assert!(
                matches!(
                    other,
                    Err(AdmissionError::CapabilityCountMismatch {
                        required_count: 3,
                        granted_count: 4,
                    })
                ),
                "over-grant (caps=4 superset-of required=3) must return \
                 AdmissionError::CapabilityCountMismatch {{ required_count: 3, granted_count: 4 }}, got {other:?}"
            );
        }
    }

    // --- Shape (c): cardinality mismatch with smaller numbers (2 vs 3) ---
    let req_a = cap("mqa.role.alpha", 10);
    let req_b = cap("mqa.role.beta", 11);
    let store_small_req = FixedAcceptedStore {
        artifact: accepted_artifact_with_caps(Box::new([req_a.clone(), req_b.clone()])),
    };
    let extra_c = cap("mqa.role.gamma", 12);
    let granted_three =
        CapabilitySet::from_grants(Box::new([req_a.clone(), req_b.clone(), extra_c]));
    let result_card = admit_artifact_run_with_certificate_floor(
        &store_small_req,
        RuntimePolicy::Strict,
        RunId::new(3),
        test_digest(),
        granted_three,
        EventSeq::new(0),
    );
    match result_card {
        Err(AdmissionError::CapabilityCountMismatch {
            required_count: 2,
            granted_count: 3,
        }) => {
            // Pinned: typed cardinality-mismatch variant for the reachable
            // shape where granted is a strict superset of required.
        }
        other => {
            assert!(
                matches!(
                    other,
                    Err(AdmissionError::CapabilityCountMismatch {
                        required_count: 2,
                        granted_count: 3,
                    })
                ),
                "cardinality mismatch (caps=3 superset-of required=2) must return \
                 AdmissionError::CapabilityCountMismatch {{ required_count: 2, granted_count: 3 }}, got {other:?}"
            );
        }
    }
}
