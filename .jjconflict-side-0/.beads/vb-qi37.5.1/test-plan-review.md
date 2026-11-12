STATUS: APPROVED

# Test Plan Review: vb-qi37.5.1 — verifier idempotency contract model

## VERDICT: APPROVED

Repaired plan earns approval. Prior rejection findings were addressed with direct BDD coverage for every public contract API, exact typed error assertions, and unit density above the required floor.

## Mode 1 — Plan Inquisition

### Contract Parity

[PASS] Public function BDD coverage

- `contract.md:216` — `validate_workflow_idempotency_contracts(...)` has direct BDD coverage in `test-plan.md:113-188`, including empty workflow, pure acceptance, side-effecting acceptance, exact idempotency violations, missing contract, orphan contract, and Gate 12 precedence.
- `contract.md:221` — `validate_action_idempotency_contract(...)` prior gap is closed by direct BDD scenarios in `test-plan.md:190-237`, covering exact `Ok(())`, `SideEffectingRetryUnsafe`, `SideEffectingAtLeastOnceExternal`, and `SideEffectingDeterministicPure`.
- `contract.md:225` — `collect_idempotency_contract_violations(...)` prior gap is closed by direct BDD scenarios in `test-plan.md:239-286`, covering empty input, all-legal input, single illegal input for every violation kind, multi-violation collection, boxed contents, and deterministic order.
- `contract.md:229` — `is_statically_idempotent_contract(...)` prior gap is closed by direct BDD scenarios in `test-plan.md:288-328`, covering pure acceptance, accepted side-effecting shapes, and every exact violation variant.

[PASS] Error variant exactness

- `contract.md:167-171` — `ActionContractMissing`, `ActionContractOrphan`, and `IdempotencyViolations` are asserted exactly in `test-plan.md:169-188`, `test-plan.md:141-160`, and `test-plan.md:162-167`.
- `contract.md:173-192` — `SideEffectingRetryUnsafe`, `SideEffectingAtLeastOnceExternal`, and `SideEffectingDeterministicPure` are asserted exactly through workflow, direct action, collect, and is-static scenarios at `test-plan.md:141-160`, `test-plan.md:218-237`, `test-plan.md:253-272`, and `test-plan.md:309-328`.
- Runtime variants intentionally retained by the contract context are asserted exactly in `test-plan.md:330-349`.

### Assertion Sharpness

[PASS] No tautological planned Then clauses

- `test-plan.md:18` explicitly bans `is_ok()` / `is_err()` assertions.
- Success assertions use concrete `Ok(())`, for example `test-plan.md:117`, `test-plan.md:124`, `test-plan.md:131`, `test-plan.md:194`, `test-plan.md:243`, and `test-plan.md:292`.
- Failure assertions use exact typed variants and fields, for example `test-plan.md:145`, `test-plan.md:152`, `test-plan.md:159`, `test-plan.md:173`, `test-plan.md:222`, `test-plan.md:257`, and `test-plan.md:313`.
- No reviewed Then clause relies on `Some(_)`, `> 0`, `is_ok()`, or `is_err()` as the oracle.

### Trophy Allocation

[PASS] Density floor

- `contract.md:216-231` exposes 4 public functions.
- `test-plan.md:70-102` names 27 required unit tests.
- Required floor is `5 × 4 = 20`; planned unit density is `27 / 4 = 6.75x`, above target.

[PASS] Proptest and fuzz coverage

- Pure/non-trivial decision APIs have proptest invariants in `test-plan.md:386-436`, including finite decision-table equivalence, accepted-shape iff, rejection variants, collection order, completeness relation, and no mutation.
- Parser/verification boundary fuzzing is planned in `test-plan.md:438-448`; the typed verifier gate also gets a direct bounded-IR fuzz target.

### Boundary Completeness

[PASS] Boundaries named with executable targets

- Empty/minimum cases are explicit: no Do/no contracts at `test-plan.md:113-118`, empty collect input at `test-plan.md:239-244`, empty runtime key slots at `test-plan.md:330-335`.
- Failure boundary cases are explicit: missing contract at `test-plan.md:169-174`, orphan contract at `test-plan.md:176-181`, Gate 12 precedence at `test-plan.md:183-188`, unsafe/at-least-once/deterministic-pure rejection at `test-plan.md:141-160` and `test-plan.md:218-237`.
- Maximum/bounded cases are explicit: bounded `Vec<ActionContract>` length `0..128` at `test-plan.md:423-426`, fuzz seed `128 illegal contracts` at `test-plan.md:441-443`, and CI resource check up to 128 contracts at `test-plan.md:533`.
- Overflow/resource behavior is addressed by bounded traversal, violation count constraints, Kani bounds, and fuzz/static gates in `test-plan.md:467-475` and `test-plan.md:523-534`.

### Mutation Survivability

[PASS] Mutation targets have named killing tests

- Decision-table mutations are mapped to exact tests in `test-plan.md:481-485`.
- Collection short-circuit and ordering mutations are mapped to exact tests in `test-plan.md:486-487`.
- Completeness false-success mutations are mapped to exact tests in `test-plan.md:488-490`.
- Runtime key and taint mutations are mapped to exact tests in `test-plan.md:491-492`.
- CLI/certificate false proof mutations are mapped to exact tests in `test-plan.md:493`.

### Holzmann Plan Audit

[PASS] Rule 2 bounded iteration

- Matrix/proptest iteration is bounded by finite enum domains and explicit vector bounds at `test-plan.md:388-436`; Kani bounds are explicit at `test-plan.md:452-475`.

[PASS] Rule 5 explicit preconditions

- BDD scenarios state concrete Given preconditions with action IDs, side-effect/idempotency/retry variants, workflow shape, registry contents, or runtime key taint. Representative examples: `test-plan.md:115`, `test-plan.md:143`, `test-plan.md:171`, `test-plan.md:220`, `test-plan.md:255`, and `test-plan.md:339`.

[PASS] Rule 8 surfaced side effects

- Static verifier scenarios demand typed values and no filesystem/network/parser/action dispatch side effects at `test-plan.md:365-370`.
- CLI and IPC side effects are isolated as black-box boundary tests at `test-plan.md:372-384`, not hidden in helper setup.

## LETHAL FINDINGS

None.

## MAJOR FINDINGS (0)

None.

## MINOR FINDINGS (0/5 threshold)

None.

## MANDATE

Proceed to implementation/test-writing state. The next state must implement the named tests with exact typed assertions; do not weaken any planned oracle into `is_ok()`, `is_err()`, wildcard variants, or proof-status handwaving.
