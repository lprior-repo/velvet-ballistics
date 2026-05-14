use crate::RequiredEvidence;

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

    pub(crate) fn support_counts_for(&self, text: &str) -> (usize, usize) {
        let cited = self
            .supports
            .iter()
            .filter(|support| support.is_cited_by(text))
            .count();
        let pending = self
            .supports
            .iter()
            .filter(|support| support.is_pending_for(text))
            .count();
        (cited, pending)
    }

    pub(crate) fn supports_sentence(&self, sentence: &str) -> bool {
        self.supports
            .iter()
            .any(|support| support.matches_sentence(sentence))
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

    fn is_cited_by(&self, text: &str) -> bool {
        match &self.kind {
            EvidenceSupportKind::Cited { artifact } => {
                text.contains(&self.sentence) && text.contains(artifact)
            }
            EvidenceSupportKind::Pending => false,
        }
    }

    fn is_pending_for(&self, text: &str) -> bool {
        match self.kind {
            EvidenceSupportKind::Cited { .. } => false,
            EvidenceSupportKind::Pending => {
                text.to_ascii_lowercase().contains("pending")
                    || text.to_ascii_lowercase().contains("unverified")
            }
        }
    }

    fn matches_sentence(&self, sentence: &str) -> bool {
        self.sentence == sentence
            || match &self.kind {
                EvidenceSupportKind::Cited { artifact } => sentence.contains(artifact),
                EvidenceSupportKind::Pending => sentence.to_ascii_lowercase().contains("pending"),
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
