// Stub for vb_doc evidence module — functionality deferred to implementation phase.
pub mod evidence {
    use crate::doc_reconcile::RequiredEvidence;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EvidenceIndex {
        supports: Vec<EvidenceSupport>,
    }

    impl EvidenceIndex {
        pub fn empty() -> Self {
            Self {
                supports: Vec::new(),
            }
        }

        pub fn from_supports(supports: Vec<EvidenceSupport>) -> Self {
            Self { supports }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EvidenceSupport {
        sentence: String,
        kind: EvidenceSupportKind,
    }

    impl EvidenceSupport {
        pub fn cited(sentence: &str, artifact: &str) -> Self {
            Self {
                sentence: sentence.to_owned(),
                kind: EvidenceSupportKind::Cited {
                    artifact: artifact.to_owned(),
                },
            }
        }

        pub fn pending(sentence: &str) -> Self {
            Self {
                sentence: sentence.to_owned(),
                kind: EvidenceSupportKind::Pending,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum EvidenceSupportKind {
        Cited { artifact: String },
        Pending,
    }

    pub(crate) fn required_evidence() -> RequiredEvidence {
        RequiredEvidence::ConcreteArtifactOrPendingMarker
    }
}
