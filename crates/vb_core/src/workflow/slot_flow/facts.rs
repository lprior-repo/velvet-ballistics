use super::{FlowResult, SlotFlowFailure, capacity, failure};
use crate::ids::StepIdx;
use crate::value::{ConstValue, SlotValue};
use crate::workflow::{InputSlotKind, WorkflowParts};

pub(super) type Facts = Vec<Fact>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Fact {
    pub(super) kinds: u16,
    pub(super) absent: bool,
}

impl Fact {
    pub(super) const NULL: u16 = 1;
    pub(super) const BOOL: u16 = 2;
    pub(super) const I64: u16 = 4;
    pub(super) const F64: u16 = 8;
    pub(super) const SYMBOL: u16 = 16;
    pub(super) const LIST: u16 = 32;
    pub(super) const OBJECT: u16 = 64;
    pub(super) const BLOB: u16 = 128;
    pub(super) const DYNAMIC: u16 = 256;
    pub(super) const UNSET: Self = Self {
        kinds: 0,
        absent: true,
    };

    pub(super) const fn assigned(kinds: u16) -> Self {
        Self {
            kinds,
            absent: false,
        }
    }

    pub(super) fn join(&mut self, other: Self) -> bool {
        let before = *self;
        self.kinds |= other.kinds;
        self.absent |= other.absent;
        *self != before
    }

    pub(super) fn input(kind: InputSlotKind) -> Self {
        Self::assigned(match kind {
            InputSlotKind::Null => Self::NULL,
            InputSlotKind::Bool => Self::BOOL,
            InputSlotKind::I64 => Self::I64,
            InputSlotKind::F64 => Self::F64,
            InputSlotKind::Symbol => Self::SYMBOL,
            InputSlotKind::List => Self::LIST,
            InputSlotKind::Object => Self::OBJECT,
            InputSlotKind::Blob => Self::BLOB,
            InputSlotKind::Any => Self::DYNAMIC,
        })
    }

    pub(super) fn value(value: SlotValue) -> Self {
        Self::assigned(match value {
            SlotValue::Null => Self::NULL,
            SlotValue::Bool(_) => Self::BOOL,
            SlotValue::I64(_) => Self::I64,
            SlotValue::F64(_) => Self::F64,
            SlotValue::Symbol(_) => Self::SYMBOL,
            SlotValue::List(_) => Self::LIST,
            SlotValue::Object(_) => Self::OBJECT,
            SlotValue::Blob(_) => Self::BLOB,
        })
    }

    pub(super) fn constant(value: &ConstValue) -> Self {
        Self::assigned(match value {
            ConstValue::Null => Self::NULL,
            ConstValue::Bool(_) => Self::BOOL,
            ConstValue::I64(_) => Self::I64,
            ConstValue::F64(_) => Self::F64,
            ConstValue::Symbol(_) => Self::SYMBOL,
        })
    }
}

pub(super) fn copy(facts: &[Fact]) -> FlowResult<Facts> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(facts.len())
        .map_err(|_| capacity())?;
    result.extend_from_slice(facts);
    Ok(result)
}

pub(super) fn seed(parts: &WorkflowParts) -> FlowResult<Facts> {
    let mut facts = Vec::new();
    facts
        .try_reserve_exact(usize::from(parts.slot_count))
        .map_err(|_| capacity())?;
    facts.resize(usize::from(parts.slot_count), Fact::UNSET);
    for input in &parts.input_slots {
        let mut fact = Fact::input(input.kind);
        if input.constraint.nullability.allows_null() {
            fact.kinds |= Fact::NULL;
        }
        if let Some(value) = input.constraint.default_value {
            fact.kinds |= Fact::value(value).kinds;
        }
        fact.absent = input.constraint.presence.is_optional()
            && input.constraint.default_value.is_none()
            && input.constraint.default_cold.is_none();
        let dest = facts.get_mut(input.slot.as_usize()).ok_or_else(|| {
            failure(
                parts.entry,
                Some(input.slot),
                SlotFlowFailure::InvalidReference,
            )
        })?;
        *dest = fact;
    }
    Ok(facts)
}

pub(super) fn merge(left: &mut [Fact], right: &[Fact]) -> bool {
    let mut changed = false;
    for (dest, source) in left.iter_mut().zip(right) {
        changed |= dest.join(*source);
    }
    changed
}

pub(super) fn constant_writers(parts: &WorkflowParts) -> FlowResult<()> {
    let mut writers = seed(parts)?;
    writers.fill(Fact::UNSET);
    for node in &parts.nodes {
        if let crate::workflow::CompiledNodeKind::SetConst { value } = node.kind {
            let Some(constant) = parts.constants.get(value.as_usize()) else {
                continue;
            };
            let Some(slot) = node.output.slot() else {
                continue;
            };
            let dest = writers
                .get_mut(slot.as_usize())
                .ok_or_else(|| failure(node.id, Some(slot), SlotFlowFailure::InvalidReference))?;
            let kind = Fact::constant(constant);
            if dest.kinds != 0 && dest.kinds != kind.kinds {
                return Err(failure(
                    node.id,
                    Some(slot),
                    SlotFlowFailure::ConflictingConstantWrites,
                ));
            }
            *dest = kind;
        }
    }
    Ok(())
}

pub(super) fn invalid(step: StepIdx) -> super::SlotFlowError {
    failure(step, None, SlotFlowFailure::InvalidReference)
}
