use super::support::{TestResult, fixture};
use super::wire::{decode, encode, fixture_wire, redigest, wrap};
use vb_core::Taint;
use vb_core::root_bundle::OwnedRootBundleError;
use vb_storage::{JournalError, OwnedInputError, RecordKind};

#[test]
fn owned_version_kind_and_exact_consumption_fail_closed() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let mut wire = fixture_wire(&f)?;
    wire.body.version = 2;
    assert!(matches!(
        decode(&f, &encode(&wire, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(
            OwnedInputError::UnsupportedVersion { version: 2 }
        ))
    ));
    let wire = fixture_wire(&f)?;
    assert!(matches!(
        decode(&f, &encode(&wire, RecordKind::RunInputs)?),
        Err(JournalError::RecordKindPayloadMismatch {
            envelope_kind: 4,
            payload_kind: 42
        })
    ));
    let mut payload = postcard::to_allocvec(&wire)?;
    payload.push(0);
    assert!(matches!(
        decode(&f, &wrap(&payload, RecordKind::RunOwnedInputs, f.run)?),
        Err(JournalError::PostcardDecodeFailed(_))
    ));
    let malformed = wrap(&[1], RecordKind::RunOwnedInputs, f.run)?;
    assert!(matches!(
        decode(&f, &malformed),
        Err(JournalError::PostcardDecodeFailed(_))
    ));
    Ok(())
}

#[test]
fn outer_checksum_and_complete_inner_digest_are_both_required() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let mut bytes = encode(&fixture_wire(&f)?, RecordKind::RunOwnedInputs)?;
    *bytes.last_mut().ok_or("empty envelope")? ^= 1;
    assert!(matches!(
        decode(&f, &bytes),
        Err(JournalError::PayloadDigestMismatch)
    ));
    let mut wire = fixture_wire(&f)?;
    wire.body
        .checkpoint
        .blobs
        .first_mut()
        .ok_or("blob missing")?
        .clone_from(&vec![8u8].into());
    assert!(matches!(
        decode(&f, &encode(&wire, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(OwnedInputError::DigestMismatch))
    ));
    Ok(())
}

#[test]
fn self_consistent_forged_projection_and_cap_still_revalidate_bundle() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let mut projection = fixture_wire(&f)?;
    projection.body.resources.allows_secret_results =
        !projection.body.resources.allows_secret_results;
    redigest(&mut projection)?;
    assert!(matches!(
        decode(&f, &encode(&projection, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(
            OwnedInputError::ProjectionMismatch
        ))
    ));
    let mut cap = fixture_wire(&f)?;
    cap.body.checkpoint.max_arena_entries = cap
        .body
        .checkpoint
        .max_arena_entries
        .checked_add(1)
        .ok_or("cap overflow")?;
    redigest(&mut cap)?;
    assert!(matches!(
        decode(&f, &encode(&cap, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(OwnedInputError::Bundle(
            OwnedRootBundleError::CapMismatch { .. }
        )))
    ));
    Ok(())
}

#[test]
fn forged_owned_taint_is_rejected_by_the_real_constructor() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let mut taint = fixture_wire(&f)?;
    let root = taint
        .body
        .roots
        .iter_mut()
        .find(|(_, value, _)| matches!(value, vb_core::SlotValue::List(_)))
        .ok_or("list root missing")?;
    root.2 = Taint::Clean;
    redigest(&mut taint)?;
    assert!(matches!(
        decode(&f, &encode(&taint, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(OwnedInputError::Bundle(
            OwnedRootBundleError::TaintMismatch { .. }
        )))
    ));
    Ok(())
}

#[test]
fn forged_owned_reference_is_rejected_by_the_real_constructor() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let mut reference = fixture_wire(&f)?;
    let item = reference
        .body
        .checkpoint
        .lists
        .first_mut()
        .ok_or("list missing")?
        .first_mut()
        .ok_or("item missing")?;
    *item = vb_core::SlotValue::List(vb_core::ListId::new(99));
    redigest(&mut reference)?;
    assert!(matches!(
        decode(&f, &encode(&reference, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(OwnedInputError::Bundle(
            OwnedRootBundleError::Checkpoint(_)
        )))
    ));
    Ok(())
}

#[test]
fn decoded_roots_must_already_be_in_canonical_order() -> TestResult {
    let f = fixture(7, Taint::DerivedFromSecret, Taint::Secret)?;
    let mut order = fixture_wire(&f)?;
    order.body.roots.reverse();
    redigest(&mut order)?;
    assert!(matches!(
        decode(&f, &encode(&order, RecordKind::RunOwnedInputs)?),
        Err(JournalError::OwnedInputs(
            OwnedInputError::NonCanonicalRoots
        ))
    ));
    Ok(())
}
