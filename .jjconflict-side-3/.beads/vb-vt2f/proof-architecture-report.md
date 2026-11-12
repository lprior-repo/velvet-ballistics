# vb-vt2f Proof Architecture Report

## Scope

- Bead: `vb-vt2f`
- Sublane: `owner-authorized-unblock / kani-tractable-proof-kernel`
- Attempt: 1 under owner-authorized deeper proof architecture
- Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`

## Architecture Decision

The previous harnesses executed full `Runtime`/`Shard` constructors and pulled CBMC through `hashbrown`, `quick_cache`, `lsm_tree`, and `getrandom`. This sublane replaced those harness bodies with `#[cfg(kani)]` proof kernels under the same harness names:

- `vt2f_runtime_facade_semantics`
- `vt2f_shard_lower_semantics`

The kernels model only the bead-local transition contracts:

- strict admission rejects missing accepted artifacts before queue-depth mutation;
- accepted/relaxed admission increments queue depth;
- public facade `fail_action` maps lower missing/invalid action completion to `InvalidActionCompletion` and preserves unrelated run snapshot;
- ask answer writes only the target answer slot/taint and does not mutate unrelated run state;
- lower shard `ActionFailed` absent-run returns `RunNotFound`;
- `RuntimeActionFailed` maps `RunNotFound` to `InvalidActionCompletion`;
- explicit shard store selection is independent from runtime no-store construction;
- bool ask prompts reject before executed-count mutation, non-bool prompts increment exactly once.

## Trusted Boundary

This is an approved equivalent proof kernel, not a full concrete-runtime Kani proof. Trusted equivalence surfaces:

- `KernelRuntimeError::{AdmissionArtifactNotFound, InvalidActionCompletion, RunNotFound}` is a projection of `crate::RuntimeError` variants.
- `StoreMode::{Missing, AlwaysPresent, StorageBackedAccepted}` is a projection of accepted artifact store behavior.
- `FacadeKernelState` and `ShardKernelState` project only queue depth, active/wrong/absent runs, ask slot/taint, and store-policy facts.
- `AskKernelFrame` projects `wait_ask::ask` semantics for prompt/timeout validation and executed-count mutation.

No stubs, `kani::assume`, `unsafe`, `bounded_any`, disabled checks, or Kani experimental flags are used in the replacement harnesses. Non-vacuity is via `kani::cover!` over policy, store, matching/stale/wrong/absent tickets, and bool/non-bool prompt domains.

## Harness Coverage

- Runtime facade: 7/7 cover properties satisfied.
- Shard lower: 8/8 cover properties satisfied.

## Residual Risk

- The exact concrete `Runtime`/`Shard` implementations are no longer directly executed by these two Kani harnesses; equivalence rests on manual review of the projection against the concrete code paths.
- The original full concrete harness path remains a known Kani tractability risk if reintroduced.
