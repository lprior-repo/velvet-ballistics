#![forbid(unsafe_code)]
//! Compiled state-machine node routing planes.

use super::CompiledNodeKind;
use crate::ids::{SlotIdx, StepIdx};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One compiled state-machine node.
///
/// Data routing (`output`) and control routing (`routing`) are separate
/// planes. Field order is frozen: it defines the canonical postcard
/// artifact bytes, so fields are never reordered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledNode {
    /// Step index of this node.
    pub id: StepIdx,
    /// Where this node writes its result.
    pub output: NodeOutput,
    /// Control and error routing for this node.
    pub routing: NodeRouting,
    /// Node behavior.
    pub kind: CompiledNodeKind,
}

/// Where a node writes its result, when it writes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeOutput {
    /// The node writes nothing.
    Discard,
    /// The node writes its result into this slot.
    Slot(SlotIdx),
}

impl NodeOutput {
    /// Output slot, when this node writes one.
    #[must_use]
    pub const fn slot(self) -> Option<SlotIdx> {
        match self {
            Self::Discard => None,
            Self::Slot(slot) => Some(slot),
        }
    }
}

/// Control-flow routing for a node: fallthrough plus failure handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRouting {
    /// Fallthrough step after this node completes.
    pub next: NodeContinuation,
    /// Where failures of this node are routed.
    pub error: ErrorRouting,
}

/// Fallthrough continuation after a node completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeContinuation {
    /// No fallthrough step; stepping off raises `MissingNextStep`.
    End,
    /// Fall through to this step.
    Step(StepIdx),
}

impl NodeContinuation {
    /// Fallthrough target, or `None` when this node ends the path.
    #[must_use]
    pub const fn step(self) -> Option<StepIdx> {
        match self {
            Self::End => None,
            Self::Step(step) => Some(step),
        }
    }
}

impl From<Option<StepIdx>> for NodeContinuation {
    fn from(next: Option<StepIdx>) -> Self {
        match next {
            Some(step) => Self::Step(step),
            None => Self::End,
        }
    }
}

impl From<Option<SlotIdx>> for NodeOutput {
    fn from(output: Option<SlotIdx>) -> Self {
        match output {
            Some(slot) => Self::Slot(slot),
            None => Self::Discard,
        }
    }
}

/// Failure routing for a node.
///
/// The serialized form is the legacy `(on_error, error_slot)` option pair,
/// not the enum discriminant: `None` encodes as two absent options and
/// `Handler` as two present values. This keeps postcard bytes identical to
/// the pre-enum layout so existing artifacts and digests keep verifying.
/// Reads are lenient in exactly one case: a legacy present-slot-without-handler
/// pair decodes to `None`, matching the old runtime, which failed the run
/// without touching the slot when no handler was configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRouting {
    /// No error handler; failures fail the run.
    None,
    /// Route failures to a handler step, optionally recording into a slot.
    Handler {
        /// Handler step receiving control on failure.
        handler: StepIdx,
        /// Slot receiving failure information before routing.
        error_slot: Option<SlotIdx>,
    },
}

impl Serialize for ErrorRouting {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::None => (None::<StepIdx>, None::<SlotIdx>).serialize(serializer),
            Self::Handler {
                handler,
                error_slot,
            } => (Some(handler), error_slot).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ErrorRouting {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (handler, error_slot) =
            <(Option<StepIdx>, Option<SlotIdx>)>::deserialize(deserializer)?;
        Ok(match handler {
            Some(handler) => Self::Handler {
                handler,
                error_slot,
            },
            None => Self::None,
        })
    }
}

impl ErrorRouting {
    /// Handler step, when routing is configured.
    #[must_use]
    pub const fn handler(self) -> Option<StepIdx> {
        match self {
            Self::None => None,
            Self::Handler { handler, .. } => Some(handler),
        }
    }

    /// Failure-information slot, when configured.
    #[must_use]
    pub const fn error_slot(self) -> Option<SlotIdx> {
        match self {
            Self::None => None,
            Self::Handler { error_slot, .. } => error_slot,
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for NodeOutput {
    fn any() -> Self {
        if kani::any() {
            Self::Slot(SlotIdx::new(kani::any()))
        } else {
            Self::Discard
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for NodeContinuation {
    fn any() -> Self {
        if kani::any() {
            Self::Step(StepIdx::new(kani::any()))
        } else {
            Self::End
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for ErrorRouting {
    fn any() -> Self {
        if kani::any() {
            Self::Handler {
                handler: StepIdx::new(kani::any()),
                error_slot: kani::any(),
            }
        } else {
            Self::None
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for NodeRouting {
    fn any() -> Self {
        Self {
            next: kani::any(),
            error: kani::any(),
        }
    }
}
