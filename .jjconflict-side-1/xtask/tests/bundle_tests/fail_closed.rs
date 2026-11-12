/// OBL-006: Fail-closed validation — empty required fields trigger rejection.
#[test]
fn prop_fail_closed_missing_bead_id() {
    use proptest::prelude::*;

    proptest!(
        |(agent in any::<String>(),
          timestamp in any::<String>(),
          machine in any::<String>(),
          major in 2u64..,
          minor in any::<String>())| {
            let bundle = EvidenceBundle {
                schema_version: format!("{}.{}", major, minor),
                executor_context: ExecutorContext {
                    agent,
                    timestamp,
                    machine,
                },
                linked_bead_id: String::new(),
                gates: vec![],
                source_test_mappings: vec![],
                release_artifacts: vec![],
            };

            let errors = validate_bundle(&bundle);
            assert!(
                !errors.is_empty(),
                "validate_bundle must reject empty linked_bead_id"
            );
            assert!(
                errors.iter().any(|e| {
                    matches!(
                        e,
                        xtask::evidence::Error::MissingRequiredField {
                            field
                        } if field == "linked_bead_id"
                    )
                }),
                "must produce MissingRequiredField for linked_bead_id"
            );
        }
    );
}

#[test]
fn prop_fail_closed_missing_agent() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.agent = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty agent"
        );
    });
}

#[test]
fn prop_fail_closed_missing_timestamp() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.timestamp = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty timestamp"
        );
    });
}

#[test]
fn prop_fail_closed_missing_machine() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.machine = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty machine"
        );
    });
}

