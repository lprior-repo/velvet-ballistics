# Current API Mutation Plan

This plan refreshes mutation targets against the current public API surface for bead `vb-c3k9`. It is a plan, not a claim that mutation gates have already passed. Full mutation and coverage evidence remains owned by `vb-gmtg` and related release evidence beads.

## Helper Semantics Mutation Targets

Current helper semantics span interpreter and generated-mode behavior:

| API surface | Mutations to kill | Expected kill evidence | Owner bead |
|---|---|---|---|
| `contains`, `starts_with`, `ends_with` | invert boolean result, accept missing symbol text, swap prefix/suffix operands | helper unit/property tests reject wrong symbol semantics and generated-mode rejection tests still fail closed | `vb-c3k9` plan, execution in `vb-gmtg` |
| `length`, `empty` | accept wrong operand type, return off-by-one length, treat non-empty list/symbol as empty | expression helper tests assert exact typed errors and exact finite counts | `vb-c3k9` plan, execution in `vb-gmtg` |
| `has`, `exists`, `sum`, `count` | delete null checks, ignore list element type errors, off-by-one count, unchecked numeric sum | interpreter/generated helper tests and proptests cover typed errors and checked arithmetic | `vb-c3k9` plan, execution in `vb-gmtg` |
| `append`, `append_if`, `merge`, `unique` | drop taint joins, skip bounded capacity checks, reverse deterministic order, fail to de-duplicate | generated helper capacity/taint tests plus value-store property tests kill ordering and capacity mutants | `vb-c3k9` plan, execution in `vb-gmtg` |

## Runtime Recovery Mutation Targets

Runtime recovery must kill mutants that hide ordering or hydration regressions:

| API surface | Mutations to kill | Expected kill evidence | Owner bead |
|---|---|---|---|
| Action resume ordering | move `ActionCompleted before frame mutation` after frame state updates | recovery/replay tests fail on journal order mismatch | `vb-qi37.12` family |
| Journal replay | skip journal sequence hydration checks, accept missing sequence, conflate duplicate sequence | replay hydration tests and storage journal tests fail with exact typed errors | `vb-gmtg` evidence |
| Snapshot recovery | accept stale snapshot hydration, ignore run header mismatch, drop frame pc restoration | snapshot/recovery tests assert exact restored pc, run id, and correlation | `vb-gmtg` evidence |
| Retry state | mutate retry state max-attempt branch or off-by-one attempt comparison | retry parity tests reject wrong branch and typed policy result | `vb-gmtg` evidence |
| Runtime admission branch | remove admission branch gating, accept unadmitted artifacts, or downgrade the release-blocking disposition | `test_mutation_gate_fails_when_admission_branch_removed` fails closed and scoped cargo-mutants targets the admission branch instead of unrelated smoke | `vb-njju` |

## Generated Rust Parity Mutation Targets

Generated Rust mode must remain semantically bound to the interpreter:

| API surface | Mutations to kill | Expected kill evidence | Owner bead |
|---|---|---|---|
| generated-interpreter suspension parity | continue past `Do`, `WaitUntil`, `WaitEvent`, or `Ask` suspension | generated suspension tests assert exact `SuspensionOutcome` and resume pc | `vb-qi37.10`/`vb-gmtg` |
| full final IR equivalence | drop taint propagation, skip checked slot read/write, wrong branch pc | equivalence/proptest lanes compare interpreter and generated results | `vb-gmtg` |
| unsupported generated-mode rejection | silently fallback for `Together`, `Reduce`, `Repeat`, or `Collect` | compile/codegen tests require typed `UnsupportedIr` before emission | `vb-c3k9` plan, execution in `vb-gmtg` |

## CLI, IPC, and Storage Envelope Mutation Targets

Boundary envelopes must reject stale or malformed inputs through public surfaces:

| API surface | Mutations to kill | Expected kill evidence | Owner bead |
|---|---|---|---|
| binary IPC frame length | accept short/long frames, wrong magic, wrong CRC, or partial payload | IPC frame tests and fuzz smoke fail with typed `IpcError` variants | `vb-9ihz`/`vb-gmtg` |
| postcard envelope | decode corrupt artifact/journal bytes or ignore digest mismatch | artifact and journal decode tests reject corrupt payloads | `vb-gmtg` |
| Fjall journal | mutate key ordering, missing flush, or wrong partition mapping | storage journal/replay tests assert durable event order | `vb-gmtg` |
| CLI accepted artifact path | bypass verify/simulate/submit accepted-artifact checks | CLI envelope tests reject raw source submission and wrong artifact path | `vb-gmtg` |

## UI Model Contract Mutation Targets

Cold UI model contracts must not hide release evidence gaps:

| API surface | Mutations to kill | Expected kill evidence | Owner bead |
|---|---|---|---|
# allow-removed-crate: API surface table enumerates the removed UI model crate as a mutation target
| `vb_ui_model` screen taxonomy | drop required certificate, incident, or replay screen state | UI model acceptance tests assert exact screen/state identifiers | `vb-nf2u`/`vb-gmtg` |
| certificate cards | accept missing verification command/evidence mapping | UI snapshot/model tests reject missing evidence links | `vb-nf2u`/`vb-gmtg` |
| incident replay | conflate incident timeline and replay timeline state | UI model tests require distinct incident and replay states | `vb-nf2u`/`vb-gmtg` |

## Owner Beads and Release Blockers

Every mutation target has an owner bead and a release disposition:

- `vb-c3k9`: current API mutation-plan refresh and validation tests.
- `vb-gmtg`: mutation and coverage evidence capture for current APIs.
- `vb-9ihz`, `vb-d12k`, `vb-qk69`: known production survivor-kill beads for IPC, compile, and core surfaces.
- `vb-nf2u`: UI model and snapshot release acceptance coverage.

Critical semantic survivor policy:

1. A critical survivor in a scoped target is `BLOCK_LOCAL` for its owner bead.
2. A survivor outside the current bead scope must be recorded with exact command output and either an existing owner bead or `bd create` follow-up text.
3. No release-risk acceptance is valid unless it names the survivor, affected public API, risk, compensating evidence, and owner bead.
4. Mutation exclusions are allowed only for generated boilerplate, unreachable compile-time rejected states, or tool limitations, and must cite exact tests or proof evidence that cover the semantics.

Validation command and threshold:

- Focused validator command: `cargo test --package velvet-ballistics-workspace-tests --test vb_c3k9_current_api_mutation_plan`.
- Scoped mutation command: `cargo mutants --package velvet-ballistics-workspace-tests --test vb_c3k9_current_api_mutation_plan`.
- Admission-branch closure command: `cargo mutants --package velvet-ballistics-workspace-tests --test vb_njju_mutation_fuzz_property_closure`.
- Unrelated smoke substitution policy: `moon run :mutants-smoke` over `crates/vb_core/src/diagnostic.rs` is regression smoke only and never satisfies admission-branch closure for `vb-njju`.
- Release evidence threshold: at least `90% mutation kill rate` for scoped semantic targets.
- Mutation exclusion policy: exclusions are only valid for generated boilerplate, unreachable compile-time rejected states, or documented tool limitations with exact compensating test/proof evidence.
