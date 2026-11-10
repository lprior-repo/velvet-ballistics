use super::*;

/// Hash a `ScalarValue`, failing closed on future unknown variants.
pub(super) fn digest_scalar_value(
    hasher: &mut blake3::Hasher,
    value: &crate::ScalarValue,
) -> Result<(), CompileErrors> {
    match value {
        crate::ScalarValue::String(s) => {
            hasher.update(&[0x01]);
            digest_string(hasher, s)
        }
        crate::ScalarValue::Integer(n) => {
            hasher.update(&[0x02]);
            digest_i64(hasher, *n);
            Ok(())
        }
        _ => Err(digest_error(
            "unknown scalar value variant cannot be canonicalized",
        )),
    }
}
