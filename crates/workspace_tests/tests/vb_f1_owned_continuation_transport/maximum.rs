use super::{
    golden::limits,
    support::{self, TestResult},
};
use vb_runtime::run_continuation::{ContinuationTransportLimits, RunContinuationV1};

#[test]
fn genuine_owned_payload_at_full_mebibyte_and_one_byte_over() -> TestResult {
    let small = support::at_cut(false)?;
    let small_authority = small.authority()?;
    let base = RunContinuationV1::capture(&small.state, &small_authority)?
        .to_bytes()?
        .len();
    // The genuine blob grows from three bytes (one-byte length) to a length
    // requiring three postcard bytes. All other component bytes remain identical.
    let blob_bytes = 1_048_576_usize
        .checked_sub(base)
        .and_then(|n| n.checked_add(1))
        .ok_or("payload geometry")?;
    assert!((16_384..2_097_152).contains(&blob_bytes));
    for extra in [0_usize, 1] {
        let fixture = support::at_cut_with_blob(
            false,
            blob_bytes.checked_add(extra).ok_or("blob overflow")?,
        )?;
        let authority = fixture.authority()?;
        let ordinary = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
        assert_eq!(
            ordinary.len(),
            1_048_576_usize.checked_add(extra).ok_or("wire overflow")?
        );
        let captured =
            RunContinuationV1::capture_transport(&fixture.state, &authority, limits(1_048_576)?);
        if extra == 0 {
            assert_eq!(captured?.as_bytes(), ordinary);
            RunContinuationV1::from_transport_bytes(&ordinary, &authority, limits(1_048_576)?)?;
        } else {
            assert!(captured.is_err());
            let error =
                RunContinuationV1::from_transport_bytes(&ordinary, &authority, limits(1_048_576)?)
                    .err()
                    .ok_or("oversized accepted")?;
            assert!(!error.strong_decode_started);
            assert_eq!(error.usage.allocation_bytes, 0);
        }
    }
    Ok(())
}

#[test]
fn hard_operation_and_component_limits_never_silently_increase() {
    assert!(ContinuationTransportLimits::new(1_048_577, 67_108_864, 16_777_216).is_err());
    assert!(ContinuationTransportLimits::new(1_048_576, 67_108_865, 16_777_216).is_err());
    assert!(ContinuationTransportLimits::new(1_048_576, 67_108_864, 16_777_217).is_err());
    assert!(ContinuationTransportLimits::new(0, 67_108_864, 16_777_216).is_err());
}

#[test]
fn overlong_varints_refuse_without_entering_strong_decode() -> TestResult {
    let fixture = support::at_cut(false)?;
    let authority = fixture.authority()?;
    let bytes = RunContinuationV1::capture(&fixture.state, &authority)?.to_bytes()?;
    let mut hostile = vec![0x81, 0];
    hostile.extend_from_slice(bytes.get(1..).ok_or("wire prefix")?);
    let error = RunContinuationV1::from_transport_bytes(&hostile, &authority, limits(1_048_576)?)
        .err()
        .ok_or("overlong integer accepted")?;
    assert!(!error.strong_decode_started);
    Ok(())
}
