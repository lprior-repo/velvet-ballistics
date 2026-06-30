use crate::errors::{CoreError, CoreResult};
use crate::ids::BlobId;
use crate::limits::MAX_BLOB_BYTES_PER_VALUE;

pub fn next_blob_id(len: usize) -> CoreResult<BlobId> {
    u64::try_from(len)
        .map(BlobId::new)
        .map_err(|_| CoreError::ResourceLimitExceeded { resource: "blobs" })
}

pub fn validate_blob_len(len: usize) -> CoreResult<()> {
    if len > MAX_BLOB_BYTES_PER_VALUE {
        Err(CoreError::ResourceLimitExceeded {
            resource: "blob_bytes",
        })
    } else {
        Ok(())
    }
}

pub fn blob_index(id: BlobId) -> CoreResult<usize> {
    usize::try_from(id.get()).map_err(|_| CoreError::BlobOutOfBounds { blob: id })
}
