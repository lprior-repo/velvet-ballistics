# vb-vt2f Verification Layers

## Boundary

- Primary evidence: executable Given/When/Then scenarios through direct Rust public APIs.
- Touched crates expected later: `velvet-ballistics-workspace-tests`, catalog module, and possibly `vb_runtime`/`vb_core` only if public fixture/API gaps prevent scenario implementation.
- Formal proof stance: State 5 TLA+ obligations are primary temporal proof gates and have PASS evidence. The two Kani obligations are owner-authorized projection proof-kernel gates, not full concrete `Runtime`/`Shard` execution gates. Earlier BDD-only TLA/Verus void waivers are retained only as historical audit records and are not approval paths.
- Canonical release gate: `moon ci`, with scoped nextest evidence required first.

## Layer Assignment

- PRE-001, INV-002 -> review-artifact public-surface audit limited to public API import/use violations and private runtime-core access; test helper style/local structure is out of scope.
- PRE-002, INV-005 -> BDD nextest scenarios with fresh fixtures; mutation/coverage later if runner supports it.
- PRE-003, INV-001 -> deterministic BDD nextest repeated-run evidence; no sleeps/network/IPC/YAML/JSON/HTTP.
- PRE-004, ERR-006 -> acceptance catalog regression and runner metadata assertions.
- PRE-005, POST-012, ERR-002 -> strict admission BDD scenario; drift recorded if current implementation disagrees with master.
- POST-001 -> scoped nextest command for `vb_vt2f_direct_runtime_api_acceptance`.
- POST-002 -> scoped nextest command for `vb_hxm0_acceptance_catalog`.
- POST-003 through POST-011, ERR-001 through ERR-005 -> direct API BDD scenario assertions.
- INV-006 -> static-scan/source review for runtime-core exclusions in touched production files if any; no lint/style gate is imposed on test helpers.
- Runtime lifecycle TLA clauses -> `TLA-VT2F-LIFECYCLE-001` is required.
- Strict admission TLA clauses -> `TLA-VT2F-STRICT-ADMISSION-001` is required.
- Runtime facade Kani clauses -> `KANI-VT2F-RUNTIME-FACADE-001` is required against the owner-authorized projection kernel `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs::vt2f_runtime_facade_semantics`.
- Lower shard/admission/ask Kani clauses -> `KANI-VT2F-SHARD-LOWER-001` is required against the owner-authorized projection kernel `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs::vt2f_shard_lower_semantics`.
- Projection-equivalence risk -> `PROJ-EQ-VT2F-001` is required as explicit manual review/waiver evidence. It must map projection types/actions to concrete code, state limitations, expiry, and non-reuse caveat; it is not an executable equivalence proof.
- Verus waiver candidate -> `WAIVER-VERUS-VT2F-002` is candidate-only until State 6 reviewer approval after all compensating TLA/Kani projection-kernel/BDD/catalog/CI evidence passes and `PROJ-EQ-VT2F-001` is accepted or rejected explicitly.
- Lean clauses -> waived unless a theorem kernel is introduced.
- Release confidence -> `moon ci` after scoped tests pass.

## Exact Evidence Commands Known At Contract Time

- `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance`
- `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog`
- `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance --test vb_hxm0_acceptance_catalog`
- `tlc -config verification/tla/Vt2fRuntimeLifecycle.cfg verification/tla/Vt2fRuntimeLifecycle.tla`
- `tlc -config verification/tla/Vt2fStrictAdmission.cfg verification/tla/Vt2fStrictAdmission.tla`
- `cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics` (owner-authorized projection kernel; not full concrete Runtime API execution)
- `cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics` (owner-authorized projection kernel; not full concrete shard/admission/store execution)
- `moon ci`

## Verus Scope

- Current bead: no approved non-vacuum Verus proof exists for mutable runtime/shard invariants.
- `WAIVER-VERUS-VT2F-002` is candidate-only, not approval evidence. It may be approved only after `TLA-VT2F-LIFECYCLE-001`, `TLA-VT2F-STRICT-ADMISSION-001`, owner-authorized projection-kernel `KANI-VT2F-RUNTIME-FACADE-001`, owner-authorized projection-kernel `KANI-VT2F-SHARD-LOWER-001`, `PROJ-EQ-VT2F-001`, BDD nextest, catalog nextest, and `moon ci` or accepted deferred-global evidence pass.
- If a pure transition kernel is extracted or runtime/core implementation changes again, replace the candidate waiver with executable Verus obligations bound to actual `vb_runtime`/`vb_core` exec functions; no vacuum proofs.

## TLA+ Scope

- `TLA-VT2F-LIFECYCLE-001`: required and passed for submit-to-finish, inspect/cancel, action completion/failure, ask answer, trace list/drain, shutdown, deterministic explicit ticks, and typed error transitions.
- `TLA-VT2F-STRICT-ADMISSION-001`: required and passed for strict/journaled admission, missing accepted-artifact rejection before enqueue, accepted digest/capability pairs, relaxed volatile separation, and explicit shard accepted-store construction versus `Runtime::new` strict missing-store behavior.
- Earlier `WAIVER-TLA-VT2F-001` and `WAIVER-TLA-VT2F-002` are superseded and must not be used as approval evidence.

## Kani Projection-Kernel Scope

- `KANI-VT2F-RUNTIME-FACADE-001`: proof target is the local `#[cfg(kani)]` owner-authorized projection kernel in `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs`, covering strict missing accepted-artifact rejection before projected queue mutation, relaxed/accepted enqueue behavior, action failure mapping to `InvalidActionCompletion`, target ask answer slot/taint writes, unrelated run preservation, and matching/stale/wrong/absent ticket cover points.
- `KANI-VT2F-SHARD-LOWER-001`: proof target is the local `#[cfg(kani)]` owner-authorized projection kernel in `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs`, covering projected lower `ActionFailed` absent-run `RunNotFound`, public-boundary mapping to `InvalidActionCompletion`, Relaxed/Strict/Journaled policy coverage, Missing/AlwaysPresent/StorageBackedAccepted store coverage, explicit shard store versus runtime no-store selection, and bool/non-bool ask mutation behavior.
- Trusted boundary: the projection types and actions named in `.beads/vb-vt2f/proof-architecture-report.md` are manually reviewed abstractions of concrete runtime/shard/admission/ask behavior. The Kani commands prove the projection kernels only; they do not prove full concrete `Runtime`/`Shard` constructors, Fjall/store internals, public snapshots/traces, or scheduler/runtime shell equivalence.
- Non-reuse caveat: approval of these Kani rows is valid only for bead `vb-vt2f` and only for this owner-authorized sublane. Future beads or semantic edits must not reuse these rows as concrete-runtime Kani evidence.

## Theorem Scope

- None.

## Waivers

- WAIVED-TLA-001: SUPERSEDED by `TLA-VT2F-LIFECYCLE-001`; retained for audit only; not an approval path.
- WAIVED-TLA-002: SUPERSEDED by `TLA-VT2F-STRICT-ADMISSION-001`; retained for audit only; not an approval path.
- WAIVED-VERUS-001: SUPERSEDED by `WAIVER-VERUS-VT2F-002` plus required TLA/Kani compensating obligations; retained for audit only; not an approval path.
- PROJ-EQ-VT2F-001: required trusted-boundary review/waiver; owner=State 6 contract-verification reviewer; expiry=before further runtime/shard/admission/ask/action/journal/trace/store-selection semantic edits; limitation=manual projection review only, not executable equivalence; compensating evidence=proof-architecture-report plus proof-review approval for owner-authorized projection kernels.
- WAIVER-VERUS-VT2F-002: candidate-only; owner=State 6 proof-reviewer; expiry=before further runtime/shard/admission/ask/action/journal/trace/store-selection semantic edits; approval requires all listed compensating evidence, accepted projection-equivalence risk, and a reviewer finding that non-vacuum Verus binding is infeasible without broader production refactor.
- WAIVED-LEAN-001: no theorem kernel target; compensating evidence is BDD/public-surface verification.

## Downstream Gate Policy

- If only workspace tests/catalog change: scoped nextest + catalog nextest + `moon ci`.
- If runtime production code changes: add `cargo nextest run -p vb_runtime`, source clippy for touched crates, and reopen proof obligations for changed pure/temporal semantics.
- If admission policy changes: reopen TLA/Verus planning and add exact proof/model obligations before implementation is accepted.
