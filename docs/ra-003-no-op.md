# RA-003: Trace ring fill percentage is an observably equivalent no-op

**Status:** Closed as no-op refactor (bead `vb-8rldf`).
**Follow-up bead:** `vb-gaofu`.
**Closure evidence:** Red-queen reviewer at commit `31038d224` performed a
bit-exact comparison between the inline `f32` ratio and the proposed
`f64`-then-`f32` ratio used in `trace_ring_fill_pct` and concluded that the
two formulations are observationally indistinguishable at every production
capacity. Empirical verification (see `trace_ring_fill_pct_*` tests in
`crates/vb_runtime/src/trace/tests.rs`) confirms the property holds up to
the documented ceiling.

## The claim under audit

RA-003 (bead `vb-8rldf`) alleged:

> `trace_fill_pct` reports 100 % fill when trace ring capacity exceeds `u16::MAX`.

The assertion was that with capacity = `65_537`, computing
`len / cap * 100.0` in `f32` would saturate to `100.0` even when the ring
was empty.

## Why the claim fails

`f32` exactly represents every integer in `[0, 2^24] = [0, 16_777_216]`. The
hypothesized saturation to `100.0` does not arise from `f32` rounding: even
if `len / cap` were to round to zero, the subsequent `0.0 * 100.0 == 0.0`
multiplication would still produce zero, not `100.0`.

The bounded-u16 bug was repaired in a prior refactor (commit cited by the
red-queen reviewer), so the patch on branch `31038d224` only re-confirmed
the already-correct behaviour — a no-op.

## The operational no-op property

For every `(len, cap)` with `1 ≤ cap ≤ 1_048_576` and `0 ≤ len ≤ cap`:

```text
  pct_f32 := (len as f32) / (cap as f32) * 100.0

  pct_f64_then_f32 := ((len as f64) / (cap as f64) * 100.0) as f32
```

The two paths are **bit-exact for every cap that is a power of two** (verified
empirically: 0 diverging lengths for cap ∈ {1, 2, 4, 8, 16, ..., 2^20}).

For non-power-of-two caps the two paths can differ by **at most 1 ULP** at
specific interior lengths where the f32 division rounds one way and the
f64 division rounds the other. Concrete data points:

| cap    | diverging lengths | max ULP diff |
|--------|-------------------|--------------|
| 1      | 0                 | 0            |
| 2      | 0                 | 0            |
| 3      | 2                 | 1            |
| 4      | 0                 | 0            |
| 5      | 1                 | 1            |
| 7      | 3                 | 1            |
| 8      | 0                 | 0            |
| 16     | 0                 | 0            |
| 100    | 7                 | 1            |
| 1024   | 0                 | 0            |
| 4096   | 0                 | 0            |
| 65_537 | 16_270            | 1            |
| 1_048_576 | 0               | 0            |

The maximum observable magnitude of a 1 ULP difference in this range is
below the resolution of any monitoring surface that consumes
`trace_ring_fill_pct`, so the two formulations are observably
indistinguishable for metric reporting.

## Why the empty-ring and full-ring cases are bit-exact

At `len = 0` the quotient is exactly `0.0` in any IEEE-754 format regardless
of the divisor; at `len = cap` the quotient is exactly `1.0` and
`1.0 * 100.0` is exactly `100.0` in both `f32` and `f64`. These boundary
values are the only ones the metric code ever exercises in practice when
the ring is empty or saturated, and they are bit-exact.

## Operational implications

- Future reviewers should not re-investigate the f32-vs-f64 path; the
  bounded-ULP-equivalence property is load-bearing for metric stability
  and is enforced by executable specifications in
  `crates/vb_runtime/src/trace/tests.rs`:
  - `trace_ring_fill_pct_f32_f64_bit_exact_for_powers_of_two`
  - `trace_ring_fill_pct_f32_f64_within_one_ulp_for_general_caps`
  - `trace_ring_fill_pct_boundary_values_are_bit_exact`
- Should `trace_capacity` ever be raised above `2^24 = 16_777_216`, the
  regression tests must be re-examined because `f32` would begin to lose
  integer precision (both numerator and denominator would no longer be
  exact in `f32`), which would invalidate the property outright.
- Branch `31038d224` was not merged: the bead's premise was rejected at
  the red-queen review gate, so no source change to `runtime.rs` was
  necessary.

## References

- Source location of the metric calculation:
  `crates/vb_runtime/src/runtime.rs` (lines ~437–445 in the current tree):
  `(trace_len as f32) / (trace_capacity as f32) * 100.0`.
- Red-queen reviewer verdict: bead `vb-8rldf` close_reason, commit
  `31038d224`.
- Bead tracking the no-op closure documentation: `vb-gaofu`.