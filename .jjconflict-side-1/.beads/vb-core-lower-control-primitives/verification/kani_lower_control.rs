// verification/kani_lower_control.rs
// Kani harness: id+1 overflow bounded model checking for lower_repeat and lower_ask
//
// bead_id: vb-core-lower-control-primitives
// obligation: KANI-OVERFLOW
//
// Fixed defects per proof-repair-guide.md:
//   1. Removed dead code (duplicate StepIdx::new)
//   2. Removed unnecessary .max(0) (unsigned addition cannot be negative)
//   3. Added concrete attempt_slot raw_value() verification
//
// STUB: This harness targets the actual lower_repeat and lower_ask implementations
// in crates/vb_compile/src/lib.rs. When vb-f04l lands or when this module is added
// to the vb_compile crate, this file should be placed at:
//   crates/vb_compile/src/kani_lower_control.rs
// and added to lib.rs under:  #[cfg(kani)] pub mod kani_lower_control;
//
// Scope: id ∈ [0, u16::MAX−1] — verifies no counterexample for id+1 overflow
//
// PROOF OBLIGATION: For all id in 0..u16::MAX-1, id.checked_add(1) succeeds
//                   and the resulting StepIdx/SlotIdx is valid.
//
// Integration:  cargo kani --harness kani_lower_control --force-mc-flags

#![forbid(unsafe_code)]

// When compiled under kani, this module exposes the lowering functions.
#[cfg(kani)]
mod proof_harness {
    use crate::{lower_repeat, lower_ask, SlotCompiler, StepIdx, SlotIdx};
    use crate::CompiledNodeKind;

    /// KANI-OVERFLOW: Bounded model checking for id+1 overflow in lower_repeat.
    ///
    /// id ∈ [0, u16::MAX−1] is assumed (excludes u16::MAX to keep +1 in range).
    /// Proves: for all such id, id.checked_add(1) succeeds and produces a valid StepIdx.
    ///
    /// The implementation at lib.rs:555-559:
    ///   let attempt_slot = slot_idx_for_step(
    ///       id.as_usize().checked_add(1)
    ///           .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?,
    ///   )?;
    ///
    /// FIX 1: Removed duplicate `let id = StepIdx::new(kani::any());`
    /// FIX 2: Removed `.max(0)` from body/done StepIdx construction
    /// FIX 3: Added concrete attempt_slot raw_value() assertion
    #[kani::proof]
    #[kani::unwind(5)] // id+1 path depth
    fn kani_lower_repeat_id_increment() {
        // Choose any id in the safe range [0, u16::MAX−1].
        let id_val: u16 = kani::any();
        kani::assume(id_val < u16::MAX); // Exclude MAX to keep +1 in bounds

        // Convert to StepIdx — valid for u16 range.
        let id = StepIdx::new(id_val as usize);

        // Build a minimal SlotCompiler.
        let mut builder = SlotCompiler::new();

        // body and done must be distinct from id per v1 rules.
        // FIX: removed .max(0) — unsigned addition cannot overflow to negative.
        let body = StepIdx::new((id_val as usize) + 100);
        let done = StepIdx::new((id_val as usize) + 101);

        // Run the function — it must not panic and should return Ok.
        let result = lower_repeat(id, 3, body, done, &mut builder);

        // Assert: the function succeeds for all non-MAX ids.
        match result {
            Ok(nodes) => {
                // Verify we got exactly 3 nodes as per POST-005.
                assert!(nodes.len() == 3, "lower_repeat must return exactly 3 nodes");

                // FIX 3: Verify concrete attempt_slot value.
                // nodes[1] is RepeatAttempt with attempt_slot = id+1.
                if let CompiledNodeKind::RepeatAttempt { attempt_slot, .. } = &nodes[1].kind {
                    assert!(
                        attempt_slot.raw_value() == id_val as usize + 1,
                        "attempt_slot must equal id + 1"
                    );
                } else {
                    // If the kind doesn't match, this is a structural bug.
                    assert!(false, "nodes[1] must be RepeatAttempt");
                }

                // Verify RepeatFinish uses the same attempt_slot.
                if let CompiledNodeKind::RepeatFinish { result: result_slot } = &nodes[2].kind {
                    assert!(
                        result_slot.raw_value() == id_val as usize + 1,
                        "RepeatFinish.result must equal id + 1"
                    );
                } else {
                    assert!(false, "nodes[2] must be RepeatFinish");
                }
            }
            Err(_) => {
                // Any error for id < u16::MAX indicates a bug in the implementation.
                // The only legitimate error would be if id == u16::MAX - 1 caused
                // a checked_add failure, but we excluded that via kani::assume.
                assert!(false, "lower_repeat returned Err for valid id < u16::MAX");
            }
        }
    }

    /// KANI-OVERFLOW: Bounded model checking for id+1 overflow in lower_ask.
    ///
    /// Same structure as kani_lower_repeat_id_increment but for lower_ask.
    /// The implementation at lib.rs:654-661 uses the same id.checked_add(1) pattern.
    #[kani::proof]
    #[kani::unwind(5)]
    fn kani_lower_ask_id_increment() {
        let id_val: u16 = kani::any();
        kani::assume(id_val < u16::MAX);

        let id = StepIdx::new(id_val as usize);

        let mut builder = SlotCompiler::new();
        let prompt = SlotIdx::new(0);
        let answer = SlotIdx::new(1);

        let result = lower_ask(id, prompt, answer, None, &mut builder);

        match result {
            Ok(nodes) => {
                // lower_ask returns exactly 2 nodes.
                assert!(nodes.len() == 2, "lower_ask must return exactly 2 nodes");

                // nodes[0] is Ask.
                assert!(
                    matches!(&nodes[0].kind, CompiledNodeKind::Ask { .. }),
                    "nodes[0] must be Ask"
                );

                // nodes[1] is AskResume; its id should be id+1.
                if let CompiledNodeKind::AskResume { answer: ans } = &nodes[1].kind {
                    // The resume step id = id+1 is the key overflow check.
                    // If id.checked_add(1) had overflowed, we would get Err.
                    assert!(
                        nodes[1].id.raw_value() == id_val as usize + 1,
                        "AskResume.id must equal id + 1"
                    );
                    assert!(
                        *ans == answer,
                        "AskResume.answer must match input answer slot"
                    );
                } else {
                    assert!(false, "nodes[1] must be AskResume");
                }
            }
            Err(_) => {
                // Any error for id < u16::MAX would indicate a bug.
                assert!(false, "lower_ask returned Err for valid id < u16::MAX");
            }
        }
    }
}
