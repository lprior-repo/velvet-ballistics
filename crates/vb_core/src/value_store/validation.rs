//! Value store arena validation helpers.

use crate::errors::{CoreError, CoreResult};
use crate::limits::{
    MAX_BLOB_BYTES_PER_VALUE, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
    MAX_SYMBOL_BYTES_PER_VALUE,
};

pub(super) fn validate_list_len(len: usize) -> CoreResult<()> {
    if len > MAX_LIST_ITEMS_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_symbol_len(len: usize) -> CoreResult<()> {
    if len > MAX_SYMBOL_BYTES_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "symbol_bytes",
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_blob_len(len: usize) -> CoreResult<()> {
    if len > MAX_BLOB_BYTES_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_object_len(len: usize) -> CoreResult<()> {
    if len > MAX_OBJECT_FIELDS_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "object_fields",
        })
    } else {
        Ok(())
    }
}
