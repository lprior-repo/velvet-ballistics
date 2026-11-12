#![forbid(unsafe_code)]

use vb_core::frame::RunFrame;
use vb_storage::recovery::{
    MissingRunStateComponents, NonResumableRecoveryFrameSeedProduct, RecoveryCannotResumeState,
    RecoveryFrameSeed, RecoveryFrameSeedProduct, RecoveryRuntimeSummary,
    ResumableRecoveryFrameSeedProduct, UnsupportedRecoveryState,
};

use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DurableFrameRecoveryProduct {
    InvalidShape(InvalidFrameRecoveryProduct),
    CannotResume(CannotResumeRecoveryFrame),
}

impl DurableFrameRecoveryProduct {
    pub(super) fn from_seed(seed: RecoveryFrameSeed) -> Self {
        Self::from_product(RecoveryFrameSeedProduct::from_seed(seed))
    }

    pub(super) fn from_product(product: RecoveryFrameSeedProduct) -> Self {
        match ShapeCheckedRecoveryFrameProduct::parse(product) {
            ShapeCheckedParse::Valid(product) => {
                Self::CannotResume(CannotResumeRecoveryFrame::from_shape_checked(product))
            }
            ShapeCheckedParse::Invalid(invalid) => Self::InvalidShape(invalid),
        }
    }

    pub(super) fn summary(&self) -> RecoveryRuntimeSummary {
        match self {
            Self::InvalidShape(product) => product.summary(),
            Self::CannotResume(product) => product.summary(),
        }
    }

    pub(super) fn resume_status(&self) -> super::RecoveryResumeStatus {
        super::RecoveryResumeStatus::CannotResume(self.cannot_resume_state())
    }

    pub(super) fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        match self {
            Self::InvalidShape(_) => Err(RuntimeError::InvalidRecoveryHydration),
            Self::CannotResume(product) => product.hydrate_run_frame(),
        }
    }

    pub(super) fn unsupported_state(&self) -> UnsupportedRecoveryState {
        match self {
            Self::InvalidShape(product) => product.unsupported_state(),
            Self::CannotResume(product) => product.unsupported_state(),
        }
    }

    pub(super) fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        match self {
            Self::InvalidShape(product) => product.cannot_resume_state(),
            Self::CannotResume(product) => product.cannot_resume_state(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InvalidFrameRecoveryProduct {
    summary: RecoveryRuntimeSummary,
    unsupported: UnsupportedRecoveryState,
    cannot_resume: RecoveryCannotResumeState,
}

impl InvalidFrameRecoveryProduct {
    fn from_product(product: &RecoveryFrameSeedProduct) -> Self {
        let seed = product.seed();
        Self {
            summary: seed.summary,
            unsupported: seed.unsupported,
            cannot_resume: product_cannot_resume_state(product),
        }
    }

    const fn summary(&self) -> RecoveryRuntimeSummary {
        self.summary
    }

    const fn unsupported_state(&self) -> UnsupportedRecoveryState {
        self.unsupported
    }

    const fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        self.cannot_resume
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShapeCheckedRecoveryFrameProduct {
    CannotResume(NonResumableRecoveryFrameSeedProduct),
    Resumable(ResumableRecoveryFrameSeedProduct),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ShapeCheckedParse {
    Valid(ShapeCheckedRecoveryFrameProduct),
    Invalid(InvalidFrameRecoveryProduct),
}

impl ShapeCheckedRecoveryFrameProduct {
    fn parse(product: RecoveryFrameSeedProduct) -> ShapeCheckedParse {
        let invalid = InvalidFrameRecoveryProduct::from_product(&product);
        if super::frame::validate_recovery_seed_shape(product.seed()).is_err() {
            return ShapeCheckedParse::Invalid(invalid);
        }
        match product {
            RecoveryFrameSeedProduct::CannotResume(product) => {
                ShapeCheckedParse::Valid(Self::CannotResume(product))
            }
            RecoveryFrameSeedProduct::Resumable(product) => {
                ShapeCheckedParse::Valid(Self::Resumable(product))
            }
            _ => ShapeCheckedParse::Invalid(invalid),
        }
    }

    const fn seed(&self) -> &RecoveryFrameSeed {
        match self {
            Self::CannotResume(product) => product.seed(),
            Self::Resumable(product) => product.seed(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CannotResumeRecoveryFrame {
    product: ShapeCheckedRecoveryFrameProduct,
    cannot_resume: RecoveryCannotResumeState,
}

impl CannotResumeRecoveryFrame {
    fn from_shape_checked(product: ShapeCheckedRecoveryFrameProduct) -> Self {
        let cannot_resume = shape_checked_cannot_resume_state(&product);
        Self {
            product,
            cannot_resume,
        }
    }

    fn summary(&self) -> RecoveryRuntimeSummary {
        self.product.seed().summary
    }

    fn unsupported_state(&self) -> UnsupportedRecoveryState {
        self.product.seed().unsupported
    }

    const fn cannot_resume_state(&self) -> RecoveryCannotResumeState {
        self.cannot_resume
    }

    fn hydrate_run_frame(&self) -> RuntimeResult<RunFrame> {
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from(self.cannot_resume.unsupported_reason()),
        })
    }
}

const fn product_cannot_resume_state(
    product: &RecoveryFrameSeedProduct,
) -> RecoveryCannotResumeState {
    match product {
        RecoveryFrameSeedProduct::CannotResume(product) => product
            .cannot_resume_state()
            .mark_missing_components(MissingRunStateComponents::ALL),
        RecoveryFrameSeedProduct::Resumable(_) => RecoveryCannotResumeState::RESUMABLE
            .mark_missing_components(MissingRunStateComponents::ALL),
        _ => RecoveryCannotResumeState::RESUMABLE
            .mark_missing_components(MissingRunStateComponents::ALL),
    }
}

const fn shape_checked_cannot_resume_state(
    product: &ShapeCheckedRecoveryFrameProduct,
) -> RecoveryCannotResumeState {
    match product {
        ShapeCheckedRecoveryFrameProduct::CannotResume(product) => product
            .cannot_resume_state()
            .mark_missing_components(MissingRunStateComponents::ALL),
        ShapeCheckedRecoveryFrameProduct::Resumable(_) => RecoveryCannotResumeState::RESUMABLE
            .mark_missing_components(MissingRunStateComponents::ALL),
    }
}
