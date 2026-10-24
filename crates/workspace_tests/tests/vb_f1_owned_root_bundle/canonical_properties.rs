// canonical_properties.rs
// Property-based test: reversed root ordering preserves all values.
// 64 generated cases under ONE run test; sampled property, not formal proof.

use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, RngSeed, TestCaseError};

use super::support::{TestResult, checkpoint, fixture};
use vb_core::ids::SlotIdx;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledInputSlot, InputSlotKind};

fn canonical_case(values: Vec<i64>) -> TestResult<()> {
    let frame_count = u16::try_from(values.len())?;
    let inputs: Vec<CompiledInputSlot> = values
        .iter()
        .enumerate()
        .map(
            |(index, _)| -> Result<CompiledInputSlot, std::num::TryFromIntError> {
                let slot = SlotIdx::new(u16::try_from(index)?);
                Ok(CompiledInputSlot {
                    slot,
                    kind: InputSlotKind::I64,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    let wf = fixture(frame_count, inputs, 32, 4)?;
    let cp = checkpoint(32);
    let ascending: Box<[(SlotIdx, SlotValue, Taint)]> = values
        .iter()
        .enumerate()
        .map(
            |(index, v)| -> Result<(SlotIdx, SlotValue, Taint), std::num::TryFromIntError> {
                let slot = SlotIdx::new(u16::try_from(index)?);
                Ok((slot, SlotValue::I64(*v), Taint::Clean))
            },
        )
        .collect::<Result<Vec<_>, _>>()?
        .into();
    let reversed = ascending.clone();
    let mut rev: Vec<(SlotIdx, SlotValue, Taint)> = reversed.into();
    rev.reverse();
    let bundle = OwnedRootBundle::new(rev.into(), cp.clone(), &wf)?;
    assert_eq!(bundle.roots(), ascending.as_ref());
    assert_eq!(bundle.checkpoint(), &cp);
    assert_eq!(bundle.input_slots(), wf.input_slots());
    assert!(bundle.matches_workflow(&wf));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        rng_seed: RngSeed::Fixed(0x5eeda11),
        failure_persistence: None,
        max_shrink_iters: 256,
        ..ProptestConfig::default()
    })]

    #[test]
    fn reversed_roots_preserve_all_values(
        values in proptest::collection::vec(any::<i64>(), 1..33)
    ) {
        canonical_case(values)
            .map_err(|err| TestCaseError::fail(err.to_string()))?;
    }
}
