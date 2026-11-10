use super::{Check, InputValueIssue};
use crate::{
    BlobId, CompiledInputConstraint, InputSlotKind, ListId, ObjectId, SlotValue, SymbolId,
};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::mem::Discriminant;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Handle {
    Symbol(SymbolId),
    List(ListId),
    Object(ObjectId),
    Blob(BlobId),
}

#[derive(Clone, Copy)]
struct ConstraintIdentity<'a>(&'a CompiledInputConstraint);

impl PartialEq for ConstraintIdentity<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for ConstraintIdentity<'_> {}
impl Hash for ConstraintIdentity<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Key<'a> {
    handle: Handle,
    kind: Discriminant<InputSlotKind>,
    constraint: ConstraintIdentity<'a>,
    depth: usize,
}

impl<'a> Key<'a> {
    pub(super) fn of(check: &Check<'a>) -> Option<Self> {
        let handle = match check.value {
            SlotValue::Symbol(id) => Handle::Symbol(id),
            SlotValue::List(id) => Handle::List(id),
            SlotValue::Object(id) => Handle::Object(id),
            SlotValue::Blob(id) => Handle::Blob(id),
            _ => return None,
        };
        Some(Self {
            handle,
            kind: std::mem::discriminant(&check.kind),
            constraint: ConstraintIdentity(check.constraint),
            depth: check.depth,
        })
    }
}

pub(super) fn complete<'a>(
    completed: &mut HashSet<Key<'a>>,
    key: Key<'a>,
) -> Result<(), InputValueIssue> {
    completed
        .try_reserve(1)
        .map_err(|_| InputValueIssue::Capacity)?;
    completed.insert(key);
    Ok(())
}
