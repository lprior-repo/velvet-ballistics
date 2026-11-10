#![forbid(unsafe_code)]

use super::{RecoveryCannotResumeState, RecoveryFrameSeed, RecoveryRuntimeSummary};

/// Explicit recovery product. Supports summary-only recovery or a typed
/// frame-seed product recovered from durable journal events.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryHydration {
    /// Summary-only recovery product.
    Summary(RecoveryRuntimeSummary),
    /// Frame-seed recovery product classified at the storage boundary.
    FrameSeed(RecoveryFrameSeedProduct),
}

impl RecoveryHydration {
    /// Builds a summary-only recovery product.
    #[must_use]
    pub const fn from_summary(summary: RecoveryRuntimeSummary) -> Self {
        Self::Summary(summary)
    }

    /// Builds a typed frame-seed recovery product from a raw recovered seed.
    #[must_use]
    pub fn from_frame_seed(seed: RecoveryFrameSeed) -> Self {
        Self::FrameSeed(RecoveryFrameSeedProduct::from_seed(seed))
    }

    /// Returns the summary carried by this hydration product.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::Summary(summary) => *summary,
            Self::FrameSeed(product) => product.summary(),
        }
    }

    /// Returns the typed frame-seed product when this hydration carries one.
    #[must_use]
    pub const fn frame_seed_product(&self) -> Option<&RecoveryFrameSeedProduct> {
        match self {
            Self::FrameSeed(product) => Some(product),
            Self::Summary(_) => None,
        }
    }
}

/// Storage recovery product for a raw [`RecoveryFrameSeed`].
///
/// The seed remains a durable replay DTO, while this product is the typed
/// boundary claim. Callers cannot turn a raw seed into a `RecoveryHydration`
/// frame product without classifying it as `CannotResume` or `Resumable`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryFrameSeedProduct {
    /// Frame-seed evidence is present but cannot safely resume.
    CannotResume(NonResumableRecoveryFrameSeedProduct),
    /// Frame-seed evidence is sufficient for a lower-level live-frame resume.
    Resumable(ResumableRecoveryFrameSeedProduct),
}

/// Cannot-resume frame-seed product carrying the typed witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonResumableRecoveryFrameSeedProduct {
    seed: RecoveryFrameSeed,
    cannot_resume: RecoveryCannotResumeState,
}

/// Resumable frame-seed product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumableRecoveryFrameSeedProduct {
    seed: RecoveryFrameSeed,
}

impl RecoveryFrameSeedProduct {
    /// Classifies a raw recovered frame seed into a storage recovery product.
    #[must_use]
    pub fn from_seed(seed: RecoveryFrameSeed) -> Self {
        let cannot_resume = seed.cannot_resume_state();
        if cannot_resume.is_resumable() {
            Self::Resumable(ResumableRecoveryFrameSeedProduct { seed })
        } else {
            Self::CannotResume(NonResumableRecoveryFrameSeedProduct {
                seed,
                cannot_resume,
            })
        }
    }

    /// Summary carried by the underlying recovered seed.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed().summary
    }

    /// Typed cannot-resume witness computed when the product was constructed.
    /// Resumable products return [`RecoveryCannotResumeState::RESUMABLE`].
    #[must_use]
    pub const fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        match self {
            Self::CannotResume(product) => product.cannot_resume,
            Self::Resumable(_) => RecoveryCannotResumeState::RESUMABLE,
        }
    }

    /// Returns true only if the typed witness contains no cannot-resume flags.
    #[must_use]
    pub const fn is_resumable(&self) -> bool {
        matches!(self, Self::Resumable(_))
    }

    /// Borrows the raw seed for lower-level recovery code that still consumes
    /// replay DTOs directly.
    #[must_use]
    pub const fn seed(&self) -> &RecoveryFrameSeed {
        match self {
            Self::CannotResume(product) => &product.seed,
            Self::Resumable(product) => &product.seed,
        }
    }

    /// Returns the cannot-resume product when classification found missing
    /// recovery evidence.
    #[must_use]
    pub const fn cannot_resume_product(&self) -> Option<&NonResumableRecoveryFrameSeedProduct> {
        match self {
            Self::CannotResume(product) => Some(product),
            Self::Resumable(_) => None,
        }
    }
}

impl std::ops::Deref for RecoveryFrameSeedProduct {
    type Target = RecoveryFrameSeed;

    fn deref(&self) -> &Self::Target {
        self.seed()
    }
}

impl PartialEq<RecoveryFrameSeed> for RecoveryFrameSeedProduct {
    fn eq(&self, other: &RecoveryFrameSeed) -> bool {
        self.seed() == other
    }
}

impl NonResumableRecoveryFrameSeedProduct {
    /// Summary carried by the underlying recovered seed.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed.summary
    }

    /// Typed cannot-resume witness for this product.
    #[must_use]
    pub const fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        self.cannot_resume
    }

    /// Borrows the raw seed for diagnostic or compatibility paths.
    #[must_use]
    pub const fn seed(&self) -> &RecoveryFrameSeed {
        &self.seed
    }
}

impl ResumableRecoveryFrameSeedProduct {
    /// Summary carried by the underlying recovered seed.
    #[must_use]
    pub fn summary(&self) -> RecoveryRuntimeSummary {
        self.seed.summary
    }

    /// Borrows the raw seed for lower-level frame hydration.
    #[must_use]
    pub const fn seed(&self) -> &RecoveryFrameSeed {
        &self.seed
    }
}
