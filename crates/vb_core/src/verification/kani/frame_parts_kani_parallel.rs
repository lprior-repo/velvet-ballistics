mod parallel_in_flight_kani {
    use crate::frame::{RunFrame, StepIdx};
    use crate::ids::RunId;

    #[kani::proof]
    fn add_parallel_in_flight_no_panic() {
        let count: u16 = kani::any();

        let frame = RunFrame::new(RunId::new(0), StepIdx::ZERO, 2, 4);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        kani::cover(count == u16::MAX, "max count");
        kani::cover(count == 0, "zero count");
        kani::cover(count > 0 && count < u16::MAX, "normal count");

        let result = frame.add_parallel_in_flight(count);
        kani::assert(result.is_ok(), "add_parallel_in_flight must not panic");
    }

    #[kani::proof]
    fn sub_parallel_in_flight_no_panic() {
        let count: u16 = kani::any();

        let frame = RunFrame::new(RunId::new(0), StepIdx::ZERO, 2, 4);
        kani::assume(frame.is_ok());
        let mut frame = frame.unwrap();

        let _result = frame.sub_parallel_in_flight(count);
    }
}
