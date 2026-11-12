# Proof Strategy: vb-qi37.10

## Scope

- Bead: `vb-qi37.10` - codegen: Complete remaining final IR coverage and parity.
- Workspace: `/tmp/opencode/go-skill-vb-qi37-10`.
- Artifact scope: `.beads/vb-qi37.10/` only for State 4.
- Production scope for later states: `vb_codegen` generated final IR support and generated executable parity against `vb_runtime`/`vb_core` oracles.
- Explicit non-scope: `vb-qi37.11` suspension-error expansion, `vb-gvmt` broad generated-mode evidence, Phase 33/44 crash recovery, Phase 37/45 aggregate resource policy, and speed claims.

## Discovery Evidence

- `pwd -P && test -s ...` passed in `/tmp/opencode/go-skill-vb-qi37-10`.
- Risk-trigger scan over scoped files found generated codegen/runtime state machinery, retry/collect state, serialization in the runtime oracle, test assertions, generated source `forbid(unsafe_code)`, and fuzz entry points.
- Verifier-artifact scan over scoped files found no production-bound Verus, Kani, Flux, Loom, Miri, or TLA artifacts for `vb-qi37.10`; only existing source tests, fuzz targets, and generated source safety scans are present.
- `fuzz/src/bin/generated_compare.rs` and fuzz entry points exist, but current exploration says the target is shallow and should be build-gated only if touched.

## Risk Classification

- Critical: final IR support/rejection totality must be fail-closed and cannot silently emit partial Rust.
- Critical: generated execution must match runtime oracle on result, typed error, final pc, slot values, taints, step states, journal signature, and budget behavior.
- Critical: `Collect*` has side-table/pagination lineage/stale/duplicate semantics and is the highest-risk final IR family.
- High: `Together*`, `Reduce*`, and `Repeat*` have multi-step state transitions, accumulator/branch/retry state, and taint propagation.
- High: expression/accessor helpers need executable parity for value, order, missing-field/index errors, and taint.
- High: text helpers `Contains`, `StartsWith`, `EndsWith` are not currently generated; they require either parity implementation or exact fail-closed rejection plus blocker.
- Critical: trybuild compile-fail currently has an empty-fixture loophole; non-empty compile-fail evidence is required.
- Medium: fuzz generated-compare build is useful for panic-freedom/admission robustness only if the fuzz target is changed in this bead.

## Required Proof And Verification Lanes

- Primary required lane: executable generated-vs-runtime parity tests in `vb_codegen` for `Repeat*`, `Reduce*`, `Together*`, `Collect*`, expression/accessor helpers, taint, and journal signature.
- Required compile lane: generated source contract test must compile emitted Rust, run rustfmt/clippy or repository equivalent, and scan generated source for forbidden constructs and unchecked operations.
- Required compile-fail lane: `cargo test -p vb_codegen --test trybuild_tests` must execute non-empty compile-fail fixtures and fail if fixture coverage is empty.
- Required support lane: support/rejection totality must be proven by tests over every final `CompiledNodeKind` and relevant `ExprOp`, with pre-emission typed rejection for unsupported shapes.
- Required final gate after implementation: `moon ci` or scoped formal-verifier classification if unrelated global debt blocks full CI.
- Conditional lane: `cargo fuzz build generated_compare` is required only if `fuzz/src/lib.rs`, `fuzz/src/bin/generated_compare.rs`, or `fuzz/fuzz_targets.rs` are changed.

## Deferred Formal Lanes

- TLA+: deferred follow-up, not a State 6 blocker for this bead, because no production-bound `verification/tla/VbQi3710GeneratedParity.tla` or `.cfg` exists in scope and State 4 is forbidden to create proof files outside `.beads/vb-qi37.10/`. Any future TLA model must be bounded, include typed Err states, model counter overflow as Err, and bind its observations to generated/runtime parity traces.
- Verus: deferred follow-up, not a State 6 blocker for this bead, because there is no non-vacuous Verus proof surface bound to `vb_codegen::validate_generated_subset` or generated storage helper APIs. A future proof must bind to production APIs and cannot prove a standalone copied model.
- Kani: deferred follow-up, not a State 6 blocker for this bead, because no production-bound harness exists for support-matrix totality or generated store bounds. A future harness must use `kani::Arbitrary` or safe exhaustive generators for core workflow shapes; hardcoded dummy workflows are forbidden.
- Loom: not applicable; scoped code has no concurrent implementation change target for this bead.
- Miri: not applicable by default; scoped source forbids unsafe and no unsafe/FFI/interior-mutability target is in scope. Use Miri only if implementation introduces a UB-sensitive primitive or changes core/expr memory semantics.
- Flux: not applicable; no refinement-type surface exists for this codegen bead.

## Acceptance Rule

`vb-qi37.10` must not close while a required final IR family remains unsupported unless the implementation records an exact typed rejection test, an explicit blocker, and an approved scope decision. Executable parity evidence is the acceptance-critical proof for this bead; deferred formal lanes create follow-up work, not fake pass evidence.
