    /// K-S1: read_slot never panics for SlotIdx within valid bounds.
    /// Uses kani::any() for slot_count with assume bound > 0 and <= 16.
    /// NOTE: Tighter bound (slot_count <= 16) prevents Kani timeout from large symbolic state space.
    #[kani::proof]
    fn read_slot_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16); // Tighter bound to reduce symbolic state space

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let init_result = frame.write_slot(slot, SlotValue::Null);
        kani::assume(init_result.is_ok());

        let result = frame.read_slot(slot);
        kani::assert(result.is_ok(), "read_slot with valid idx returns Ok");
    }

    /// K-S2: write_slot never panics for SlotIdx within valid bounds.
    /// Uses kani::any() for slot_count with assume bound > 0 and <= 16.
    /// NOTE: Tighter bound (slot_count <= 16) prevents Kani timeout from large symbolic state space.
    #[kani::proof]
    fn write_slot_no_panic() {
        let slot_count: u16 = kani::any();
        kani::assume(slot_count > 0);
        kani::assume(slot_count <= 16); // Tighter bound to reduce symbolic state space

        let slot_raw: u16 = kani::any();
        kani::assume(slot_raw < slot_count);
        let slot = SlotIdx::new(slot_raw);

        let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let result = frame.write_slot(slot, SlotValue::Null);
        kani::assert(result.is_ok(), "write_slot with valid idx returns Ok");
    }
}

