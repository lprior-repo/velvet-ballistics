// ──────────────────────────────────────────────────────────────────────────────
// Helper: proptest strategy for generating arbitrary EvidenceBundle values
// ──────────────────────────────────────────────────────────────────────────────

use proptest::strategy::{BoxedStrategy, Strategy};

fn evidence_bundle_strategy() -> BoxedStrategy<EvidenceBundle> {
    use proptest::collection::vec;

    fn arb_executor_context() -> BoxedStrategy<ExecutorContext> {
        (any::<String>(), any::<String>(), any::<String>())
            .prop_map(|(agent, timestamp, machine)| ExecutorContext {
                agent,
                timestamp,
                machine,
            })
            .boxed()
    }

    fn arb_gate_evidence() -> BoxedStrategy<GateEvidence> {
        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            any::<i32>(),
            any::<PathBuf>(),
        )
            .prop_flat_map(|(kind, gate_name, command, exit_code, log)| {
                let status_strategy = prop_oneof![
                    Just(GateStatus::Pass),
                    Just(GateStatus::Fail),
                    any::<String>().prop_map(|reason| GateStatus::Skipped { reason }),
                ];
                status_strategy.prop_map(move |status| GateEvidence {
                    kind: kind.clone(),
                    gate_name: gate_name.clone(),
                    command: command.clone(),
                    exit_code,
                    log: log.clone(),
                    status,
                    why_failed: None,
                })
            })
            .boxed()
    }

    fn arb_source_test_mapping() -> BoxedStrategy<SourceTestMapping> {
        (any::<String>(), vec(any::<String>(), 0..=5))
            .prop_map(|(source_path, tests)| SourceTestMapping { source_path, tests })
            .boxed()
    }

    fn arb_release_artifact() -> BoxedStrategy<ReleaseGateArtifact> {
        use proptest::sample::select;

        let artifact_type_strategy = select(vec![
            ArtifactType::Benchmark,
            ArtifactType::Coverage,
            ArtifactType::Mutation,
            ArtifactType::SupplyChain,
            ArtifactType::Miri,
            ArtifactType::Clippy,
            ArtifactType::Fmt,
        ]);

        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            artifact_type_strategy,
        )
            .prop_map(|(name, path, digest, artifact_type)| ReleaseGateArtifact {
                name,
                path,
                digest,
                artifact_type,
            })
            .boxed()
    }

    (
        arb_executor_context(),
        any::<String>(),
        vec(arb_gate_evidence(), 0..=5),
        vec(arb_source_test_mapping(), 0..=5),
        vec(arb_release_artifact(), 0..=5),
    )
        .prop_map(
            |(executor_context, linked_bead_id, gates, stms, rga)| EvidenceBundle {
                schema_version: "1.0".to_string(),
                executor_context,
                linked_bead_id,
                gates,
                source_test_mappings: stms,
                release_artifacts: rga,
            },
        )
        .boxed()
}
