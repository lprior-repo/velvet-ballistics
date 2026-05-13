//! vb-qi37.2.1: Runtime Admission Tests for `admit_run_with_budget`
//!
//! Integration tests for the runtime admission function that combines artifact
//! checking, capability verification, and aggregate budget capacity enforcement.
//!
//! Test plan: `.beads/vb-qi37.2.1/test-plan.md` (behaviors 30-36)

use vb_core::budget::{
    AggregateResourceBudget, AggregateResourceCapacity,
};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{
    admit_run_with_budget, AdmissionError, AlwaysPresentArtifactStore,
};

/// Helper: creates a minimal aggregate resource budget with the given dimension values.
/// Note: AggregateResourceBudget does NOT have max_active_runs - that field is in
/// AggregateResourceCapacity only.
fn make_budget(
    max_steps_executable: u32,
    max_action_tickets: u32,
    max_parallel_in_flight: u16,
    max_gather_pages: u32,
    max_gather_items: u32,
    max_result_bytes: u32,
    max_total_slots_written: u32,
    max_queue_depth: u32,
    max_journal_batch_bytes: u32,
    max_step_budget_per_tick: u64,
    max_transitions_per_tick: u64,
) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable,
        max_action_tickets,
        max_parallel_in_flight,
        max_retries_per_action: 0,
        max_gather_pages,
        max_gather_items,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes,
        max_total_slots_written,
        max_queue_depth,
        max_journal_batch_bytes,
        max_step_budget_per_tick,
        max_transitions_per_tick,
    }
}

/// Helper: creates an aggregate resource capacity with the given dimension values.
fn make_capacity(
    max_steps_executable: u64,
    max_action_tickets: u64,
    max_parallel_in_flight: u32,
    max_gather_pages: u64,
    max_gather_items: u64,
    max_result_bytes: u64,
    max_total_slots_written: u64,
    max_active_runs: u64,
    max_queue_depth: u64,
    max_journal_batch_bytes: u64,
    max_step_budget_per_tick: u64,
    max_transitions_per_tick: u64,
) -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable,
        max_action_tickets,
        max_parallel_in_flight,
        max_gather_pages,
        max_gather_items,
        max_result_bytes,
        max_total_slots_written,
        max_active_runs,
        max_queue_depth,
        max_journal_batch_bytes,
        max_step_budget_per_tick,
        max_transitions_per_tick,
    }
}

// =========================================================================
// Behavior 30: admit_run_with_budget admits when artifact exists,
//             capabilities pass, and requested budget equals available capacity.
// =========================================================================

#[test]
fn admit_run_with_budget_accepts_when_requested_equals_capacity() {
    // Given: an always-present artifact store, Relaxed policy, and no capabilities needed
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xAB; 32]);
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();

    // And: requested budget equals available capacity
    let requested = make_budget(100, 50, 8, 100, 500, 4096, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    // When: admit_run_with_budget is called
    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    // Then: admission succeeds
    assert!(result.is_ok(), "admission must succeed when requested equals capacity");
    let admission = result.unwrap();
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.policy(), policy);
    assert_eq!(admission.budget(), Some(requested));
}

// =========================================================================
// Behavior 31: admit_run_with_budget admits when requested budget is
//             below available capacity.
// =========================================================================

#[test]
fn admit_run_with_budget_accepts_when_requested_below_capacity() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xAC; 32]);
    let run_id = RunId::new(2);
    let caps = CapabilitySet::empty();

    // Requested is below available
    let requested = make_budget(50, 25, 4, 50, 250, 2048, 25, 32, 4096, 500, 500);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_ok(), "admission must succeed when requested is below capacity");
    let admission = result.unwrap();
    assert_eq!(admission.budget(), Some(requested));
}

// =========================================================================
// Behavior 32: admit_run_with_budget rejects over-capacity requests with
//             AdmissionError::ResourceCapacityExceeded
// =========================================================================

#[test]
fn admit_run_with_budget_rejects_when_action_tickets_exceed_capacity() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xAD; 32]);
    let run_id = RunId::new(3);
    let caps = CapabilitySet::empty();

    // Requested action_tickets (51) exceeds available (50)
    let requested = make_budget(100, 51, 8, 100, 500, 4096, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "admission must fail when action_tickets exceed capacity");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ResourceCapacityExceeded {
            resource,
            requested: req,
            available: av,
        } => {
            assert_eq!(resource, "max_action_tickets", "resource name must match");
            assert_eq!(req, 51, "requested must be 51");
            assert_eq!(av, 50, "available must be 50");
        }
        other => panic!("expected ResourceCapacityExceeded, got {:?}", other),
    }
}

#[test]
fn admit_run_with_budget_rejects_when_parallel_exceeds_capacity() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xAE; 32]);
    let run_id = RunId::new(4);
    let caps = CapabilitySet::empty();

    // Requested parallel (11) exceeds available (10)
    let requested = make_budget(100, 50, 11, 100, 500, 4096, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 10, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "admission must fail when parallel exceeds capacity");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ResourceCapacityExceeded { resource, .. } => {
            assert_eq!(resource, "max_parallel_in_flight");
        }
        other => panic!("expected ResourceCapacityExceeded, got {:?}", other),
    }
}

#[test]
fn admit_run_with_budget_rejects_when_gather_items_exceed_capacity() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xAF; 32]);
    let run_id = RunId::new(5);
    let caps = CapabilitySet::empty();

    // Requested gather_items (501) exceeds available (500)
    let requested = make_budget(100, 50, 8, 100, 501, 4096, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "admission must fail when gather_items exceeds capacity");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ResourceCapacityExceeded { resource, .. } => {
            assert_eq!(resource, "max_gather_items");
        }
        other => panic!("expected ResourceCapacityExceeded, got {:?}", other),
    }
}

// =========================================================================
// Behavior 33: Strict/Journaled policy rejects when artifact not found
// =========================================================================

#[test]
fn admit_run_with_budget_rejects_strict_policy_when_artifact_missing() {
    // Given: a store that reports artifact as NOT present
    struct MissingArtifactStore;
    impl vb_runtime::admission::ArtifactStore for MissingArtifactStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            false
        }
    }
    let store = MissingArtifactStore;
    let policy = RuntimePolicy::Strict;
    let digest = WorkflowDigest::from_bytes([0xB1; 32]);
    let run_id = RunId::new(7);
    let caps = CapabilitySet::empty();

    let requested = make_budget(100, 50, 8, 100, 500, 4096, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        &store,
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "Strict policy must reject when artifact is missing");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ArtifactNotFound { digest: found_digest } => {
            assert_eq!(found_digest, digest);
        }
        other => panic!("expected ArtifactNotFound, got {:?}", other),
    }
}

// =========================================================================
// Behavior 34: Relaxed policy admits without artifact check
// =========================================================================

#[test]
fn admit_run_with_budget_relaxed_policy_allows_missing_artifact() {
    // Given: a store that reports artifact as NOT present
    struct MissingArtifactStore;
    impl vb_runtime::admission::ArtifactStore for MissingArtifactStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            false
        }
    }
    let store = MissingArtifactStore;
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xB2; 32]);
    let run_id = RunId::new(8);
    let caps = CapabilitySet::empty();

    let requested = make_budget(100, 50, 8, 100, 500, 4096, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        &store,
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    // Relaxed policy must NOT check artifact existence
    assert!(result.is_ok(), "Relaxed policy must admit even when artifact is missing");
}

// =========================================================================
// Behavior 35: fits_within capacity check works for all dimensions
// =========================================================================

#[test]
fn admit_run_with_budget_rejects_when_result_bytes_exceed() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xB3; 32]);
    let run_id = RunId::new(9);
    let caps = CapabilitySet::empty();

    // Requested result_bytes (4097) exceeds available (4096)
    let requested = make_budget(100, 50, 8, 100, 500, 4097, 50, 64, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "admission must fail when result_bytes exceeds capacity");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ResourceCapacityExceeded { resource, .. } => {
            assert_eq!(resource, "max_result_bytes");
        }
        other => panic!("expected ResourceCapacityExceeded, got {:?}", other),
    }
}

// =========================================================================
// Behavior 36: queue_depth and journal_batch dimensions also checked
// =========================================================================

#[test]
fn admit_run_with_budget_rejects_when_queue_depth_exceeds() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xB4; 32]);
    let run_id = RunId::new(10);
    let caps = CapabilitySet::empty();

    // Requested queue_depth (65) exceeds available (64)
    let requested = make_budget(100, 50, 8, 100, 500, 4096, 50, 65, 8192, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "admission must fail when queue_depth exceeds capacity");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ResourceCapacityExceeded { resource, .. } => {
            assert_eq!(resource, "max_queue_depth");
        }
        other => panic!("expected ResourceCapacityExceeded, got {:?}", other),
    }
}

#[test]
fn admit_run_with_budget_rejects_when_journal_batch_exceeds() {
    let store = AlwaysPresentArtifactStore::shared_artifact();
    let policy = RuntimePolicy::Relaxed;
    let digest = WorkflowDigest::from_bytes([0xB5; 32]);
    let run_id = RunId::new(11);
    let caps = CapabilitySet::empty();

    // Requested journal_batch (8193) exceeds available (8192)
    let requested = make_budget(100, 50, 8, 100, 500, 4096, 50, 64, 8193, 1000, 1000);
    let available = make_capacity(100, 50, 8, 100, 500, 4096, 50, 1, 64, 8192, 1000, 1000);

    let result = admit_run_with_budget(
        store.as_ref(),
        policy,
        digest,
        run_id,
        caps,
        requested,
        available,
    );

    assert!(result.is_err(), "admission must fail when journal_batch exceeds capacity");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ResourceCapacityExceeded { resource, .. } => {
            assert_eq!(resource, "max_journal_batch_bytes");
        }
        other => panic!("expected ResourceCapacityExceeded, got {:?}", other),
    }
}
