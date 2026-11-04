use super::{GrantSignature, HistoricalGrantSignature, MAX_GRANT_SIGNATURE_LEN};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn require_copy_hash<T: Copy + std::hash::Hash>(value: T) -> T {
    value
}

#[test]
fn fixed_signature_preserves_64_byte_api_and_postcard_shape() -> TestResult {
    let mut bytes = [0_u8; 64];
    for (index, slot) in bytes.iter_mut().enumerate() {
        *slot = u8::try_from(index)?;
    }
    let signature = require_copy_hash(GrantSignature::from_bytes(bytes));
    let copied = signature;
    assert_eq!(signature.as_bytes(), bytes);
    assert_eq!(copied.as_bytes(), bytes);
    let encoded = postcard::to_allocvec(&signature)?;
    assert_eq!(encoded, bytes.to_vec());
    assert_eq!(postcard::from_bytes::<GrantSignature>(&encoded)?, signature);
    let mut expected_some = vec![1_u8];
    expected_some.extend_from_slice(&bytes);
    assert_eq!(postcard::to_allocvec(&Some(signature))?, expected_some);
    assert_eq!(
        postcard::from_bytes::<Option<GrantSignature>>(&expected_some)?,
        Some(signature)
    );
    Ok(())
}

#[test]
fn historical_signature_preserves_three_byte_some_postcard_shape() -> TestResult {
    let signature = HistoricalGrantSignature::new(Box::new([3_u8, 5, 8]))
        .ok_or("historical three-byte signature must be accepted")?;
    assert!(signature.is_valid());
    assert_eq!(signature.bytes.as_ref(), &[3_u8, 5, 8]);
    let expected_some = vec![1_u8, 3, 3, 5, 8];
    assert_eq!(
        postcard::to_allocvec(&Some(signature.clone()))?,
        expected_some
    );
    assert_eq!(
        postcard::from_bytes::<Option<HistoricalGrantSignature>>(&expected_some)?,
        Some(signature)
    );
    Ok(())
}

#[test]
fn historical_signature_rejects_empty_and_overbound_lengths() -> TestResult {
    assert!(HistoricalGrantSignature::new(Box::new([])).is_none());
    let overbound = MAX_GRANT_SIGNATURE_LEN
        .checked_add(1)
        .ok_or("historical signature length bound must permit one larger length")?;
    assert!(HistoricalGrantSignature::new(vec![1_u8; overbound].into_boxed_slice()).is_none());
    assert!(
        HistoricalGrantSignature::new(vec![1_u8; MAX_GRANT_SIGNATURE_LEN].into_boxed_slice())
            .is_some()
    );
    Ok(())
}
