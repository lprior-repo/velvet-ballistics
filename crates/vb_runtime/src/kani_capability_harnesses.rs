// Kani harness for KAN-CHECK-CAP-001: check_capability action match/mismatch and name grant/deny
// Verifies no UB, no panic, and Ok or Err(CapabilityDenied) for all combinations

#![forbid(unsafe_code)]

use crate::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    admit_artifact_run, check_capability,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;

#[cfg(kani)]
mod kani_capability_harnesses {
    use super::*;

    /// Generate an arbitrary Box<str> for capability names.
    /// Uses a bounded symbolic corpus so Kani proves equality, mismatch,
    /// hierarchical, and partial-prefix cases without modeling UTF-8 decoding.
    fn arbitrary_capability_name() -> Box<str> {
        capability_name_from_selector(kani::any())
    }

    fn capability_name_from_selector(selector: u8) -> Box<str> {
        match selector % 4 {
            0 => Box::from("network"),
            1 => Box::from("network.api"),
            2 => Box::from("secrets"),
            _ => Box::from("storage"),
        }
    }

    fn partial_segment_pair(selector: u8) -> (Box<str>, Box<str>) {
        match selector % 3 {
            0 => (Box::from("network"), Box::from("net")),
            1 => (Box::from("storage"), Box::from("stor")),
            _ => (Box::from("secrets"), Box::from("sec")),
        }
    }

    fn distinct_capability_name_pair(selector: u8) -> (Box<str>, Box<str>) {
        match selector % 4 {
            0 => (Box::from("network"), Box::from("secrets")),
            1 => (Box::from("network.api"), Box::from("network")),
            2 => (Box::from("secrets"), Box::from("storage")),
            _ => (Box::from("storage"), Box::from("network.api")),
        }
    }

    fn forget_capability_check_inputs(required: Capability, granted: CapabilitySet) {
        std::mem::forget(required);
        std::mem::forget(granted);
    }

    struct AdmissionCaseStore {
        case: u8,
        digest: WorkflowDigest,
    }

    struct MissingArtifactStore;

    impl AcceptedArtifactStore for MissingArtifactStore {
        fn load_accepted_artifact(
            &self,
            artifact_digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::ArtifactNotFound {
                digest: artifact_digest,
            })
        }
    }

    struct MalformedArtifactStore;

    impl AcceptedArtifactStore for MalformedArtifactStore {
        fn load_accepted_artifact(
            &self,
            _artifact_digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::PostcardDecodeFailed)
        }
    }

    struct InvalidGateCountStore;

    impl AcceptedArtifactStore for InvalidGateCountStore {
        fn load_accepted_artifact(
            &self,
            _artifact_digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::InvalidGateCount {
                found: 0,
                required: REQUIRED_GATE_COUNT,
            })
        }
    }

    struct InvalidProofFlagStore;

    impl AcceptedArtifactStore for InvalidProofFlagStore {
        fn load_accepted_artifact(
            &self,
            _artifact_digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::MissingRequiredProofFlagBounded)
        }
    }

    impl AcceptedArtifactStore for AdmissionCaseStore {
        fn load_accepted_artifact(
            &self,
            artifact_digest: WorkflowDigest,
        ) -> Result<vb_storage::admission::AcceptedArtifact, ArtifactEnvelopeError> {
            match self.case {
                0 => Err(ArtifactEnvelopeError::ArtifactNotFound {
                    digest: artifact_digest,
                }),
                1 => Err(ArtifactEnvelopeError::PostcardDecodeFailed),
                2 => Err(ArtifactEnvelopeError::InvalidGateCount {
                    found: 0,
                    required: REQUIRED_GATE_COUNT,
                }),
                3 => Err(ArtifactEnvelopeError::MissingRequiredProofFlagBounded),
                4 => Ok(accepted_artifact(
                    self.digest,
                    Box::new([Capability::new(
                        arbitrary_capability_name(),
                        ActionId::new(7),
                    )]),
                )),
                _ => Ok(accepted_artifact(self.digest, Box::new([]))),
            }
        }
    }

    fn accepted_artifact(
        digest: WorkflowDigest,
        required_capabilities: Box<[Capability]>,
    ) -> vb_storage::admission::AcceptedArtifact {
        vb_storage::admission::AcceptedArtifact {
            digest,
            source_digest: digest,
            policy_digest: digest,
            ir: Vec::new(),
            verification: vb_storage::admission::VerificationProof {
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
            accepted_at_seq: vb_storage::EventSeq::new(0),
            required_capabilities,
        }
    }

    fn strict_admission_with_store(
        store: &dyn AcceptedArtifactStore,
    ) -> Result<crate::admission::RunAdmission, AdmissionError> {
        let digest = WorkflowDigest::from_bytes([0xA5; 32]);
        admit_artifact_run(
            store,
            RuntimePolicy::Strict,
            RunId::new(1),
            digest,
            CapabilitySet::empty(),
        )
    }

    #[kani::proof]
    fn strict_admission_invalid_artifact_cases_reject() {
        let missing = strict_admission_with_store(&MissingArtifactStore);
        kani::assert(
            matches!(missing, Err(AdmissionError::ArtifactNotFound { .. })),
            "missing artifact rejects strict admission",
        );

        let malformed = strict_admission_with_store(&MalformedArtifactStore);
        kani::assert(
            matches!(malformed, Err(AdmissionError::ArtifactEnvelopeDecodeFailed)),
            "malformed artifact decode rejects strict admission",
        );

        let gate_count = strict_admission_with_store(&InvalidGateCountStore);
        kani::assert(
            matches!(
                gate_count,
                Err(AdmissionError::ArtifactInvalidGateCount { .. })
            ),
            "invalid gate count rejects strict admission",
        );

        let proof_flag = strict_admission_with_store(&InvalidProofFlagStore);
        kani::assert(
            matches!(
                proof_flag,
                Err(AdmissionError::ArtifactInvalidProofFlag { .. })
            ),
            "invalid proof flag rejects strict admission",
        );
    }

    #[kani::proof]
    fn strict_admission_invalid_capability_rejects() {
        let digest = WorkflowDigest::from_bytes([0xA5; 32]);
        let store = AdmissionCaseStore { case: 4, digest };
        let capability = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(1),
            digest,
            CapabilitySet::empty(),
        );
        kani::assert(
            matches!(capability, Err(AdmissionError::CapabilityDenied { .. })),
            "invalid capability grant rejects strict admission",
        );
        std::mem::forget(capability);
    }

    #[kani::proof]
    fn strict_admission_digest_mismatch_rejects_required_blocker() {
        let requested_digest = WorkflowDigest::from_bytes([0x11; 32]);
        let stored_digest = WorkflowDigest::from_bytes([0x22; 32]);
        let store = AdmissionCaseStore {
            case: 5,
            digest: stored_digest,
        };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(3),
            requested_digest,
            CapabilitySet::empty(),
        );

        kani::assert(
            result.is_err(),
            "digest mismatch must reject before admission",
        );
    }

    #[kani::proof]
    fn strict_legacy_presence_only_bypass_rejects_required_blocker() {
        let digest = WorkflowDigest::from_bytes([0x33; 32]);
        let store = MissingArtifactStore;
        let result = crate::admission::admit_run(
            &store,
            RuntimePolicy::Strict,
            digest,
            RunId::new(4),
            CapabilitySet::empty(),
        );

        kani::assert(
            result.is_err(),
            "strict presence-only bypass must reject before admission",
        );
    }

    #[kani::proof]
    fn strict_admission_valid_artifact_admits() {
        let digest = WorkflowDigest::from_bytes([0x5A; 32]);
        let store = AdmissionCaseStore { case: 5, digest };
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(2),
            digest,
            CapabilitySet::empty(),
        );

        kani::assert(result.is_ok(), "valid strict accepted artifact admits");
    }

    #[kani::proof]
    fn check_capability_harness() {
        let req_action: u16 = kani::any();
        let grant_action: u16 = kani::any();
        let req_action_id = ActionId::new(req_action);
        let grant_action_id = ActionId::new(grant_action);

        let required = Capability::new(arbitrary_capability_name(), req_action_id);
        let grant = Capability::new(arbitrary_capability_name(), grant_action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(req_action_id, &required, &granted);

        match &result {
            Ok(()) => {}
            Err(AdmissionError::CapabilityDenied { .. }) => {}
            Err(_) => {
                kani::assert(false, "Only CapabilityDenied expected for denied cases");
            }
        }
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }

    #[kani::proof]
    fn check_capability_grants_exact_match() {
        let action_id = ActionId::new(7);
        // GOD RULE fix: use arbitrary capability name instead of hardcoded "action".
        let name = arbitrary_capability_name();
        let required = Capability::new(name.clone(), action_id);
        let exact = CapabilitySet::from_grants(Box::new([Capability::new(name, action_id)]));

        let result = check_capability(action_id, &required, &exact);
        kani::assert(result.is_ok(), "exact grant is accepted");
        std::mem::forget(result);
        forget_capability_check_inputs(required, exact);
    }

    #[kani::proof]
    fn check_capability_action_match_name_grants() {
        let action_id = ActionId::new(1);
        // GOD RULE fix: use arbitrary capability name instead of hardcoded "network".
        let name = arbitrary_capability_name();
        let required = Capability::new(name.clone(), action_id);
        let grant = Capability::new(name, action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(result.is_ok(), "action match + name grants → Ok");
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }

    #[kani::proof]
    fn check_capability_action_match_name_denies() {
        let action_id = ActionId::new(1);
        let (req_name, grant_name) = distinct_capability_name_pair(kani::any());
        let required = Capability::new(req_name, action_id);
        let grant = Capability::new(grant_name, action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action match + name denies -> CapabilityDenied",
        );
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }

    #[kani::proof]
    fn check_capability_action_mismatch_name_grants() {
        let action_id = ActionId::new(1);
        // GOD RULE fix: use arbitrary capability name instead of hardcoded "network".
        let name = arbitrary_capability_name();
        let required = Capability::new(name.clone(), action_id);
        let grant = Capability::new(name, ActionId::new(99));
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action mismatch -> CapabilityDenied regardless of name",
        );
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }

    #[kani::proof]
    fn check_capability_action_mismatch_name_denies() {
        let action_id = ActionId::new(1);
        // GOD RULE fix: use arbitrary capability names instead of hardcoded "secrets"/"network".
        let req_name = arbitrary_capability_name();
        let grant_name = arbitrary_capability_name();
        let required = Capability::new(req_name, action_id);
        let grant = Capability::new(grant_name, ActionId::new(99));
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action mismatch + name denies -> CapabilityDenied",
        );
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }

    #[kani::proof]
    fn check_capability_hierarchical_rejects_subpath() {
        let action_id = ActionId::new(1);
        let required = Capability::new(Box::from("network"), action_id);
        let grant = Capability::new(Box::from("network.api"), action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "prefix grant must not satisfy subpath requirement",
        );
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }

    #[kani::proof]
    fn check_capability_partial_segment_rejected() {
        let action_id = ActionId::new(1);
        let (req_name, grant_name) = partial_segment_pair(kani::any());
        let required = Capability::new(req_name, action_id);
        let grant = Capability::new(grant_name, action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "partial segment must not grant",
        );
        std::mem::forget(result);
        forget_capability_check_inputs(required, granted);
    }
}
