use super::{InputMappingError, map_runtime_inputs};
use std::error::Error;
use vb_core::ids::{StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts,
};
use vb_core::{SlotIdx, SlotValue};

type TestResult<T> = Result<T, Box<dyn Error>>;

fn fixture(input_slots: Box<[CompiledInputSlot]>) -> TestResult<CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("input-mapping"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
        input_slots,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string().into())
}

fn i64_port() -> CompiledInputSlot {
    CompiledInputSlot {
        slot: SlotIdx::new(2),
        kind: InputSlotKind::I64,
    }
}
fn bool_port() -> CompiledInputSlot {
    CompiledInputSlot {
        slot: SlotIdx::new(3),
        kind: InputSlotKind::Bool,
    }
}

#[test]
fn zeroports_raw_empty() -> TestResult<()> {
    let wf = fixture(Box::from([]))?;
    let result = map_runtime_inputs(&wf, &[]);
    assert_eq!(result, Ok(Box::from([])));
    Ok(())
}

#[test]
fn zeroports_encoded_empty() -> TestResult<()> {
    let wf = fixture(Box::from([]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Ok(Box::from([])));
    Ok(())
}

#[test]
fn zeroports_malformed_255() -> TestResult<()> {
    let wf = fixture(Box::from([]))?;
    let buf: Vec<u8> = vec![255];
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Err(InputMappingError::DecodeFailed));
    Ok(())
}

#[test]
fn zeroports_one_i64() -> TestResult<()> {
    let wf = fixture(Box::from([]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(99)]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Err(InputMappingError::SlotCountExceeded));
    Ok(())
}

#[test]
fn zeroports_encoded_empty_plus_255() -> TestResult<()> {
    let wf = fixture(Box::from([]))?;
    let mut buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([]))?;
    buf.push(255);
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Err(InputMappingError::DecodeFailed));
    Ok(())
}

#[test]
fn with_i64_port_empty_raw() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let result = map_runtime_inputs(&wf, &[]);
    assert_eq!(result, Ok(Box::from([])));
    Ok(())
}

#[test]
fn slot2_i64_mapping_i64_42() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(42)]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(
        result,
        Ok(Box::from([(SlotIdx::new(2), SlotValue::I64(42))]))
    );
    Ok(())
}

#[test]
fn slots_2_i64_3_bool_partial_1_maps_2() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port(), bool_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(11)]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(
        result,
        Ok(Box::from([(SlotIdx::new(2), SlotValue::I64(11))]))
    );
    Ok(())
}

#[test]
fn full_2_correct_order() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port(), bool_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([
        SlotValue::I64(1),
        SlotValue::Bool(true),
    ]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(
        result,
        Ok(Box::from([
            (SlotIdx::new(2), SlotValue::I64(1)),
            (SlotIdx::new(3), SlotValue::Bool(true)),
        ]))
    );
    Ok(())
}

#[test]
fn i64_port_bool() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::Bool(true)]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(
        result,
        Err(InputMappingError::KindMismatch {
            slot: SlotIdx::new(2),
            expected_kind: InputSlotKind::I64,
        })
    );
    Ok(())
}

#[test]
fn two_values_for_one_port() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([
        SlotValue::I64(1),
        SlotValue::I64(2),
    ]))?;
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Err(InputMappingError::SlotCountExceeded));
    Ok(())
}

#[test]
fn ports_255() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let buf: Vec<u8> = vec![255];
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Err(InputMappingError::DecodeFailed));
    Ok(())
}

#[test]
fn valid_one_plus_255() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let mut buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(1)]))?;
    buf.push(255);
    let result = map_runtime_inputs(&wf, &buf);
    assert_eq!(result, Err(InputMappingError::DecodeFailed));
    Ok(())
}

#[test]
fn trailing_bytes_precede_excess_count() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([
        SlotValue::I64(1),
        SlotValue::I64(2),
    ]))?;
    let mut buf = buf;
    buf.push(255);
    assert_eq!(
        map_runtime_inputs(&wf, &buf),
        Err(InputMappingError::DecodeFailed)
    );
    Ok(())
}

#[test]
fn trailing_bytes_precede_kind_mismatch() -> TestResult<()> {
    let wf = fixture(Box::from([i64_port()]))?;
    let buf = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::Bool(true)]))?;
    let mut buf = buf;
    buf.push(255);
    assert_eq!(
        map_runtime_inputs(&wf, &buf),
        Err(InputMappingError::DecodeFailed)
    );
    Ok(())
}

proptest::proptest! {
    #[test]
    fn typed_i64_mapping_and_trailing_rejection(value in proptest::num::i64::ANY, junk in proptest::collection::vec(proptest::num::u8::ANY, 1..17)) {
        let wf = fixture(Box::from([i64_port()]))
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let mut bytes = postcard::to_allocvec(&Box::<[SlotValue]>::from([SlotValue::I64(value)]))
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        proptest::prop_assert_eq!(
            map_runtime_inputs(&wf, &bytes),
            Ok(Box::from([(SlotIdx::new(2), SlotValue::I64(value))]))
        );
        bytes.extend(junk);
        proptest::prop_assert_eq!(
            map_runtime_inputs(&wf, &bytes),
            Err(InputMappingError::DecodeFailed)
        );
    }
}

#[test]
fn any_port_maps_primitives() -> TestResult<()> {
    let wf = fixture(Box::from([CompiledInputSlot {
        slot: SlotIdx::new(2),
        kind: InputSlotKind::Any,
    }]))?;
    let values: [SlotValue; 4] = [
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::I64(i64::MIN),
        SlotValue::I64(i64::MAX),
    ];
    for value in values {
        let bytes = postcard::to_allocvec(&Box::<[SlotValue]>::from([value]))?;
        assert_eq!(
            map_runtime_inputs(&wf, &bytes),
            Ok(Box::from([(SlotIdx::new(2), value)]))
        );
    }
    Ok(())
}
