//! Kani Arbitrary implementations for ID types needed by vb_storage harnesses.
//!
//! These enable kani to generate arbitrary values of ID types for harness testing.

#![forbid(unsafe_code)]

use super::{ActionId, EventSeq, RunId, SeqNo, SlotIdx, StepIdx};
use crate::WorkflowDigest;

impl kani::Arbitrary for RunId {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for SeqNo {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for EventSeq {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for StepIdx {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for SlotIdx {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for ActionId {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for WorkflowDigest {
    fn any() -> Self {
        Self::from_bytes(kani::any())
    }
}
