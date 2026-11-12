use super::support::*;
use vb_core::RunId;
use vb_storage::EventSeq;
use vb_storage::{CarrierError, ContinuationCarrierHeader as Header, decode_owned_continuation};

#[test]
fn every_scalar_version_identity_and_component_digest_is_checked() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    let payload = expected_payload(header, &component)?;
    for (offset, value) in [
        (0, 44),
        (1, 2),
        (2, 0),
        (3, 3),
        (4, 0),
        (5, 0),
        (6, 2),
        (7, 99),
    ] {
        let mut forged = payload.clone();
        *forged.get_mut(offset).ok_or("scalar")? = value;
        let raw = envelope(&forged, 2)?;
        assert!(
            decode_owned_continuation(&key(header)?, header.run(), &raw, &mut budget()?).is_err()
        );
    }
    Ok(())
}

#[test]
fn opaque_component_digest_and_reserved_constructor_values_are_typed() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    let mut bad_component = component.clone();
    *bad_component.last_mut().ok_or("component")? ^= 1;
    let raw = envelope(&expected_payload(header, &bad_component)?, 2)?;
    assert!(matches!(
        decode_owned_continuation(&key(header)?, header.run(), &raw, &mut budget()?),
        Err(CarrierError::Digest)
    ));
    assert!(
        Header::new(
            RunId::new(0),
            EventSeq::new(2),
            1,
            EventSeq::new(1),
            [0; 32]
        )
        .is_err()
    );
    assert!(Header::new(header.run(), EventSeq::new(2), 0, EventSeq::new(1), [0; 32]).is_err());
    assert!(
        Header::new(
            header.run(),
            EventSeq::MAX,
            1,
            EventSeq::new(u64::MAX - 1),
            [0; 32]
        )
        .is_err()
    );
    assert!(Header::new(header.run(), EventSeq::new(0), 1, EventSeq::MAX, [0; 32]).is_err());
    Ok(())
}

#[test]
fn huge_declared_length_overlong_and_trailing_refuse_without_owning_copy() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    let payload = expected_payload(header, &component)?;
    let prefix = payload.get(..39).ok_or("fixed scalar/digest prefix")?;
    let mut huge = prefix.to_vec();
    huge.extend_from_slice(&[255, 255, 255, 255, 255, 255, 255, 255, 255, 1]);
    let raw = envelope(&huge, 2)?;
    let mut operation = budget()?;
    assert!(matches!(
        decode_owned_continuation(&key(header)?, header.run(), &raw, &mut operation),
        Err(CarrierError::Bounds(
            vb_core::value_store::transport::CheckpointTransportError::Bound("opaque_bytes")
        ))
    ));
    assert_eq!(
        operation.usage(),
        vb_storage::CarrierUsage {
            allocation_bytes: 3 * MAX + 512,
            work: 4 * MAX + 1024,
        }
    );
    let mut overlong = vec![0xab, 0];
    overlong.extend_from_slice(payload.get(1..).ok_or("tag")?);
    let mut trailing = payload.clone();
    trailing.push(0);
    for hostile in [overlong, trailing] {
        let raw = envelope(&hostile, 2)?;
        assert!(
            decode_owned_continuation(&key(header)?, header.run(), &raw, &mut budget()?).is_err()
        );
    }
    Ok(())
}

#[test]
fn exact_key_envelope_checksum_kind_and_all_truncations_are_mandatory() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    let raw = envelope(&expected_payload(header, &component)?, 2)?;
    for end in 0..raw.len() {
        assert!(
            decode_owned_continuation(
                &key(header)?,
                header.run(),
                raw.get(..end).ok_or("prefix")?,
                &mut budget()?
            )
            .is_err()
        );
    }
    for offset in [0_usize, 4, 6, 8, 12, 16, 24, 56, 60] {
        let mut hostile = raw.clone();
        *hostile.get_mut(offset).ok_or("header")? ^= 1;
        assert!(
            decode_owned_continuation(&key(header)?, header.run(), &hostile, &mut budget()?)
                .is_err()
        );
    }
    let wrong_key = vb_storage::keys::run_event_key(header.run(), EventSeq::new(3))?;
    assert!(decode_owned_continuation(&wrong_key, header.run(), &raw, &mut budget()?).is_err());
    assert!(
        decode_owned_continuation(&key(header)?, RunId::new(18), &raw, &mut budget()?).is_err()
    );
    let wrong_seq = envelope(&expected_payload(header, &component)?, 3)?;
    assert!(
        decode_owned_continuation(&key(header)?, header.run(), &wrong_seq, &mut budget()?).is_err()
    );
    Ok(())
}

#[test]
fn explicit_versions_and_opaque_body_hash_fail_with_exact_typed_causes() -> TestResult {
    let (_, _, component, header) = fixture(false, 3)?;
    for (offset, field) in [(1_usize, "carrier"), (6, "component")] {
        let mut payload = expected_payload(header, &component)?;
        *payload.get_mut(offset).ok_or("version")? = 2;
        let raw = envelope(&payload, 2)?;
        assert!(
            matches!(decode_owned_continuation(&key(header)?, header.run(), &raw,
            &mut budget()?), Err(CarrierError::Version { field: found, version: 2 }) if found == field)
        );
    }
    let mut corrupt = component.clone();
    *corrupt.get_mut(1).ok_or("body")? ^= 1;
    let raw = envelope(&expected_payload(header, &corrupt)?, 2)?;
    assert!(matches!(
        decode_owned_continuation(&key(header)?, header.run(), &raw, &mut budget()?),
        Err(CarrierError::Digest)
    ));
    Ok(())
}
