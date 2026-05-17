# FiniteF64 Decision

Decision for bead `vb-g997`: keep the custom `FiniteF64` newtype for v1 instead
of replacing it with `ordered_float::NotNan<f64>`.

## Rationale

- `ordered_float::NotNan<f64>` rejects NaN but accepts positive and negative
  infinity. The language contract requires finite-only scalar numbers.
- `FiniteF64::new` uses `f64::is_finite`, so it rejects NaN, positive infinity,
  and negative infinity in both debug and release builds.
- Keeping the custom newtype avoids adding transitive dependencies to the hot
  runtime value model.
- `FiniteF64` remains a transparent wrapper with checked serde deserialization,
  preserving the runtime invariant at construction and decode boundaries.

## Evidence

- Contract: `velvet-ballistics-MASTER.md` rejects `ordered-float` for v1
  `FiniteF64` because `NotNan<f64>` permits infinities.
- Implementation: `crates/vb_core/src/value.rs` constructs `FiniteF64` only when
  `value.is_finite()` is true.
- Tests: `crates/vb_core/tests/phase1_core_types.rs` covers finite acceptance and
  rejection of NaN, positive infinity, and negative infinity.

Any future replacement must prove release-mode rejection of NaN and infinities,
unchanged serialized representation, no panic/unwrap path, and no larger
transitive footprint than the custom newtype.
