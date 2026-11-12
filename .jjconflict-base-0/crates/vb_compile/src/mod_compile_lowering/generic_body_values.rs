//! Total calculations shared by native lowering and the direct Verus inclusion.
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_verify)]
#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_spec(out =>
    ensures out == match candidate { Some(value) => Some(value), None => previous },
))]
pub(crate) fn keep_last<T>(previous: Option<T>, candidate: Option<T>) -> Option<T> {
    match candidate {
        Some(value) => Some(value),
        None => previous,
    }
}

#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_verify)]
#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_spec(out =>
    ensures out == match input {
        Err(error) => Err(error), Ok(None) => Ok(None),
        Ok(Some((slot, _))) => Ok(Some(slot)),
    },
))]
pub(crate) fn strip_type<S, T, E>(input: Result<Option<(S, T)>, E>) -> Result<Option<S>, E> {
    match input {
        Err(error) => Err(error),
        Ok(None) => Ok(None),
        Ok(Some((slot, _))) => Ok(Some(slot)),
    }
}

#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_verify)]
#[cfg_attr(verus_keep_ghost, vstd::prelude::verus_spec(out =>
    ensures match out {
        Some(value) => value as int == left as int + right as int,
        None => left as int + right as int > usize::MAX as int,
    },
))]
pub(crate) fn checked_sum(left: usize, right: usize) -> Option<usize> {
    left.checked_add(right)
}
