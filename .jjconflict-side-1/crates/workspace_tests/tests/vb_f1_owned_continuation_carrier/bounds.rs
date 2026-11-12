use super::support::*;
use vb_core::value_store::transport::CheckpointTransportError as Bounds;
use vb_runtime::run_continuation::{
    ContinuationTransportLimits, RunContinuationV1, TransportFailure,
};
use vb_storage::{
    CarrierError, ContinuationCarrierBudget as Budget, decode_owned_continuation,
    encode_owned_continuation,
};

#[test]
fn genuine_complete_record_at_one_mebibyte_and_one_over() -> TestResult {
    let (_, _, small, _) = fixture(false, 3)?;
    // Big component length needs three postcard bytes; fixed scalar prefix is7,
    // digest32, length3 and envelope60. Blob length grows from1 to3 wire bytes.
    let component_limit = MAX.checked_sub(102).ok_or("geometry")?;
    let blob = component_limit
        .checked_sub(small.len())
        .and_then(|n| n.checked_add(1))
        .ok_or("blob geometry")?;
    for extra in [0_usize, 1] {
        let (_, _, component, header) = fixture(false, blob.checked_add(extra).ok_or("blob")?)?;
        assert_eq!(
            component.len(),
            component_limit.checked_add(extra).ok_or("component")?
        );
        assert_maximum(&component, header, extra)?;
    }
    Ok(())
}

fn assert_maximum(
    component: &[u8],
    header: vb_storage::ContinuationCarrierHeader,
    extra: usize,
) -> TestResult {
    let result = encode_owned_continuation(header, &component, &mut budget()?);
    if extra == 0 {
        let record = result?;
        assert_eq!(record.as_bytes().len(), MAX);
        decode_owned_continuation(
            &key(header)?,
            header.run(),
            record.as_bytes(),
            &mut budget()?,
        )?;
        assert!(
            encode_owned_continuation(
                header,
                &component,
                &mut Budget::new(MAX - 1, ALLOCATION, WORK)?
            )
            .is_err()
        );
    } else {
        assert!(matches!(
            result,
            Err(CarrierError::Bounds(Bounds::Bound("whole_record")))
        ));
        let raw = envelope(&expected_payload(header, &component)?, 2)?;
        assert_eq!(raw.len(), MAX + 1);
        assert!(matches!(
            decode_owned_continuation(&key(header)?, header.run(), &raw, &mut budget()?),
            Err(CarrierError::Bounds(Bounds::Bound("whole_record")))
        ));
    }
    Ok(())
}

#[test]
fn outer_exact_allocation_work_one_less_and_hard_caps() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    let exact_allocation = 3 * MAX + 512;
    let exact_work = 4 * MAX + 1024;
    encode_owned_continuation(
        header,
        &component,
        &mut Budget::new(MAX, exact_allocation, exact_work)?,
    )?;
    for (allocation, work, category) in [
        (exact_allocation - 1, WORK, "allocation_bytes"),
        (ALLOCATION, exact_work - 1, "work"),
    ] {
        let error =
            encode_owned_continuation(header, &component, &mut Budget::new(MAX, allocation, work)?);
        assert!(
            matches!(error, Err(CarrierError::Bounds(Bounds::Budget(value))) if value == category)
        );
    }
    assert!(Budget::new(MAX + 1, ALLOCATION, WORK).is_err());
    assert!(Budget::new(MAX, ALLOCATION + 1, WORK).is_err());
    assert!(Budget::new(MAX, ALLOCATION, WORK + 1).is_err());
    assert!(Budget::new(0, ALLOCATION, WORK).is_err());
    Ok(())
}

#[test]
fn combined_exact_and_one_less_charge_precede_strong_decode() -> TestResult {
    let (_, authority, component, header) = fixture(false, 3)?;
    let record = encode_owned_continuation(header, &component, &mut budget()?)?;
    let (_, usage) = RunContinuationV1::from_transport_bytes(
        &component,
        &authority,
        ContinuationTransportLimits::new(component.len(), ALLOCATION, WORK)?,
    )?;
    for field in ["allocation_bytes", "work"] {
        for less in [0_usize, 1] {
            assert_combined(
                &component,
                header,
                record.as_bytes(),
                &authority,
                usage,
                field,
                less,
            )?;
        }
    }
    Ok(())
}

fn assert_combined(
    component: &[u8],
    header: vb_storage::ContinuationCarrierHeader,
    record: &[u8],
    authority: &vb_runtime::run_continuation::ContinuationAuthority,
    usage: vb_storage::CarrierUsage,
    field: &str,
    less: usize,
) -> TestResult {
    let allocation = (3 * MAX + 512 + usage.allocation_bytes)
        - if field == "allocation_bytes" { less } else { 0 };
    let work = (4 * MAX + 1024 + usage.work) - if field == "work" { less } else { 0 };
    let mut account = Budget::new(MAX, allocation, work)?;
    let view = decode_owned_continuation(&key(header)?, header.run(), record, &mut account)?;
    let allowance = account.component_allowance()?;
    let (a, w) = allowance.limits();
    let result = RunContinuationV1::from_transport_bytes(
        view.component(),
        authority,
        ContinuationTransportLimits::new(component.len(), a, w)?,
    );
    absorb_result(result, allowance, field, less)?;
    assert!(account.usage().allocation_bytes <= allocation);
    assert!(account.usage().work <= work);
    Ok(())
}

fn absorb_result(
    result: Result<
        (RunContinuationV1, vb_storage::CarrierUsage),
        vb_runtime::run_continuation::ContinuationTransportError,
    >,
    allowance: vb_storage::ComponentAllowance<'_>,
    field: &str,
    less: usize,
) -> TestResult {
    match result {
        Ok((_, consumed)) => {
            assert_eq!(less, 0);
            allowance.absorb(consumed)?;
        }
        Err(error) => {
            assert_eq!(less, 1);
            assert!(
                matches!(error.failure, TransportFailure::Bounds(Bounds::Budget(value)) if value == field)
            );
            assert!(!error.strong_decode_started);
            allowance.absorb(error.usage)?;
        }
    }
    Ok(())
}

#[test]
fn allowance_is_single_use_and_abandonment_closes_account() -> TestResult {
    let mut account = budget()?;
    {
        let allowance = account.component_allowance()?;
        assert_eq!(
            allowance.limits(),
            (ALLOCATION - (3 * MAX + 512), WORK - (4 * MAX + 1024))
        );
    }
    assert!(matches!(
        account.component_allowance(),
        Err(CarrierError::AccountClosed)
    ));
    assert!(matches!(
        account.reserve_outer(),
        Err(CarrierError::AccountClosed)
    ));
    let mut account = budget()?;
    let allowance = account.component_allowance()?;
    assert!(
        allowance
            .absorb(vb_storage::CarrierUsage {
                allocation_bytes: ALLOCATION,
                work: 0
            })
            .is_err()
    );
    assert!(matches!(
        account.reserve_outer(),
        Err(CarrierError::AccountClosed)
    ));
    Ok(())
}

#[test]
fn real_owned_blob_varint_boundaries_preserve_exact_whole_record_caps() -> TestResult {
    for blob in [127_usize, 128, 16_383, 16_384] {
        let (_, _, component, header) = fixture(false, blob)?;
        let expected = envelope(&expected_payload(header, &component)?, 2)?;
        let mut exact = Budget::new(expected.len(), ALLOCATION, WORK)?;
        let actual = encode_owned_continuation(header, &component, &mut exact)?;
        assert_eq!(actual.as_bytes(), expected);
        let mut small = Budget::new(expected.len() - 1, ALLOCATION, WORK)?;
        assert!(matches!(
            encode_owned_continuation(header, &component, &mut small),
            Err(CarrierError::Bounds(Bounds::Bound("whole_record")))
        ));
    }
    Ok(())
}
