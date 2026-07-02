use crate::errors::{CoreError, CoreResult};
use crate::ids::{ObjectId, SymbolId};
use crate::limits::MAX_OBJECT_FIELDS_PER_VALUE;
use crate::value::{SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectField {
    pub key: SymbolId,
    pub value: SlotValue,
    pub taint: Taint,
}

impl ObjectField {
    #[must_use]
    pub const fn clean(key: SymbolId, value: SlotValue) -> Self {
        Self {
            key,
            value,
            taint: Taint::Clean,
        }
    }

    #[must_use]
    pub const fn with_taint(key: SymbolId, value: SlotValue, taint: Taint) -> Self {
        Self { key, value, taint }
    }
}

pub fn next_object_id(len: usize) -> CoreResult<ObjectId> {
    u32::try_from(len)
        .map(ObjectId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded {
            resource: "objects",
        })
}

pub fn validate_object_len(len: usize) -> CoreResult<()> {
    if len > MAX_OBJECT_FIELDS_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "object_fields",
        })
    } else {
        Ok(())
    }
}

pub fn object_index(id: ObjectId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::ObjectOutOfBounds { object: id })
}
