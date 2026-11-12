#![forbid(unsafe_code)]

//! Unbiased delay jitter for retry scheduling.
//!
//! Jitter breaks the synchronization between concurrent retryers that would
//! otherwise wake on the same tick and stampede a recovering dependency.
//! The reduction maps a random `u64` onto a delay span with a wide-multiply
//! (`(random * span) >> 64`) step rather than a modulo, which stays uniform
//! without floating point.

use rand::Rng;

/// Maps a random `u64` uniformly into `0..span`.
///
/// `span == 0` yields `0`. The high half of the 128-bit product is uniform
/// over `0..span` up to one part in `2^64`, unlike `random % span`, which
/// over-weights low values whenever `span` does not divide `2^64`.
#[must_use]
pub fn scale_random(random: u64, span: u64) -> u64 {
    if span == 0 {
        return 0;
    }
    // A `u64 * u64` product always fits in a `u128`, so the checked multiply
    // never hits its `None` arm; it keeps the operation free of unchecked
    // arithmetic. The same holds for the narrowing conversion below: the high
    // half of a 128-bit product is always representable as a `u64`.
    let high = u128::from(random)
        .checked_mul(u128::from(span))
        .map_or(0, |product| product >> 64);
    u64::try_from(high).map_or(u64::MAX, |scaled| scaled)
}

/// Returns a delay in `[lower, upper)` drawn from `rng`.
///
/// `upper <= lower` returns `lower`: an empty span admits a single delay.
pub fn jitter_delay(lower: u64, upper: u64, rng: &mut impl Rng) -> u64 {
    jitter_delay_from_random(lower, upper, rng.next_u64())
}

/// Deterministic core of [`jitter_delay`], for callers that already hold a
/// random sample and for tests that must not depend on RNG behavior.
#[must_use]
pub fn jitter_delay_from_random(lower: u64, upper: u64, random: u64) -> u64 {
    let span = upper.saturating_sub(lower);
    if span == 0 {
        return lower;
    }
    lower.saturating_add(scale_random(random, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_random_zero_span_is_zero() {
        assert_eq!(scale_random(0, 0), 0);
        assert_eq!(scale_random(u64::MAX, 0), 0);
    }

    #[test]
    fn scale_random_stays_below_span() {
        assert_eq!(scale_random(0, 1_000), 0);
        assert!(scale_random(u64::MAX, 1_000) < 1_000);
        assert_eq!(scale_random(u64::MAX, 1_000), 999);
    }

    #[test]
    fn scale_random_uses_the_high_half_of_the_product() {
        // A span just above 2^63 makes the low half of the product non-zero,
        // so a reduction that kept low bits (or used `%`) would miss these
        // endpoints. Wide-multiply maps the extremes to the span extremes.
        let span = (1_u64 << 63) + 1;
        assert_eq!(scale_random(1, span), 0);
        assert_eq!(scale_random(u64::MAX, span), span - 1);
    }

    #[test]
    fn scale_random_near_full_width_fits_u64() {
        assert_eq!(scale_random(u64::MAX, u64::MAX), u64::MAX - 1);
    }

    #[test]
    fn jitter_delay_from_random_respects_bounds() {
        let cases = [
            (0_u64, 1_000_u64, 0_u64),
            (0, 1_000, u64::MAX),
            (500, 1_000, 0),
            (500, 1_000, u64::MAX),
            (7, 8, u64::MAX),
        ];
        for (lower, upper, random) in cases {
            let delay = jitter_delay_from_random(lower, upper, random);
            assert!(
                delay >= lower && delay < upper,
                "delay {delay} outside [{lower}, {upper})"
            );
        }
    }

    #[test]
    fn jitter_delay_from_random_empty_span_returns_lower() {
        assert_eq!(jitter_delay_from_random(250, 250, u64::MAX), 250);
        assert_eq!(jitter_delay_from_random(250, 100, u64::MAX), 250);
    }

    #[test]
    fn jitter_delay_from_random_single_value_span_is_empty() {
        assert_eq!(jitter_delay_from_random(0, 1, u64::MAX), 0);
        assert_eq!(jitter_delay_from_random(7, 8, 0), 7);
    }

    #[test]
    fn jitter_delay_has_no_modulo_skew() {
        // `3 * 2^62` is the case where modulo bias is measurable: `2^64` is
        // `4/3` spans, so `random % span` gives the lowest third two preimages
        // and one third of the range carries double weight. Wide-multiply is
        // even. Uniform expects ~1365 of 4096 draws in the lowest third;
        // modulo expects ~2048.
        let span = 3_u64 << 62;
        let third = 1_u64 << 62;
        let mut rng = rand::rng();
        let mut low_third = 0_u32;
        for _ in 0..4_096 {
            let delay = jitter_delay(0, span, &mut rng);
            assert!(delay < span);
            if delay < third {
                low_third = low_third.saturating_add(1);
            }
        }
        assert!(
            (1_200..=1_550).contains(&low_third),
            "lowest-third draws {low_third}/4096, distribution is skewed"
        );
    }

    #[test]
    fn jitter_delay_is_not_constant() {
        let mut rng = rand::rng();
        let values: Vec<u64> = (0..64).map(|_| jitter_delay(0, 1_000, &mut rng)).collect();
        let first = values.first().copied();
        assert!(
            values.iter().any(|value| Some(*value) != first),
            "every draw returned {first:?}"
        );
    }

    #[test]
    fn jitter_delay_draws_within_bounds() {
        let mut rng = rand::rng();
        let mut low_count = 0_u32;
        let mut high_count = 0_u32;
        for _ in 0..256 {
            let delay = jitter_delay(0, 100, &mut rng);
            assert!(delay < 100);
            if delay < 50 {
                low_count += 1;
            } else {
                high_count += 1;
            }
        }
        // Both halves must be reachable; a modulo-biased or constant result
        // would collapse one side for a 100-wide span.
        assert!(low_count > 0, "no draws below the midpoint");
        assert!(high_count > 0, "no draws above the midpoint");
    }
}
