use super::{
    golden::{limits, reject},
    layout,
    support::{self, TestResult},
};
use vb_core::value_store::transport::CheckpointTransportError as Bounds;
use vb_runtime::run_continuation::{
    ContinuationTransportLimits, RunContinuationV1, TransportFailure,
};

#[test]
fn huge_lengths_in_every_present_nested_class_refuse_before_strong_decode() -> TestResult {
    let fixture = super::action_fixture::at_cut(false)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    let positions = layout::positions(&bytes, &fixture)?;
    let mut seen = std::collections::BTreeSet::new();
    for (field, offset) in positions {
        if field == "admission" {
            continue;
        }
        let hostile = layout::huge_length(&bytes, offset)?;
        let failure = reject(&hostile, &authority, limits(1_048_576)?)?;
        assert!(!failure.strong_decode_started, "allocated hostile {field}");
        assert!(
            matches!(failure.failure, TransportFailure::Bounds(_)),
            "wrong refusal for {field}"
        );
        assert!(failure.usage.allocation_bytes <= 67_108_864);
        seen.insert(field);
    }
    let expected = std::collections::BTreeSet::from([
        "frame_states",
        "frame_slots",
        "frame_taints",
        "symbols",
        "symbol_bytes",
        "lists",
        "list_items",
        "list_taints",
        "item_taints",
        "objects",
        "object_fields",
        "blobs",
        "blob_bytes",
        "action_attempts",
        "collect_entries",
        "collect_lineages",
        "action_contracts",
        "admission_grants",
        "contract_name",
        "contract_capabilities",
    ]);
    assert_eq!(seen, expected);
    Ok(())
}

#[test]
fn every_truncation_and_trailing_byte_refuse_before_strong_decode() -> TestResult {
    let fixture = support::at_cut(true)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    for end in 0..bytes.len() {
        let short = bytes.get(..end).ok_or("truncation range")?;
        let error = reject(short, &authority, limits(1_048_576)?)?;
        assert!(!error.strong_decode_started);
    }
    let mut trailing = bytes;
    trailing.push(0);
    let error = reject(&trailing, &authority, limits(1_048_576)?)?;
    assert!(matches!(
        error.failure,
        TransportFailure::Bounds(Bounds::Malformed("trailing_bytes"))
    ));
    assert!(!error.strong_decode_started);
    Ok(())
}

#[test]
fn version_digest_and_independent_authority_fail_before_strong_decode() -> TestResult {
    let fixture = support::at_cut(false)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    let mut version = bytes.clone();
    *version.first_mut().ok_or("version missing")? = 2;
    let error = reject(&version, &authority, limits(1_048_576)?)?;
    assert!(matches!(
        error.failure,
        TransportFailure::UnsupportedVersion(2)
    ));
    assert!(!error.strong_decode_started);
    let mut digest = bytes.clone();
    *digest.last_mut().ok_or("digest missing")? ^= 1;
    let error = reject(&digest, &authority, limits(1_048_576)?)?;
    assert!(matches!(error.failure, TransportFailure::Integrity));
    assert!(!error.strong_decode_started);
    let mut wrong = support::at_cut(false)?;
    wrong.expected.prior_seq = vb_storage::EventSeq::new(99);
    let other = wrong.authority()?;
    let error = reject(&bytes, &other, limits(1_048_576)?)?;
    assert!(!error.strong_decode_started);
    Ok(())
}

#[test]
fn aggregate_structural_work_budget_refuses_even_when_component_fits() -> TestResult {
    let fixture = support::at_cut(false)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    let initialization_work = 1_048_576;
    let (_, usage) =
        RunContinuationV1::from_transport_bytes(&bytes, &authority, limits(initialization_work)?)?;
    assert!(usage.work > initialization_work);
    let exact = ContinuationTransportLimits::new(initialization_work, 67_108_864, usage.work)?;
    let (_, exact_usage) = RunContinuationV1::from_transport_bytes(&bytes, &authority, exact)?;
    assert_eq!(exact_usage.work, usage.work);
    let one_less = usage.work.checked_sub(1).ok_or("zero work charge")?;
    let limited = ContinuationTransportLimits::new(initialization_work, 67_108_864, one_less)?;
    let error = reject(&bytes, &authority, limited)?;
    assert!(matches!(
        error.failure,
        TransportFailure::Bounds(Bounds::Budget("work"))
    ));
    assert!(!error.strong_decode_started);
    assert!(error.usage.work > initialization_work);
    assert!(error.usage.work <= one_less);
    Ok(())
}
