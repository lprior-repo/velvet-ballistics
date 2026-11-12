// -------------------------------------------------------------------------
// admit_run_with_budget tests
// -------------------------------------------------------------------------

#[test]
fn admit_run_with_budget_returns_resource_capacity_exceeded() {
    /// A store that always says artifact exists.
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let store = AlwaysPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    // Request more steps than the capacity allows, while staying inside policy.
    let requested = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: 0,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_open_gates: u64::MAX,
        max_signal_subscriptions: u64::MAX,
        max_incident_emissions: u64::MAX,
        max_transitions: u64::MAX,
        max_journal_bytes_total: u64::MAX,
        max_value_store_entries: u64::MAX,
        max_cost_units: u64::MAX,
        tenant_quota_cost_units: u64::MAX,
        global_quota_cost_units: u64::MAX,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps,
        requested,
        capacity,
    );

    assert!(matches!(
        result,
        Err(AdmissionError::ResourceCapacityExceeded { .. })
    ));
}

#[test]
fn admit_run_with_budget_policy_rejects_over_policy_before_capacity() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let requested = AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 2_001,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_open_gates: u64::MAX,
        max_signal_subscriptions: u64::MAX,
        max_incident_emissions: u64::MAX,
        max_transitions: u64::MAX,
        max_journal_bytes_total: u64::MAX,
        max_value_store_entries: u64::MAX,
        max_cost_units: u64::MAX,
        tenant_quota_cost_units: u64::MAX,
        global_quota_cost_units: u64::MAX,
    };
    let budget_policy = BoundednessPolicy {
        absolute_max_result_bytes: 2_000,
        ..BoundednessPolicy::DEFAULT
    };

    let result = admit_run_with_budget_policy(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(77),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested,
            available: capacity,
            policy: budget_policy,
        },
    );

    assert_eq!(
        result,
        Err(AdmissionError::BudgetPolicyExceeded {
            resource: "max_result_bytes",
            actual: 2_001,
            limit: 2_000,
        })
    );
}

#[test]
fn admit_run_with_budget_rejects_strict_without_artifact() {
    /// A store that always says artifact does NOT exist.
    struct NeverPresentStore;
    impl ArtifactStore for NeverPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            false
        }
    }
    let store = NeverPresentStore;
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let requested = AggregateResourceBudget {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
        max_open_gates: 0,
        max_signal_subscriptions: 0,
        max_incident_emissions: 0,
        max_transitions: 0,
        max_journal_bytes_total: 0,
        max_value_store_entries: 0,
        max_cost_units: 0,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_open_gates: u64::MAX,
        max_signal_subscriptions: u64::MAX,
        max_incident_emissions: u64::MAX,
        max_transitions: u64::MAX,
        max_journal_bytes_total: u64::MAX,
        max_value_store_entries: u64::MAX,
        max_cost_units: u64::MAX,
        tenant_quota_cost_units: u64::MAX,
        global_quota_cost_units: u64::MAX,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps,
        requested,
        capacity,
    );

    assert_eq!(result, Err(AdmissionError::ArtifactNotFound { digest }));
}

// ══ vb-h7rex: Policy pin constructor and serialization tests ═════════════
#[test]
fn admission_legacy_constructor_produces_explicit_unverified_pin() {
    // RA-019 regression: compatibility constructor must produce explicit
    // RunPolicyPin::Unverified, never a silent new authority.
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Relaxed);
    assert_eq!(admission.policy_pin(), RunPolicyPin::Unverified);
}

#[test]
fn admission_with_policy_pin_builder_sets_actual_pin() {
    // RA-021: verified artifact admission must set the actual policy identity,
    // not the legacy Unverified default.
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let policy_digest = WorkflowDigest::from_bytes([0xCC; 32]);
    let admission = RunAdmission::new(digest, run_id, caps, RuntimePolicy::Strict)
        .with_policy_pin(RunPolicyPin::Pinned(policy_digest));
    assert_eq!(admission.policy_pin(), RunPolicyPin::Pinned(policy_digest));
}

#[test]
fn admission_strict_journaled_admit_artifact_run_sets_pinned_policy_pin() -> Result<(), String> {
    // RA-017 / RA-020 regression: full strict and journaled admission of an
    // accepted artifact must record the artifact's actual policy_digest as the
    // RunPolicyPin, not Unverified or a caller-provided value.
    use vb_storage::admission::compute_policy_digest;
    let action = ActionId::new(1);
    let required = Capability::new("network".into(), action);
    let caps = CapabilitySet::from_grants(Box::new([required]));
    let required_for_artifact = Capability::new("network".into(), action);
    let artifact_caps = Box::new([required_for_artifact]);
    let mut artifact = accepted_artifact_with_caps(artifact_caps)?;
    let workflow_parts = build_workflow_with_single_action()?;
    let computed_policy = compute_policy_digest(&workflow_parts)
        .map_err(|e| format!("compute_policy_digest failed: {e}"))?;
    artifact.policy_digest = computed_policy;
    let store = FixedAcceptedStore { artifact };
    let run_id = RunId::new(1);
    // The artifact returned by the store has its own digest; the admission
    // gate validates that the loaded artifact's digest matches the request.
    // Use the actual artifact digest for the request to pass validation.
    let artifact_digest = store.artifact.digest;
    let result = crate::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        run_id,
        artifact_digest,
        caps,
    );
    let admission = result.map_err(|e| format!("admit_artifact_run failed: {e}"))?;
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    assert_eq!(
        admission.policy_pin(),
        RunPolicyPin::Pinned(computed_policy)
    );
    assert_ne!(admission.policy_pin(), RunPolicyPin::Unverified);
    Ok(())
}

#[test]
fn admission_relaxed_admit_run_keeps_unverified_pin() {
    // RA-019: relaxed admission without artifact validation must remain
    // explicitly Unverified, even when the caller knows the policy digest.
    let digest = test_digest();
    let run_id = RunId::new(1);
    let caps = CapabilitySet::empty();
    let result = crate::admission::admit_run(
        &AlwaysPresentArtifactStore,
        RuntimePolicy::Relaxed,
        digest,
        run_id,
        caps.clone(),
    );
    assert_eq!(
        result,
        Ok(RunAdmission::new(
            digest,
            run_id,
            caps,
            RuntimePolicy::Relaxed
        ))
    );
}

// ══ vb-h7rex: Checkpoint policy pin roundtrip tests ═════════════════════
#[test]
fn admission_record_preserves_policy_pin_through_roundtrip() -> Result<(), String> {
    // RA-022: new checkpoints with pinned policy identity must survive
    // serialize → deserialize without silent degradation to Unverified.
    let run_id = RunId::new(1);
    let digest = test_digest();
    let policy_digest = WorkflowDigest::from_bytes([0xDD; 32]);
    let admission = RunAdmission::new(
        digest,
        run_id,
        CapabilitySet::empty(),
        RuntimePolicy::Strict,
    )
    .with_policy_pin(RunPolicyPin::Pinned(policy_digest));
    let bytes = postcard::to_allocvec(&admission).map_err(|e| e.to_string())?;
    let restored: RunAdmission = postcard::from_bytes(&bytes).map_err(|e| e.to_string())?;
    assert_eq!(restored.policy_pin(), RunPolicyPin::Pinned(policy_digest));
    Ok(())
}

// ══ vb-hbav B14: AdmissionError exhaustiveness compile-time check ════════
#[test]
fn admission_error_match_covers_all_variants() {
    fn _exhaustive_match(e: &AdmissionError) -> &'static str {
        match e {
            AdmissionError::ArtifactNotFound { .. } => "artifact_not_found",
            AdmissionError::CapabilityDenied { .. } => "capability_denied",
            AdmissionError::ResourceCapacityExceeded { .. } => "resource_capacity_exceeded",
            AdmissionError::BudgetPolicyExceeded { .. } => "budget_policy_exceeded",
            AdmissionError::ResourceBudgetOverflow { .. } => "resource_budget_overflow",
            AdmissionError::ResourceBudgetUnderflow { .. } => "resource_budget_underflow",
            AdmissionError::ResourceBudgetInvalidCapacity { .. } => {
                "resource_budget_invalid_capacity"
            }
            AdmissionError::ResourceStepCeilingExceeded { .. } => "resource_step_ceiling_exceeded",
            AdmissionError::ResourcePerTickCeilingExceeded { .. } => {
                "resource_per_tick_ceiling_exceeded"
            }
            AdmissionError::ArtifactEnvelopeDecodeFailed => "artifact_envelope_decode_failed",
            AdmissionError::ArtifactInvalidGateCount { .. } => "artifact_invalid_gate_count",
            AdmissionError::ArtifactInvalidProofFlag { .. } => "artifact_invalid_proof_flag",
            AdmissionError::ArtifactDigestMismatch { .. } => "artifact_digest_mismatch",
            AdmissionError::ArtifactCertificateStale { .. } => "artifact_certificate_stale",
            AdmissionError::ArtifactVersionIncompatible { .. } => "artifact_version_incompatible",
            AdmissionError::CapabilityCountMismatch { .. } => "capability_count_mismatch",
        }
    }
    let _ = _exhaustive_match;
}
// §77.18 tenant quota, global quota, and remaining-budget admission tests
// -------------------------------------------------------------------------

fn ample_capacity_77_18() -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: u64::MAX,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_timer_entries: u64::MAX,
        max_trace_events: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_ipc_payload_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        max_input_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_open_gates: u64::MAX,
        max_signal_subscriptions: u64::MAX,
        max_incident_emissions: u64::MAX,
        max_transitions: u64::MAX,
        max_journal_bytes_total: u64::MAX,
        max_value_store_entries: u64::MAX,
        max_cost_units: u64::MAX,
        tenant_quota_cost_units: u64::MAX,
        global_quota_cost_units: u64::MAX,
    }
}

fn requested_budget_77_18(cost_units: u64, open_gates: u32) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 1,
        max_parallel_in_flight: 1,
        max_retries_per_action: 1,
        max_gather_pages: 1,
        max_gather_items: 1,
        max_for_each_iterations: 1,
        max_together_branches: 1,
        max_repeat_attempts: 1,
        max_run_time_seconds: 10,
        max_result_bytes: 1,
        max_total_slots_written: 1,
        max_timer_entries: 1,
        max_trace_events: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_blob_bytes: 1,
        max_input_bytes: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_open_gates: open_gates,
        max_signal_subscriptions: 1,
        max_incident_emissions: 1,
        max_transitions: 1,
        max_journal_bytes_total: 1,
        max_value_store_entries: 1,
        max_cost_units: cost_units,
    }
}

#[test]
fn admit_run_with_budget_and_tenant_quota_rejects_over_quota() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let result = admit_run_with_budget_and_tenant_quota(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(1),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 1),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        5,
        10,
    );
    assert_eq!(
        result,
        Err(AdmissionError::ResourceCapacityExceeded {
            resource: "tenant_quota_cost_units",
            requested: 15,
            available: 10,
        })
    );
}

#[test]
fn admit_run_with_budget_and_tenant_quota_admits_at_quota_and_exposes_remaining() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let admission = admit_run_with_budget_and_tenant_quota(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(2),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 2),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        0,
        10,
    )
    .expect("at-quota admission must succeed");
    let mut remaining = admission
        .remaining_budget()
        .expect("budget admission carries remaining counters");
    assert_eq!(remaining.remaining_open_gates(), 2);
    assert!(remaining.try_open_gate().is_ok());
    assert_eq!(remaining.remaining_open_gates(), 1);
    assert_eq!(remaining.remaining_cost_units(), 10);
}

#[test]
fn admit_run_with_budget_and_tenant_quota_from_snapshot_rejects_over_quota() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let mut usage = ShardUsageSnapshot::default();
    usage
        .record_tenant_cost(TenantId::new(7), 5)
        .expect("snapshot record fits");
    let result = admit_run_with_budget_and_tenant_quota_from_snapshot(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(3),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 1),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        TenantId::new(7),
        &usage,
        10,
    );
    assert_eq!(
        result,
        Err(AdmissionError::ResourceCapacityExceeded {
            resource: "tenant_quota_cost_units",
            requested: 15,
            available: 10,
        })
    );
}

#[test]
fn admit_run_with_budget_and_tenant_quota_from_snapshot_records_tenant() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let mut usage = ShardUsageSnapshot::default();
    usage
        .record_tenant_cost(TenantId::new(8), 100)
        .expect("other tenant record fits");
    let admission = admit_run_with_budget_and_tenant_quota_from_snapshot(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(4),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 1),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        TenantId::new(7),
        &usage,
        10,
    )
    .expect("other tenant usage must not count against this tenant");
    assert_eq!(admission.tenant(), Some(TenantId::new(7)));
    let legacy = admit_run_with_budget_and_tenant_quota(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(5),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 1),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        0,
        10,
    )
    .expect("explicit-sum path still admits");
    assert_eq!(legacy.tenant(), None);
}

#[test]
fn admit_run_with_budget_rejects_over_global_quota() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let mut capacity = ample_capacity_77_18();
    capacity.global_quota_cost_units = 0;
    let result = admit_run_with_budget(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(3),
        CapabilitySet::empty(),
        requested_budget_77_18(10, 1),
        capacity,
    );
    assert_eq!(
        result,
        Err(AdmissionError::ResourceCapacityExceeded {
            resource: "global_quota_cost_units",
            requested: 10,
            available: 0,
        })
    );
}

#[test]
fn admit_run_with_budget_and_global_quota_rejects_summed_over_quota() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let result = admit_run_with_budget_and_global_quota(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(4),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 1),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        5,
        10,
    );
    assert_eq!(
        result,
        Err(AdmissionError::ResourceCapacityExceeded {
            resource: "global_quota_cost_units",
            requested: 15,
            available: 10,
        })
    );
}

#[test]
fn admit_run_with_budget_and_global_quota_admits_at_quota_and_exposes_remaining() {
    struct AlwaysPresentStore;
    impl ArtifactStore for AlwaysPresentStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }
    let admission = admit_run_with_budget_and_global_quota(
        &AlwaysPresentStore,
        RuntimePolicy::Strict,
        test_digest(),
        RunId::new(5),
        CapabilitySet::empty(),
        AdmissionBudgetRequest {
            requested: requested_budget_77_18(10, 2),
            available: ample_capacity_77_18(),
            policy: BoundednessPolicy::DEFAULT,
        },
        0,
        10,
    )
    .expect("at-quota admission must succeed");
    let mut remaining = admission
        .remaining_budget()
        .expect("budget admission carries remaining counters");
    assert_eq!(remaining.remaining_open_gates(), 2);
    assert!(remaining.try_open_gate().is_ok());
    assert_eq!(remaining.remaining_open_gates(), 1);
    assert_eq!(remaining.remaining_cost_units(), 10);
}
