use super::*;
use vb_core::action_pack::{ActionPack, PackedArtifactEvidence};

pub(crate) enum ExecutorProfile<'a> {
    Legacy,
    Registered {
        catalog: &'a crate::executor::ExecutorCatalog,
        packs: &'a [ActionPack],
    },
}

impl ExecutorProfile<'_> {
    pub(super) fn check(
        &self,
        policy: RuntimePolicy,
        contracts: &[ActionContract],
    ) -> Result<(), JournalError> {
        match self {
            Self::Legacy
                if policy == RuntimePolicy::Strict
                    && contracts.iter().any(|c| c.side_effect != SideEffect::None) =>
            {
                Err(JournalError::MissingRequiredProofFlag {
                    flag: "executor_conformance",
                })
            }
            Self::Legacy => Ok(()),
            Self::Registered { catalog, packs } => catalog
                .check_packs(contracts, packs)
                .map_err(|_| JournalError::ArtifactMalformed),
        }
    }

    pub(super) fn wrap(&self, base: ArtifactVerificationEvidence) -> ArtifactEvidence {
        match self {
            Self::Legacy => ArtifactEvidence::Verified(Box::new(base)),
            Self::Registered { packs, .. } => {
                ArtifactEvidence::Packed(Box::new(PackedArtifactEvidence {
                    base,
                    packs: packs.to_vec().into_boxed_slice(),
                }))
            }
        }
    }
}
