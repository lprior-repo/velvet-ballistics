use crate::errors::{CoreError, CoreResult};
use crate::ids::ListId;
use crate::limits::MAX_LIST_ITEMS_PER_VALUE;

pub(crate) fn next_list_id(len: usize) -> CoreResult<ListId> {
    u32::try_from(len)
        .map(ListId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded { resource: "lists" })
}

pub(crate) fn validate_list_len(len: usize) -> CoreResult<()> {
    if len > MAX_LIST_ITEMS_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "list_items",
        })
    } else {
        Ok(())
    }
}

pub(crate) fn list_index(id: ListId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::ListOutOfBounds { list: id })
}
