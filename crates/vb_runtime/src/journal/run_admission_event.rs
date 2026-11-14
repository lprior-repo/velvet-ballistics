impl StorageRuntimeJournal {
    fn admission_storage_event(
        admission: crate::admission::RunAdmission,
        seq: EventSeq,
    ) -> JournalEvent {
        match admission.budget() {
            Some(budget) => Self::budgeted_admission_storage_event(&admission, seq, budget),
            None => JournalEvent::RunAdmission {
                input_digest: admission.input_digest(),
                run: admission.run_id(),
                seq,
                artifact_digest: admission.artifact_digest(),
                policy_pin: admission.policy_pin(),
                granted_capabilities: admission.granted_capabilities().clone(),
                policy: admission.policy(),
            },
        }
    }

    fn budgeted_admission_storage_event(
        admission: &crate::admission::RunAdmission,
        seq: EventSeq,
        budget: vb_core::AggregateResourceBudget,
    ) -> JournalEvent {
        JournalEvent::RunAdmissionBudget {
            input_digest: admission.input_digest(),
            run: admission.run_id(),
            seq,
            artifact_digest: admission.artifact_digest(),
            policy_pin: admission.policy_pin(),
            granted_capabilities: admission.granted_capabilities().clone(),
            policy: admission.policy(),
            budget,
        }
    }
}
