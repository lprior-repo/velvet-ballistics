//! vb_5iebh Verus verification artifacts.
//!
//! This module provides Verus specifications for IPC buffer extent types.

#[cfg(verus)]
pub mod checked_extents {
    use vb_ipc::BoundedReadExtent;
    use vb_ipc::BoundedWriteDrainExtent;

    /// Verus specification for BoundedReadExtent::new.
    ///
    /// Requires that offset and length are provided and the extent
    /// represents a valid read region within a buffer.
    pub spec fn is_valid_read_extent(extent: BoundedReadExtent) -> bool {
        extent.offset() <= extent.end()
    }

    /// Verus specification for BoundedWriteDrainExtent::new.
    ///
    /// Requires that offset and capacity are provided and the extent
    /// represents a valid write/drain region within a buffer.
    pub spec fn is_valid_write_drain_extent(extent: BoundedWriteDrainExtent) -> bool {
        extent.offset() <= extent.end()
    }
}
