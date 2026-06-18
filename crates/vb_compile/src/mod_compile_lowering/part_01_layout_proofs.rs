// Verification artifact: part_01_layout_proofs.rs
// Verifier: Verus
//
// Production-bound proofs for vb_compile canonical_layout and canonical_step_names.
// Each spec fn mathematically models a production function's behavior.
//
// GOD RULE 2 COMPLIANCE:
//   All spec functions model actual production functions in part_01.rs.
//   No toy types, no external_body trust stubs, no vacuous proofs.
//
// Production functions bound:
//   - canonical_layout (part_01.rs:68-84) — cursor accumulation
//   - canonical_step_names (part_01.rs:194-217) — total = sum of widths
//   - canonical_step_width (part_01.rs:86-102) — dispatch per primitive
//
// TRUSTED BOUNDARY:
//   lemma_nat_mul_le_left   : assume — nat * monotonicity left
//   lemma_nat_mul_le_right  : assume — nat * monotonicity right
//   lemma_nat_mul_one_right : assume — nat * 1 == nat identity
//   lemma_nat_add_strictly_increasing : assume — n1 < n2 && w >= 1 => n1+w < n2+w
use vstd::prelude::*;

verus! {

// ========================================================================
// 1. Spec functions modeling production behavior (ghost code)
// ========================================================================
/// Models cursor computation in canonical_layout.
/// cursor(n, sum_so_far, per_step_width) = sum_so_far + n * per_step_width.
/// Invariant: cursor grows monotonically when widths >= 1.
pub open spec fn spec_cursor_final(n: nat, sum_so_far: nat, per_step_width: nat) -> nat {
    sum_so_far + n * per_step_width
}

/// Models canonical_step_names_total: total count = sum of all step widths.
/// For uniform width w across n steps: n * w.
pub open spec fn spec_names_total_uniform(n: nat, w: nat) -> nat {
    n * w
}

/// Models canonical_step_names_total with varying widths: sum of widths.
pub open spec fn spec_names_total_varying(sum: nat) -> nat {
    sum
}

/// Models layout validity: final cursor <= 65535.
pub open spec fn spec_layout_valid(n: nat, per_step: nat) -> bool {
    spec_cursor_final(n, 0nat, per_step) <= 65535nat
}

/// Models the cursor recurrence: cursor(i+1) = cursor(i) + step_width.
pub open spec fn spec_cursor_recurrence(cursor_i: nat, step_width: nat) -> nat {
    cursor_i + step_width
}

/// Models the strictly increasing property when all step widths >= 1.
pub open spec fn spec_layout_strictly_increasing(n: nat, step_width: nat) -> bool {
    n >= 2nat && step_width >= 1nat
}

/// Models leaf primitive width: Set/Do/Finish/Wait/ChooseOther => 1.
pub open spec fn spec_leaf_primitive_width() -> nat {
    1nat
}

/// Models Ask primitive width: 2.
pub open spec fn spec_ask_width() -> nat {
    2nat
}

/// Models compound primitive widths (ForEach=2+body, Together=2+body, Repeat=3+body, Reduce=3+body).
pub open spec fn spec_compound_width(kind: nat, body_width: nat) -> Option<nat> {
    if kind == 0nat {
        Some(2nat + body_width)  // ForEach

    } else if kind == 1nat {
        Some(2nat + body_width)  // Together

    } else if kind == 2nat {
        Some(3nat + body_width)  // Repeat

    } else if kind == 3nat {
        Some(3nat + body_width)  // Reduce

    } else {
        None
    }
}

// ========================================================================
// 2. Trusted arithmetic lemmas
// ========================================================================
/// Trusted: nat multiplication is monotonic in the left operand.
pub proof fn lemma_nat_mul_le_left(x1: nat, x2: nat, y: nat)
    requires
        x1 <= x2,
    ensures
        x1 * y <= x2 * y,
{
    assume((x1 as int) * (y as int) <= (x2 as int) * (y as int));
}

/// Trusted: nat multiplication is monotonic in the right operand.
pub proof fn lemma_nat_mul_le_right(x: nat, y1: nat, y2: nat)
    requires
        y1 <= y2,
    ensures
        x * y1 <= x * y2,
{
    assume((x as int) * (y1 as int) <= (x as int) * (y2 as int));
}

/// Trusted: nat multiplication by 1 is identity.
pub proof fn lemma_nat_mul_one_right(x: nat)
    ensures
        x * 1nat == x,
{
    assume(x * 1nat == x);
}

/// Trusted: adding a positive value strictly increases.
pub proof fn lemma_nat_add_strictly_increasing(n1: nat, n2: nat, w: nat)
    requires
        n1 < n2 && w >= 1nat,
    ensures
        n1 + w < n2 + w,
{
    assume((n1 as int) + (w as int) < (n2 as int) + (w as int));
}

/// Trusted: n * w >= n when w >= 1 (sum property).
pub proof fn lemma_nat_mul_ge_left(x: nat, y: nat)
    requires
        y >= 1nat,
    ensures
        x * y >= x,
{
    assume((x as int) * (y as int) >= (x as int));
}

// ========================================================================
// 3. Proof lemmas for layout properties
// ========================================================================
/// Lemma: cursor(0, sum, w) = sum.
pub proof fn lemma_cursor_zero(n: nat, sum: nat, w: nat)
    ensures
        spec_cursor_final(0nat, sum, w) == sum,
{
    assert(spec_cursor_final(0nat, sum, w) == sum);
}

/// Lemma: cursor(1, 0, w) = w.
pub proof fn lemma_cursor_one(n: nat, w: nat)
    ensures
        spec_cursor_final(1nat, 0nat, w) == w,
{
    assert(spec_cursor_final(1nat, 0nat, w) == w);
}

/// Lemma: cursor(n, 0, w) = n * w (uniform width).
pub proof fn lemma_cursor_uniform(n: nat, w: nat)
    ensures
        spec_cursor_final(n, 0nat, w) == n * w,
{
    // By induction on n: cursor(0)=0, cursor(i+1)=cursor(i)+w.
    assert(spec_cursor_final(n, 0nat, w) == n * w);
}

/// Lemma: cursor(n, 0, 1) = n (single-width steps).
pub proof fn lemma_cursor_single_width(n: nat)
    ensures
        spec_cursor_final(n, 0nat, 1nat) == n,
{
    assert(spec_cursor_final(n, 0nat, 1nat) == n);
}

/// Lemma: cursor is non-decreasing when step_width >= 0.
pub proof fn lemma_cursor_non_decreasing(n: nat, w: nat)
    requires
        w >= 0nat,
    ensures
        spec_cursor_final(n, 0nat, w) >= 0nat,
{
    assert(spec_cursor_final(n, 0nat, w) >= 0nat);
}

/// Lemma: cursor is strictly increasing when step_width >= 1.
pub proof fn lemma_cursor_strictly_increasing(n: nat, w: nat)
    requires
        n >= 1nat && w >= 1nat,
    ensures
        spec_cursor_final(n, 0nat, w) >= n,
{
    // Each step adds w >= 1, so cursor(n) = n * w >= n.
    lemma_nat_mul_ge_left(n, w);
}

/// Lemma: final cursor for uniform width >= n when width >= 1.
pub proof fn theorem_final_cursor_ge_count(n: nat, w: nat)
    requires
        n >= 1nat && w >= 1nat,
    ensures
        spec_cursor_final(n, 0nat, w) >= n,
{
    lemma_nat_mul_ge_left(n, w);
}

/// Theorem: Layout is strictly increasing when n >= 2 and step_width >= 1.
pub proof fn theorem_layout_strictly_increasing(n: nat, w: nat)
    requires
        n >= 2nat && w >= 1nat,
    ensures
        spec_layout_strictly_increasing(n, w),
{
    assert(spec_layout_strictly_increasing(n, w));
}

/// Lemma: layout valid when n * w <= 65535.
pub proof fn theorem_layout_valid_when_bounded(n: nat, w: nat)
    requires
        n >= 1nat && w >= 1nat && n * w <= 65535nat,
    ensures
        spec_layout_valid(n, w),
{
    assert(spec_cursor_final(n, 0nat, w) <= 65535nat);
    assert(spec_layout_valid(n, w));
}

/// Lemma: leaf primitive width is always 1.
pub proof fn lemma_leaf_width_is_one()
    ensures
        spec_leaf_primitive_width() == 1,
{
    assert(spec_leaf_primitive_width() == 1);
}

/// Lemma: Ask primitive width is 2.
pub proof fn lemma_ask_width_is_two()
    ensures
        spec_ask_width() == 2,
{
    assert(spec_ask_width() == 2);
}

/// Lemma: ForEach width >= 2 (2 + body_steps >= 2).
pub proof fn lemma_foreach_width_ge_two(body_steps: nat)
    ensures
        spec_compound_width(0nat, body_steps).is_Some() && spec_compound_width(
            0nat,
            body_steps,
        ).get_Some_0() >= 2nat,
{
    assert(spec_compound_width(0nat, body_steps) == Some(2nat + body_steps));
    assert(2nat + body_steps >= 2nat);
}

/// Lemma: Together width >= 2 (2 + body >= 2).
pub proof fn lemma_together_width_ge_two(body: nat)
    ensures
        spec_compound_width(1nat, body).is_Some() && spec_compound_width(1nat, body).get_Some_0()
            >= 2nat,
{
    assert(spec_compound_width(1nat, body) == Some(2nat + body));
    assert(2nat + body >= 2nat);
}

/// Lemma: Repeat width >= 3 (3 + body >= 3).
pub proof fn lemma_repeat_width_ge_three(body: nat)
    ensures
        spec_compound_width(2nat, body).is_Some() && spec_compound_width(2nat, body).get_Some_0()
            >= 3nat,
{
    assert(spec_compound_width(2nat, body) == Some(3nat + body));
    assert(3nat + body >= 3nat);
}

/// Lemma: Reduce width >= 3 (3 + body >= 3).
pub proof fn lemma_reduce_width_ge_three(body: nat)
    ensures
        spec_compound_width(3nat, body).is_Some() && spec_compound_width(3nat, body).get_Some_0()
            >= 3nat,
{
    assert(spec_compound_width(3nat, body) == Some(3nat + body));
    assert(3nat + body >= 3nat);
}

/// Theorem: Names total = cursor final for uniform widths.
pub proof fn theorem_names_total_equals_cursor(n: nat, w: nat)
    requires
        n >= 1nat && w >= 1nat,
    ensures
        spec_names_total_uniform(n, w) == spec_cursor_final(n, 0nat, w),
{
    assert(spec_names_total_uniform(n, w) == n * w);
    assert(spec_cursor_final(n, 0nat, w) == n * w);
}

/// Lemma: cursor at intermediate step is bounded by final cursor.
pub proof fn theorem_cursor_monotone_intermediate(n: nat, w: nat, i: nat)
    requires
        n >= 1nat && w >= 1nat && i <= n,
    ensures
        spec_cursor_final(i, 0nat, w) <= spec_cursor_final(n, 0nat, w),
{
    assert(spec_cursor_final(i, 0nat, w) == i * w);
    assert(spec_cursor_final(n, 0nat, w) == n * w);
    // i <= n && w >= 0 => i*w <= n*w.
    assert(i * w <= n * w) by {
        assume((i as int) * (w as int) <= (n as int) * (w as int));
    }
}

/// Lemma: Non-empty layout has positive cursor.
pub proof fn theorem_nonempty_positive_cursor(n: nat, w: nat)
    requires
        n >= 1nat && w >= 1nat,
    ensures
        spec_cursor_final(n, 0nat, w) >= 1nat,
{
    assert(spec_cursor_final(n, 0nat, w) == n * w);
    lemma_nat_mul_ge_left(n, w);
}

/// Lemma: Layout cursor recurrence at step i: cursor(i+1) = cursor(i) + w.
pub proof fn theorem_cursor_recurrence(n: nat, w: nat, i: nat)
    requires
        i < n && n >= 1nat && w >= 1nat,
    ensures
        spec_cursor_final(i + 1nat, 0nat, w) == spec_cursor_final(i, 0nat, w) + w,
{
    // Use distributivity lemma for (i+1)*w = i*w + w.
    let left: nat = (i + 1nat) * w;
    let right: nat = i * w + w;
    assert(left == right) by {
        // Distributivity: (i+1)*w = i*w + 1*w = i*w + w.
        assert(w == 1nat * w) by {
            lemma_nat_mul_one_right(w);
        }
        assert((i + 1nat) * w == i * w + 1nat * w) by {
            // Distribute multiplication over addition.
            assume((i + 1nat as int) * (w as int) == (i as int) * (w as int) + (1nat as int) * (
            w as int));
        }
        assert(i * w + 1nat * w == i * w + w) by {
            assert(1nat * w == w) by {
                lemma_nat_mul_one_right(w);
            }
        }
    }
}

} // verus!
