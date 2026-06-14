//! PS-005 Verus proof: Duplicate key handling idempotency (POB-vb-fzgdn-019)
use vstd::prelude::*;

verus! {

    pub struct TimerSlot {
        pub present: bool,
        pub generation: u64,
    }

    impl TimerSlot {
        pub closed spec fn insert_spec(self) -> Self {
            if self.present {
                TimerSlot { present: true, generation: self.generation + 1 }
            } else {
                TimerSlot { present: true, generation: 1 }
            }
        }

        pub closed spec fn cancel_spec(self) -> (Self, bool) {
            if self.present {
                (TimerSlot { present: false, generation: 0 }, true)
            } else {
                (self, false)
            }
        }
    }

    proof fn test_first_insert()
        ensures (TimerSlot { present: false, generation: 0 }).insert_spec() == (TimerSlot { present: true, generation: 1 }),
    {
        let slot = TimerSlot { present: false, generation: 0 };
        assert(slot.insert_spec() == (TimerSlot { present: true, generation: 1 })) by (compute);
    }

    proof fn test_replacement_increments()
        ensures
            forall |s: TimerSlot| s.present ==>
                s.insert_spec().present && s.insert_spec().generation == s.generation + 1,
    {
        assert forall |s: TimerSlot| s.present ==>
            s.insert_spec().present && s.insert_spec().generation == s.generation + 1 by {
            if s.present {
                let result = s.insert_spec();
                assert(result.present);
                assert(result.generation == s.generation + 1);
            }
        };
    }

    proof fn test_cancel_empty()
        ensures (TimerSlot { present: false, generation: 0 }).cancel_spec() == ((TimerSlot { present: false, generation: 0 }), false),
    {
        let slot = TimerSlot { present: false, generation: 0 };
        assert(slot.cancel_spec().1 == false) by (compute);
    }

    proof fn test_cancel_filled()
        ensures (TimerSlot { present: true, generation: 5 }).cancel_spec() == ((TimerSlot { present: false, generation: 0 }), true),
    {
        let slot = TimerSlot { present: true, generation: 5 };
        assert(slot.cancel_spec().1 == true) by (compute);
    }

} // verus!
