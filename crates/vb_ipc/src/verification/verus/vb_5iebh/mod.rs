//! vb_5iebh Verus verification artifacts.
//!
//! This module provides Verus specifications for IPC buffer extent types.

use vstd::prelude::*;

verus! {

    // Standalone model types (vb_ipc types not available in standalone mode)
    pub struct BoundedReadExtent {
        pub offset: u64,
        pub end: u64,
    }

    pub struct BoundedWriteDrainExtent {
        pub offset: u64,
        pub end: u64,
    }

    impl BoundedReadExtent {
        pub open spec fn offset(&self) -> u64 { self.offset }
        pub open spec fn end(&self) -> u64 { self.end }
    }

    impl BoundedWriteDrainExtent {
        pub open spec fn offset(&self) -> u64 { self.offset }
        pub open spec fn end(&self) -> u64 { self.end }
    }

    /// Verus specification for BoundedReadExtent::new.
    ///
    /// Requires that offset and length are provided and the extent
    /// represents a valid read region within a buffer.
    pub open spec fn is_valid_read_extent(extent: BoundedReadExtent) -> bool {
        extent.offset() <= extent.end()
    }

    /// Verus specification for BoundedWriteDrainExtent::new.
    ///
    /// Requires that offset and capacity are provided and the extent
    /// represents a valid write/drain region within a buffer.
    pub open spec fn is_valid_write_drain_extent(extent: BoundedWriteDrainExtent) -> bool {
        extent.offset() <= extent.end()
    }
}

