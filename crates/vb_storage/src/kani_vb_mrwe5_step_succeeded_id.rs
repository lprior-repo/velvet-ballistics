// Kani proof: RecordKind::StepSucceeded.id() == 29.
// Obligation: vb-mrwe.5 PO-003

#[cfg(kani)]
mod kani_vb_mrwe5_step_succeeded_id {
    use crate::RecordKind;

    #[kani::proof]
    fn vb_mrwe5_step_succeeded_id() {
        let step_succeeded_kind = RecordKind::StepSucceeded;
        let id = step_succeeded_kind.id();

        kani::assert(id == 29, "RecordKind::StepSucceeded.id() must be 29");
        kani::assert(id != 28, "StepSucceeded id must not be 28 (RunKilled)");
        kani::assert(id != 12, "StepSucceeded id must not be 12 (SlotWritten)");
    }
}
