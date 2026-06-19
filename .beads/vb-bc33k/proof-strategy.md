# Proof Strategy — vb-bc33k

**Bead:** vb-bc33k — bind vb_expr Verus specs to production type_enforcers
**Master sections:** §40 (proof coverage), §44 (deductive Rust proofs)
**Status:** PARTIAL → planning full closure

## 1. Problem Frame

The existing `crates/vb_expr/src/eval/verus.rs` defines 5 `closed spec fn` mirrors
of the production `expect_*` functions and 6 `proof fn` lemmas, but each lemma
has only a `reveal(spec_expect_*);` body. Verus accepts this, but it does NOT
prove the spec captures production behavior — it merely unfolds a spec that
was hand-written to look like the production match.

The TASK notes (in the bead) call out a broader class of vacuous proofs in
`vb_expr/{eval,lexer,parser,bytecode}/verus.rs`, where `spec_and(a,b) = a && b`
followed by `lemma_and_commutative` proves the commutativity of Rust's `&&`
operator — a tautology.

This plan covers the **type_enforcer** subset (eval/verus.rs) which is the
PARTIAL scope. The other three vacuous files (`lexer`, `parser`, `bytecode`)
are noted as dependencies for vb-3xdp5 / vb-pr6mg and are out of scope here.

## 2. Anti-Laundering Mandate

Per `/home/lewis/.agents/skills/proof-planner/SKILL.md` ANTI-VERIFICATION
LAUNDERING MANDATE:

> When planning Verus proofs, you must explicitly mandate that the `exec fn`
> will contain the actual function body logic or a direct, verifiable path
> to production code. Your plan MUST EXPLICITLY FORBID the use of
> `#[verifier::external_body]`, `assume()`, or `axiom`.

This plan therefore:
- Forbids `#[verifier::external_body]` on the new `exec_expect_*` bridges.
- Forbids `assume(...)` and `axiom` in any new spec or lemma.
- Requires each `exec fn` body to contain the **exact** production match
  arms from `crates/vb_expr/src/eval/type_enforcers.rs` so the Verus
  verifier can symbolically execute the match and discharge the ensures.
- Requires each `proof fn` lemma body to either: (a) unfold via `reveal(...)`
  and apply a quantified case split over the 8-variant enum, or (b) cite a
  previously-proven lemma. A bare `reveal(spec);` followed by `assert ...`
  is permitted only when the assert quantifies over a public enum.

## 3. Lane Selections

| Lane | Required? | Rationale |
|---|---|---|
| Verus (L4) | YES | Type enforcers are total functions on a finite enum — the strongest evidence. |
| Kani (L3) | YES | Bounded arbitrary SlotValue harness, ensures spec fn matches exec for all 8 variants. |
| proptest (L1) | YES | Random-property forcing lane; complements Kani's bounded search with input variety. |
| Flux | NO | Spec fn is already a closed finite-enum match; Flux refinements would duplicate the partition argument. |
| Loom | NO | type_enforcers is a synchronous, single-threaded, pure-function layer. |
| cargo-fuzz | NO | No parser, no codec, no persisted bytes at this layer. |
| TLA+ | NO | No temporal logic, no state machine at this layer. |

## 4. Risk Tags (from seed)

- `type-state`: each `expect_*` is a typestate projection.
- `production-binding`: spec MUST equal production match.
- `tautology-elimination`: forbid `spec_and = a && b` and similar.
- `bounded-state`: 8-variant SlotValue is bounded; Kani can cover it.

## 5. Execution Order

1. Write `exec_expect_*` 5 bridges into `crates/vb_expr/src/eval/verus.rs`
   carrying the full match bodies from production.
2. Strengthen the 6 proof lemmas to use `match v { Bool => ..., I64 => ...,
   ... }` case-split rather than only `reveal(spec);`.
3. Write `crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs`
   with 6 harnesses using `kani::any::<SlotValue>()`.
4. Write `crates/vb_expr/tests/proptest_type_enforcer.rs` with 6 properties.
5. Run `bash scripts/verify-verus.sh`, `bash scripts/kani-list.sh vb_expr`,
   `cargo nextest run -p vb_expr`.
6. Add `obl-vb-expr-type-enforcer-verus-001` (and kani/proptest rows) to
   `contracts/proof_obligations.yaml`.
7. Update bead with close reason referencing raw verifier success logs.

## 6. Out of Scope

- lexer/verus.rs, parser/verus.rs, bytecode/verus.rs tautology cleanup
  (covered by vb-3xdp5 / vb-pr6mg).
- vb_expr core reduction semantics (separate vb-3xdp5 audit).
- vb_expr semantic boundary contract (existing obl-vb-expr-semantic-*).