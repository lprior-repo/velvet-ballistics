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

        pub fn cited_count_for(&self, sentence: &str) -> usize {
            self.supports
                .iter()
                .filter(|support| {
                    support.sentence == sentence
                        && matches!(support.kind, EvidenceSupportKind::Cited { .. })
                })
                .count()
        }

        pub fn pending_count_for(&self, sentence: &str) -> usize {
            self.supports
                .iter()
                .filter(|support| {
                    support.sentence == sentence
                        && matches!(support.kind, EvidenceSupportKind::Pending)
                })
                .count()
        }

        pub fn cited_count_in_text(&self, text: &str) -> usize {
            self.supports
                .iter()
                .filter(|support| match &support.kind {
                    EvidenceSupportKind::Cited { artifact } => {
                        text.contains(&support.sentence) && text.contains(artifact)
                    }
                    EvidenceSupportKind::Pending => false,
                })
                .count()
        }

        pub fn pending_count_in_text(&self, text: &str) -> usize {
            self.supports
                .iter()
                .filter(|support| {
                    matches!(support.kind, EvidenceSupportKind::Pending)
                        && pending_support_matches_text(&support.sentence, text)
                })
                .count()
        }

        pub fn has_support_for_claim(&self, claim: &str, text: &str) -> bool {
            self.supports.iter().any(|support| match &support.kind {
                EvidenceSupportKind::Cited { artifact } => {
                    support.sentence == claim && text.contains(claim) && text.contains(artifact)
                }
                EvidenceSupportKind::Pending => {
                    support.sentence == claim && pending_support_matches_text(claim, text)
                }
            })
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

    fn pending_support_matches_text(sentence: &str, text: &str) -> bool {
        if text.contains(sentence) {
            return true;
        }

        let text_lower = text.to_ascii_lowercase();
        let sentence_lower = sentence.to_ascii_lowercase();
        if text_lower.contains(&sentence_lower) {
            return true;
        }
        if !has_pending_marker(&text_lower) {
            return false;
        }

        let mut saw_word = false;
        for word in sentence_lower
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-')
            .filter(|word| !word.is_empty())
        {
            saw_word = true;
            if !text_lower.contains(word) {
                return false;
            }
        }
        saw_word
    }

    fn has_pending_marker(text: &str) -> bool {
        text.contains("pending") || text.contains("unverified")
    }

    pub fn required_evidence() -> RequiredEvidence {
        RequiredEvidence::ConcreteArtifactOrPendingMarker
    }
}
