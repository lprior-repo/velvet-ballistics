# Canonical action taxonomy and retained identity

Owner: vb-eirmk.60. Authoritative requirement: MASTER section 65, with retained-data and identity preservation from the current task. Baseline: f366003714753937573e04650bf63cc4e5beb8ea. This contract does not claim canonical implementation acceptance.

## Domain and boundary

Canonical execution contracts have exactly SideEffect::{Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell} and RetrySafety::{Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown}. Existing Idempotency remains an independent classification. Stored action records, accepted artifacts, admission evidence, scheduled action ABI, checkpoint contracts, pack manifests, executor versions and receipts retain their actual original serialized identities. Cold records are audit data until the original authority and a justified canonical classification have both been checked.

Legacy SideEffect tags are None=0, Writes=1, Sends=2, Creates=3 and Destroys=4; RetrySafety tags are Safe=0, KeyRequired=1 and Unsafe=2. Derived Serde ordinal encoding, rather than repr(u8), owns these values. Canonical codecs cannot reinterpret those tags. Legacy Writes cannot be inferred to be local or external from its label; Sends/Creates/Destroys cannot be collapsed into a guessed canonical value. Unresolved retained records stay cold and cannot mint execution authority.

## Identity invariants

The exact issuer-produced artifact bytes, issued admission, pending ticket, scheduled ABI digest, manifest, executor version, receipt schema and actual receipt prefix are frozen before production changes. After changes the same archive bytes must still pass their original permitted recovery/reconciliation path, and corruptions must refuse before journal, prefix, counter or effect mutation. The native registered baseline and archives are produced in an independent isolated workspace and imported by exact hash.

The original ExecutorCatalog check_pack equality remains mandatory. Historical executor identity is valid only for the real historical implementation, compiled from its actual unchanged source and retaining exact old contract semantics; hashing archived sources while executing a different implementation is forbidden. Artifact verifier identity has the same constraint: verification.rs and its source-fingerprint dependencies cannot be changed while claiming the previous verifier digest. Actual emitted compatibility decisions follow MASTER77.17, including journaling whenever admission, replay or live-effect dispatch changes.

## Retry behavior

Every reachable retry is checked before another ticket or effect. Pure computation permits retry under its consistent declaration. Reads require declared idempotence. Writes require valid stable-key attestation according to MASTER65. NotRetrySafe and Unknown refuse non-pure retries; Process and UnsafeShell refuse by default. Key ingredients reject unavailable, secret, random and time-dependent values. Declaration-versus-executor authority remains separate: labels do not prove external behavior or key enforcement.

An independently deliverable current-format repair covers the existing runtime resumption path: a non-pure Unsafe contract plus a Retryable ActionFailure must return the existing typed non-retryable error before constructing AwaitingAction. Existing Safe/KeyRequired behavior and pure bypass remain unchanged. This repair changes no enum, ticket/key algorithm, persisted format or source-hashed verifier/catalog file. It does not close the canonical requirement or stable-key obligation.

## Required decisions and acceptance

MASTER65's mapping table lists only external side effects for IdempotentExternal and AtLeastOnceExternal, leaving local/process/shell classifications unmapped. The shipped Fjall register physically writes local state while requiring IdempotentExternal. The user explicitly selected exhaustive implications and confirmed that canonical local executor completion remains blocked. Therefore LocalWrite + IdempotentExternal is forbidden, and relabeling the physical local executor as ExternalWrite is not an accepted workaround. Historical legacy identities may be retained and exercised under their original authority, but no canonical local executor projection is accepted.

Acceptance requires the seven/four canonical public domains, executable compiler/validator/runtime/CLI parity, exact registered-executor declarations, complete declaration/retry/key decision coverage, same-byte archive recovery/reconciliation, corruption/no-mutation controls, and separate independent CODE and TEST reviews. A scalar enum spelling change, a compatibility waiver, a source-hash substitution or the bounded current-format repair alone is insufficient.

## Hazards and evidence boundaries

Primary hazards are Serde tag collision, lost legacy identity, guessed classification, catalog/verifier identity drift, changed retry key when the ticket sequence advances, checkpoint contract drift and blanket rejection of non-retried contracts. The coupled cold verification/runtime projection must retain wire authority separately from canonical execution state. Focused native evidence is behavioral evidence only; canonical joined CI and remaining MASTER acceptance belong to central integration. Benchmarks are waived and no performance claim is made. Formal artifacts are not changed or claimed complete by this contract.
