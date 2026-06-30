# Proof Strategy — TLC Fix Round 2 for `vb-rpch`

Status: planning only. This artifact plans repair work for proof-writer/formal-verifier. It does not claim TLC success and does not approve any proof.

## Inputs driving this re-plan

Proof-review rejected the previous TLC fix because:

1. `specs/tla/RecoveryReplayFull.cfg` timed out after 180s with `34,898,477` states still queued.
2. `RecoveryErrorExhaustive` was only `last_error \in ErrorDomain`; most variants were not causally reached.
3. Non-vacuity was one combined reduced witness instead of independent witnesses.
4. `DigestVerificationOrder` did not model verification order.
5. Reductions were disclosed but not tied tightly enough to the recovery contract.

## Round-2 strategy

### 1. Split the old large cfg from the primary proof cfg

The current `RecoveryReplayFull.cfg` bounds are not a realistic primary TLC proof target in this environment:

- `RunId = {1,2}`
- `StepId = {1,2,3}`
- `ActionId = {1,2}`
- `Attempt = {1,2}`
- `MAX_SEQ = 100`
- `MAX_EVENTS = 20`

It timed out with tens of millions of states still queued. Round 2 must not keep calling that cfg the primary proof.

Required proof-writer model/cfg edits:

1. Preserve the old large bound as a stress target by creating `specs/tla/RecoveryReplayFull-large-stress.cfg` with the old constants.
2. Replace `specs/tla/RecoveryReplayFull.cfg` with a contract-preserving bounded primary cfg small enough to exhaust:
   - `RunId = {1}`
   - `StepId = {1}`
   - `ActionId = {1}`
   - `Attempt = {1}`
   - `EnabledEventTypes =` all event types already in the model
   - `MAX_SEQ = 3`
   - `MAX_EVENTS = 3`
3. Keep `RecoveryReplayFull-smoke.cfg` as a separate smoke parse/type target. It may be identical or smaller, but it is not the approval substitute for primary evidence.

Claim scope of the new primary cfg:

- Proves, if TLC completes, the listed invariants over the minimal non-empty domains and journals up to three events.
- Exercises every event type symbolically in a finite event alphabet.
- Establishes that repaired actions are well-typed and that the safety invariants hold over this bounded abstraction.

Out of scope for the new primary cfg:

- No proof of arbitrary run/step/action/attempt cardinality.
- No proof beyond `MAX_EVENTS = 3` and `MAX_SEQ = 3`.
- No liveness/fairness claim.
- No payload-level snapshot decoding proof.
- No proof that the old large stress cfg exhausts. The stress cfg may be run for bug-finding only and must be marked `PARTIAL_BFS` unless it drains the queue.

### 2. Repair digest verification order with explicit state

Required TLA model edits:

- Add a variable such as `digest_stage` with type:
  - `digest_stage \in [RunId -> SUBSET {"WorkflowChecked", "IrChecked"}]`
- Initialize:
  - `digest_stage = [r \in RunId |-> {}]`
- Add `digest_stage` to `vars` / `Spec` variable tuple and every `UNCHANGED` tuple.
- Replace `DigestCheckNext` nondeterministic jumping with a bounded level transition or remove it if not needed. It must not be the only thing making IR eligible.
- Change `CheckWorkflowDigest` so a matching workflow check records:
  - `digest_stage' = [digest_stage EXCEPT ![run] = @ \cup {"WorkflowChecked"}]`
  - mismatch causally records `WorkflowSourceDigestMismatch`.
- Change `CheckIrDigest` so it requires:
  - `"WorkflowChecked" \in digest_stage[run]`
  - `digest_level \in {"WorkflowAndIr", "Full"}`
  - matching IR records `"IrChecked"`; mismatch causally records `CompiledIrDigestMismatch`.
- Replace the old `DigestVerificationOrder` with an invariant that actually speaks about order:
  - `DigestVerificationOrder == \A run \in RunId : "IrChecked" \in digest_stage[run] => "WorkflowChecked" \in digest_stage[run]`
- Add independent reachability witnesses:
  - workflow check reached,
  - IR check reached after workflow,
  - workflow mismatch reached,
  - IR mismatch reached after workflow.

### 3. Repair `RecoveryErrorExhaustive` causally

Round 2 should prefer causal transitions, not a waiver. Proof-writer must add explicit actions that set each current `ErrorDomain` non-`None` variant only from modeled inputs. Required causal shape:

| ErrorDomain variant | Required causal transition |
|---|---|
| `NoRecoveryData` | `RecoverRunWithoutEvents`: choose `run \in RunId` with no journal event for that run, then set `last_error' = "NoRecoveryData"`. |
| `CorruptSnapshot` | `LoadCorruptSnapshot`: choose modeled corrupt marker or snapshot/run mismatch input, then set `last_error' = "CorruptSnapshot"`. |
| `WorkflowSourceDigestMismatch` | Existing workflow digest check mismatch, but keep it causal and preserve/unchange all other variables explicitly. |
| `CompiledIrDigestMismatch` | IR mismatch only after workflow was checked. |
| `ActionAbiMismatch` | `CheckActionAbiDigest`: choose action ABI expected/found mismatch input; this is an abstract public-proof witness for the typed variant, not a claim that GAP-3 runtime lookup is implemented. |
| `PolicyDigestMismatch` | `CheckPolicyDigest`: choose policy expected/found mismatch input; same GAP-3 caveat. |
| `NonIdempotentActionBlocked` | `DetectNonIdempotentResolved`: when a completed or failed `(action, step)` exists and a later same action/step schedule candidate is considered, set this error. |
| `ReplayDivergence` | `DetectReplayDivergence`: model out-of-order step/seq candidate or impossible replay transition, then set this error. |
| `FrameDimensionOverflow` | `DetectFrameDimensionOverflow`: when modeled derived dimension exceeds a configured small `MAX_FRAME_DIM` or when `Len(journal)` exceeds that dimension, set this error. |

Then replace the current partial predicate with:

- `ReachError(err) == last_error = err`
- Optional `RecoveryErrorExhaustive == \A err \in ErrorDomain \ {NoneError} : <witness covered by separate cfg evidence>` as documentation only; TLC cannot quantify over separate cfg evidence inside one invariant.

Each non-`None` variant gets an independent witness cfg that checks `NotReach<Variant> == last_error # "Variant"` and expects a TLC invariant violation.

If proof-writer expands TLA `ErrorDomain` to match the Rust taxonomy (`Journal`, `TerminalStateMismatch`), the same causal+witness rule applies. Otherwise the trusted base must explicitly state that round-2 TLA `ErrorDomain` is the current model domain and that Rust-only/deferred variants remain bridge obligations.

### 4. Split non-vacuity into independent cfgs

Do not reuse the combined `AllNonVacuityWitnessesReached` as primary non-vacuity evidence. Keep it only as an optional smoke witness.

Required independent witness cfgs/commands are listed in `proof-obligations.tlc-fix-round2.planned.jsonl`. Each witness cfg must intentionally check a negated reachability invariant and pass only by producing the expected invariant violation with a raw counterexample trace.

Witness bounds should be tiny and targeted, normally:

- singleton run/step/action/attempt unless the predicate needs two runs,
- `MAX_SEQ <= 4`,
- `MAX_EVENTS <= 4`,
- `EnabledEventTypes` narrowed only to events needed by the witness.

Every narrowed event/domain bound must be copied into formal evidence and recorded in the trusted base ledger/report.

### 5. Evidence and reporting discipline

Formal-verifier must capture raw logs for every command. PASS for invariant proof requires final TLC completion with queue drained. PASS for witness obligations requires the expected invariant violation for the specific negated witness, not just a partial queue.

The proof-writer/formal-verifier must not edit proof-review approvals. A new proof-reviewer pass is required after artifacts are written and evidence is executed.
