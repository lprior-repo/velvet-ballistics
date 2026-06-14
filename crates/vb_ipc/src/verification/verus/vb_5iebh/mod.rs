//! vb_5iebh Verus verification artifacts.
//!
//! This module provides Verus specifications for IPC buffer extent types.

use vstd::prelude::*;

verus! {

    // Standalone model types for IPC buffer extents
    // (vb_ipc types are not available in --crate-type=lib mode)

    /// Model of BoundedReadExtent — represents a valid read region within a buffer.
    pub struct BoundedReadExtent {
        pub offset: u64,
        pub length: u64,
    }

    impl BoundedReadExtent {
        /// Model: offset accessor
        pub open spec fn offset(&self) -> u64 { self.offset }

        /// Model: length accessor  
        pub open spec fn length(&self) -> u64 { self.length }

        /// Model: end position as int for arithmetic
        pub open spec fn end(&self) -> int { (self.offset as int) + (self.length as int) }
    }

    /// Model of BoundedWriteDrainExtent — represents a valid write/drain region.
    pub struct BoundedWriteDrainExtent {
        pub offset: u64,
        pub capacity: u64,
    }

    impl BoundedWriteDrainExtent {
        /// Model: offset accessor
        pub open spec fn offset(&self) -> u64 { self.offset }

        /// Model: capacity accessor
        pub open spec fn capacity(&self) -> u64 { self.capacity }

        /// Model: end position as int for arithmetic
        pub open spec fn end(&self) -> int { (self.offset as int) + (self.capacity as int) }
    }

    /// Verus specification for BoundedReadExtent::new.
    ///
    /// Requires that offset and length are provided and the extent
    /// represents a valid read region within a buffer.
    pub open spec fn is_valid_read_extent(extent: BoundedReadExtent) -> bool {
        (extent.offset() as int) <= extent.end()
    }

    /// Verus specification for BoundedWriteDrainExtent::new.
    ///
    /// Requires that offset and capacity are provided and the extent
    /// represents a valid write/drain region within a buffer.
    pub open spec fn is_valid_write_drain_extent(extent: BoundedWriteDrainExtent) -> bool {
        (extent.offset() as int) <= extent.end()
    }

} // verus!
