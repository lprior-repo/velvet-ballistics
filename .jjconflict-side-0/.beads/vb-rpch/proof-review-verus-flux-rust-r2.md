# Proof Review — vb-rpch verus-flux-rust-r2

reviewer_skill: proof-reviewer
reviewer_invocation_id: p6-proof-review-verus-flux-rust-r2
proof_writer_task: ses_1a34252e4ffe8tBq8TiMbHlOGF
bead: vb-rpch
state: 6 proof review
workdir: /home/lewis/src/vb-jpq7-jj-fix
date: 2026-05-24

## Disposition

REJECTED. The seven Verus files rerun successfully, but they are standalone mirror models and do not discharge the planned Verus obligations as written. The approved plan required Verus evidence bound to a State-11 production proof surface and explicitly warned against toy replacement models. These artifacts prove algebra over self-defined predicates/types, then ledger the production bridge as pending. That is useful preparatory proof code, not acceptable closure for behavior-affecting obligations.

## Commands run

1. Verus smoke rerun:

```sh
verus verification/verus/vb_rpch_unsupported_recovery_state.rs --crate-type lib && verus verification/verus/vb_rpch_seed_dimensions.rs --crate-type lib && verus verification/verus/vb_rpch_action_replay_tracker.rs --crate-type lib && verus verification/verus/vb_rpch_digest_check.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_snapshot_tail.rs --crate-type lib && verus verification/verus/vb_rpch_hydrate_events.rs --crate-type lib && verus verification/verus/vb_rpch_replay_events.rs --crate-type lib
```

Result: exit 0. Output matched writer's evidence: `4/3/4/2/2/3/4 verified, 0 errors`; replay-events emitted Verus auto-trigger notes for quantifiers.

2. Trust-marker scan over `verification/verus`:

```text
pattern: \b(assume|admit|external_body|trusted|axiom|opaque|reveal_with_fuel|unimplemented|todo|panic|unwrap|expect)\b
```

Result: no executable trust markers in the seven reviewed `vb_rpch_*` R2 files. The reviewed set contains a comment in `vb_rpch_seed_dimensions.rs` saying the production bridge is trusted until State 11.

3. Requires scan over `verification/verus/vb_rpch_*.rs`:

Result: reviewed R2 files have `requires` at `seed_dimensions.rs:24,32,37`, `action_replay_tracker.rs:24,29`, `hydrate_snapshot_tail.rs:47,65`, `hydrate_events.rs:13,22`, `replay_events.rs:39,54`. Several are definitional restatements of the desired conclusions.

4. Flux discovery:

```sh
cargo flux --version
```

Result: non-zero, `error: no such command: flux`. Flux is honestly blocked; no Flux pass is claimed.

5. Kani discovery and blocker smoke:

```sh
cargo kani --version
cargo kani -p vb_storage --harness unsupported_recovery_state_union_kani --no-unwind
```

Result: `cargo-kani 0.67.0`; planned command fails with `unexpected argument '--no-unwind' found`, suggesting `--no-unwinding-checks`. Kani is honestly blocked; no Kani pass is claimed.

6. TLC round-3 preservation check:

```sh
python3 - <<'PY'
from pathlib import Path
for p in ['.beads/vb-rpch/proof-review-tlc-fix-round3.md','.beads/vb-rpch/formal-verification-report-tlc-fix-round3.md']:
    s=Path(p).read_text()
    assert 'STATUS: APPROVED' in s or 'Model checking completed. No error has been found.' in s
print('TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY')
PY
```

Result: exit 0, `TLC_R3_EVIDENCE_PRESENT_SCOPE_TLA_ONLY`. TLC remains bounded TLA evidence only.

## Findings

### CRITICAL — Verus artifacts are disconnected mirror models, not production-bound proofs

Obligations: `VFR-R2-VERUS-001` through `VFR-R2-VERUS-007`.

Artifacts: all seven reviewed Verus files under `verification/verus/vb_rpch_*.rs`.

Evidence:

- Planned obligations lines 8-14 require evidence "bound to State 11 production proof surface; no toy replacement model" and have `behavior_affecting: true`.
- `trusted-base-ledger.verus-flux-rust-r2.jsonl` line 1 records `TB-VFR-R2-VERUS-BRIDGE-PENDING`: standalone Verus artifacts mirror production algebraically and State 11 must attach field/function correspondence before they become implementation refinement evidence.
- `vb_rpch_unsupported_recovery_state.rs` defines `SpecUnsupportedRecoveryState`, `supported`, and `union` locally and proves their own definitions.
- `vb_rpch_digest_check.rs` defines a local `SpecDigestCheck` enum and local hierarchy, without connecting to `crates/vb_storage/src/recovery/types.rs::DigestCheck`.
- `vb_rpch_hydrate_snapshot_tail.rs`, `vb_rpch_hydrate_events.rs`, and `vb_rpch_replay_events.rs` define local precondition/effect predicates rather than proving `hydrate_run_frame`, `hydrate_run_frame_from_events`, or `replay_events` behavior.

Required fix: attach the proofs to production-facing exec/spec functions or a reviewed equivalence bridge, then rerun Verus. Until that bridge exists, mark these as preparatory/blocked, not discharged.

### HIGH — Several Verus lemmas encode the conclusion in `requires`

Obligations: `VFR-R2-VERUS-002`, `VFR-R2-VERUS-003`, `VFR-R2-VERUS-005`, `VFR-R2-VERUS-006`, `VFR-R2-VERUS-007`.

Evidence:

- `vb_rpch_seed_dimensions.rs:23-29` requires `successful_seed_dimensions(...)` and ensures the conjuncts that define it.
- `vb_rpch_hydrate_snapshot_tail.rs:37-56` requires `valid_hydrate_snapshot_tail_preconditions(...)` and ensures the same conjuncts.
- `vb_rpch_hydrate_events.rs:12-15` requires `valid_hydrate_events_preconditions(...)` and ensures the same conjuncts.
- `vb_rpch_replay_events.rs:38-41` requires `old_attempts_have_no_state_effect(...)` and ensures the same quantified property.
- `vb_rpch_action_replay_tracker.rs:23-31` proves set-insert monotonicity only after requiring prior resolution; it does not connect to production tracker mutation semantics.

Required fix: replace restatement lemmas with proof obligations over constructors/transition functions that derive these predicates from inputs and state transitions, plus negative/rejection cases that fail if the predicate definition is weakened.

### HIGH — Required Kani/proptest/fuzz lanes are blocked, not closed

Obligations: `VFR-R2-KANI-001` through `007`, `VFR-R2-PROPTEST-001` through `007`, `VFR-R2-FUZZ-001` through `004`.

Evidence:

- `proof-obligations.verus-flux-rust-r2.written.jsonl` lines 15-21 mark all Kani obligations `BLOCKED_SCOPE`.
- Lines 31-41 mark all proptest/fuzz obligations `BLOCKED_SCOPE`.
- Kani command rerun failed before verification because `--no-unwind` is unsupported by cargo-kani 0.67.0.
- Writer report line 31 correctly says this is not full State 5 closure.

Required fix: repair planned Kani command/harness wiring and create meaningful property/fuzz artifacts or obtain explicit approved waivers/blocker disposition from the owning state. Do not advance these as proof passes.

### MEDIUM — Provenance ledger unavailable for self-approval check

Obligation: review provenance.

Evidence: `.beads/vb-rpch/agent-invocation-ledger.jsonl` was not present in the bead directory. I could not independently verify the proof writer and proof reviewer provenance from the required ledger.

Required fix: provide/update invocation ledger rows for proof planning, proof writing, and proof review before final approval.

## Non-findings / accepted evidence hygiene

- Flux blocker is honest: `cargo flux --version` fails and no Flux pass is claimed.
- TLC round-3 evidence is preserved only as bounded finite TLA evidence; no Rust/Flux/Kani overclaim was found in the R2 evidence.
- No executable `assume`, `admit`, `external_body`, or broad trusted marker was found in the seven reviewed R2 Verus files.

## Nearest rerun state

Rerun from State 5 proof-writer after State 11 (or an approved State-11 sublane) exposes a production proof/equivalence bridge and after Kani/proptest/fuzz blockers are repaired or explicitly waived. If only repairing the review provenance ledger, rerun State 6 afterward; that alone will not clear the Verus/Kani/property/fuzz findings.

STATUS: REJECTED
