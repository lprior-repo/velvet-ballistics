#![forbid(unsafe_code)]
//! Wall-clock-based unique run identifier generation.
//!
//! `RunId` values are derived from nanoseconds since the UNIX epoch. A
//! monotonic best-effort guarantee is provided by the underlying OS clock:
//! on Linux x86_64 the `clock_gettime(CLOCK_REALTIME)` source resolves at
//! nanosecond granularity, and consecutive back-to-back invocations
//! within a single process are expected to yield distinct values. Should
//! the clock be unavailable (pre-1970 system time, or a platform whose
//! `SystemTime` panics on underflow), the function falls back to
//! `RunId::new(0)`.

/// Generates a unique [`RunId`] from the system clock (nanoseconds since
/// `UNIX_EPOCH`).
///
/// Falls back to `RunId::new(0)` if the clock is unavailable (e.g. system
/// time predates the UNIX epoch) or if the resulting nanosecond duration
/// does not fit in a `u64`.
///
/// [`RunId`]: vb_core::RunId
#[must_use]
pub(crate) fn generate_run_id_from_clock() -> vb_core::RunId {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| u64::try_from(d.as_nanos()).ok())
        .unwrap_or(0);
    vb_core::RunId::new(nanos)
}

#[cfg(test)]
mod tests {
    use super::generate_run_id_from_clock;

    #[test]
    fn run_id_returns_unique_values_for_consecutive_calls() {
        let id1 = generate_run_id_from_clock();
        let id2 = generate_run_id_from_clock();
        assert_ne!(id1, id2, "RunIds must be unique across invocations");
    }

    #[test]
    fn run_id_returns_distinct_values_for_1000_calls() {
        let ids: Vec<u64> = (0..1000)
            .map(|_| generate_run_id_from_clock().get())
            .collect();
        let unique: std::collections::HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "All RunIds must be distinct");
    }

    #[test]
    fn run_id_uses_unix_epoch_nanoseconds_not_zero() {
        let id = generate_run_id_from_clock().get();
        assert!(
            id > 1_000_000_000_000_000_000,
            "RunId must be a real nanosecond timestamp, got {id}"
        );
    }
}
