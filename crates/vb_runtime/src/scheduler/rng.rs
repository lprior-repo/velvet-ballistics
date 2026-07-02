#![forbid(unsafe_code)]
//! Deterministic splitmix64 PRNG used by the seeded scheduler.
//!
//! `rand::thread_rng()` is intentionally **not** used because the
//! scheduler facade must produce byte-identical transcripts for a given
//! seed. splitmix64 is the canonical "good enough" deterministic PRNG
//! for replay-style exploration; it is fast, has no allocation, no
//! thread-local state, and is publicly documented by Steele, Lea &
//! Flood (2014).
//!
//! Reference: Steele Jr., G. L., Lea, D., & Flood, C. H. (2014).
//! "Fast splittable pseudorandom number generators." OOPSLA '14.

/// Deterministic splitmix64 PRNG state.
///
/// All public methods are `const`-friendly and allocation-free.
/// The PRNG is single-threaded by design — the scheduler facade holds
/// one instance per [`crate::scheduler::SeededScheduler`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RngState {
    state: u64,
}

impl RngState {
    /// Mix constant from Steele/Lea/Flood (2014), splitmix64.
    const MIX_CONST: u64 = 0x9E37_79B9_7F4A_7C15;

    /// Creates a new PRNG seeded with the supplied value.
    ///
    /// Any `u64` is a valid seed. Seeds of `0` are valid (splitmix64
    /// will produce a non-trivial sequence from the zero state).
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next `u64` from the stream and advances the state.
    ///
    /// Each call mixes the state via the standard splitmix64 round
    /// (`x += MIX_CONST; x = (x ^ (x >> 30)) * ...;`) producing a
    /// uniformly distributed 64-bit value.
    ///
    /// Note: splitmix64 relies on **wrapping** u64 multiplication,
    /// not checked multiplication. The middle multiplications
    /// (e.g. `0xBF58_476D_1CE4_E5B9 * z`) overflow u64 by design;
    /// the modular wrap is what gives splitmix64 its full period.
    /// We therefore use `wrapping_mul` here — checked_mul would
    /// saturate to zero and break the algorithm.
    ///
    /// This is the one Holzman-checked-arithmetic escape hatch in
    /// the scheduler facade. It is bounded: the wrapping result
    /// is always a valid `u64`, never an invalid value or a panic.
    #[must_use]
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64 round: see Steele/Lea/Flood (2014), algorithm 1.
        let advanced = self.state.wrapping_add(Self::MIX_CONST);
        self.state = advanced;
        let mut z = advanced;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns the next `u32` from the stream, discarding the lower
    /// 32 bits of the splitmix64 output.
    ///
    /// Suitable for selecting one of N options where N <= u32::MAX.
    #[must_use]
    pub fn next_u32(&mut self) -> u32 {
        // Explicit byte decomposition: the upper 32 bits of the
        // splitmix64 draw are the most significant four bytes in
        // big-endian order (`bytes[0..=3]`). Building the `u32`
        // from those bytes is provably loss-free (the slice length
        // is statically 4) and avoids the lossy `as`/`try_from`
        // ladder.
        let bytes = self.next_u64().to_be_bytes();
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    /// Returns the next value in `[0, bound)` for `bound > 0`.
    ///
    /// `bound == 0` returns `0` rather than panicking.
    #[must_use]
    pub fn next_bounded(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        // Use the full 64-bit draw to avoid modulo bias on small
        // bounds; `next_u64` is already uniform over [0, 2^64).
        let raw = self.next_u64();
        let bound_u64 = u64::from(bound);
        // `raw` is uniform over [0, 2^64); `raw % bound` is in [0, bound).
        // Modulo bias is bounded by 2^64 / bound which is acceptable
        // for our replay-style exploration.
        //
        // `bound_u64 > 0` is checked above, so `checked_rem` cannot
        // divide by zero and the result is in `[0, bound_u64) <=
        // [0, u32::MAX]`. We use `checked_rem` rather than `%` or
        // `wrapping_rem` to make the explicit "modulo with
        // provably-non-zero divisor" contract visible to clippy's
        // `arithmetic_side_effects` lint, and we avoid the
        // `as u32` narrowing by reconstructing the lower 32 bits
        // via little-endian byte decomposition (the upper 32 bits
        // are guaranteed zero because `result < bound_u64 <=
        // u32::MAX`).
        //
        // The `match` pattern explicitly acknowledges the
        // structurally-unreachable fallback. Clippy's
        // `manual_unwrap_or_default` lint would suggest
        // `.unwrap_or_default()` instead, but `unwrap_used =
        // "forbid"` rejects that form at the workspace level;
        // the explicit `match` is therefore the right shape for
        // this codebase. The `#[allow(...)]` on the binding
        // suppresses the suggestion without weakening any other
        // lint.
        #[allow(clippy::manual_unwrap_or, clippy::manual_unwrap_or_default)]
        let result: u64 = match raw.checked_rem(bound_u64) {
            Some(v) => v,
            // `bound_u64 > 0`, so `checked_rem` always returns
            // `Some`. The `None` arm is structurally unreachable;
            // we pick 0 as the documented fallback.
            None => 0,
        };
        let lo = result.to_le_bytes();
        u32::from_le_bytes([lo[0], lo[1], lo[2], lo[3]])
    }

    /// Returns the current raw state (mainly for diagnostics/tests).
    #[must_use]
    pub const fn raw_state(&self) -> u64 {
        self.state
    }
}
