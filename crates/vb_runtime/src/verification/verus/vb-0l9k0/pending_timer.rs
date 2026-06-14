//! Standalone model for PendingTimer and PendingTimerKind.
//!
//! Production binding targets:
//! - `crates/vb_runtime/src/shard/types.rs:29-54` - PendingTimerKind and PendingTimer

use vstd::prelude::*;

verus! {

    // Standalone model types
    struct Instant(u64);

    impl Instant {
        pub closed spec fn val(&self) -> u64 { self.0 }
    }

    enum PendingTimerKind {
        WaitUntil,
        WaitEvent,
        Ask,
    }

    struct PendingTimer {
        pub kind: PendingTimerKind,
        pub deadline: Instant,
    }

    impl PendingTimer {
        pub closed spec fn kind(&self) -> PendingTimerKind { self.kind }
        pub closed spec fn deadline(&self) -> Instant { self.deadline }
    }
}

