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
    /// Uses arbitrary bytes converted to UTF-8, filtering empty strings.
    fn arbitrary_capability_name() -> Box<str> {
        let bytes: [u8; 16] = kani::any();
        let s = String::from_utf8_lossy(&bytes);
        // Split at null and take first part to avoid trailing null issues.
        // Note: split() always yields at least one element for any &str input.
        let name = match s.split('\0').next() {
            Some(v) => v,
            None => {
                kani::assume(false);
                return;
            }
        };
        // Ensure non-empty; if empty, use default.
        if name.is_empty() {
            Box::from("cap")
        } else {
            Box::from(name)
        }
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

        let req_name: [u8; 16] = kani::any();
        let grant_name: [u8; 16] = kani::any();
        let req_name_lossy = String::from_utf8_lossy(&req_name);
        let req_name_str = match req_name_lossy.split('\0').next() {
            Some(value) => value,
            None => "cap",
        };
        let grant_name_lossy = String::from_utf8_lossy(&grant_name);
        let grant_name_str = match grant_name_lossy.split('\0').next() {
            Some(value) => value,
            None => "cap",
        };

        let required = Capability::new(req_name_str.into(), req_action_id);
        let grant = Capability::new(grant_name_str.into(), grant_action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(req_action_id, &required, &granted);

        match result {
            Ok(()) => {}
            Err(AdmissionError::CapabilityDenied { .. }) => {}
            Err(_) => {
                kani::assert(false, "Only CapabilityDenied expected for denied cases");
            }
        }
    }

    #[kani::proof]
    fn check_capability_grants_exact_match() {
        let action_id = ActionId::new(7);
        // GOD RULE fix: use arbitrary capability name instead of hardcoded "action".
        let name = arbitrary_capability_name();
        let required = Capability::new(name.clone(), action_id);
        let exact = CapabilitySet::from_grants(Box::new([Capability::new(name, action_id)]));

        kani::assert(
            check_capability(action_id, &required, &exact).is_ok(),
            "exact grant is accepted",
        );
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
    }

    #[kani::proof]
    fn check_capability_action_match_name_denies() {
        let action_id = ActionId::new(1);
        // GOD RULE fix: use arbitrary capability names instead of hardcoded "secrets"/"network".
        let req_name = arbitrary_capability_name();
        let grant_name = arbitrary_capability_name();
        // Ensure names differ so the denial case is triggered.
        kani::assume(&*req_name != &*grant_name);
        let required = Capability::new(req_name, action_id);
        let grant = Capability::new(grant_name, action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "action match + name denies -> CapabilityDenied",
        );
        std::mem::forget(result);
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
    }

    #[kani::proof]
    fn check_capability_hierarchical_rejects_subpath() {
        let action_id = ActionId::new(1);
        // GOD RULE fix: use arbitrary hierarchical capability names instead of hardcoded "network.api"/"network".
        // Generate two arbitrary names where req_name is a subpath of grant_name.
        let req_name = arbitrary_capability_name();
        // Construct grant_name as req_name + ".api" suffix for hierarchical test.
        let grant_name = format!("{}.api", &*req_name);
        let required = Capability::new(req_name, action_id);
        let grant = Capability::new(Box::from(grant_name), action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "prefix grant must not satisfy subpath requirement",
        );
        std::mem::forget(result);
    }

    #[kani::proof]
    fn check_capability_partial_segment_rejected() {
        let action_id = ActionId::new(1);
        // GOD RULE fix: use arbitrary capability names instead of hardcoded "network"/"net".
        // Generate req_name and grant_name where grant_name is a prefix of req_name.
        let req_name = arbitrary_capability_name();
        // Construct grant_name as first 3 chars of req_name (if available) to test partial segment.
        let grant_name: Box<str> = if req_name.len() >= 3 {
            Box::from(&req_name[..3])
        } else {
            req_name.clone()
        };
        // Ensure grant_name != req_name to trigger the partial segment rejection.
        kani::assume(&*grant_name != &*req_name);
        let required = Capability::new(req_name, action_id);
        let grant = Capability::new(grant_name, action_id);
        let granted = CapabilitySet::from_grants(Box::new([grant]));

        let result = check_capability(action_id, &required, &granted);
        kani::assert(
            matches!(&result, Err(AdmissionError::CapabilityDenied { .. })),
            "partial segment must not grant",
        );
        std::mem::forget(result);
    }
}
