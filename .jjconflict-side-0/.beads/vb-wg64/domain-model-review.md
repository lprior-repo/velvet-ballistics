# vb-wg64 Domain Model Review

## Domain Under Repair

This bead is a CI integrity repair, not an application feature. The domain is the repository quality gate surface: formatting, source lint, test compilation warnings, and clean-clone forced CI.

## Concepts

- Clean-clone CI: the canonical `moon ci --base HEAD --head HEAD --force` gate that must pass from current `HEAD` without relying on local incremental state.
- Targeted preflight: focused commands used to isolate known failures before running the canonical gate.
- Lint-safe repair: a local edit that satisfies formatter, clippy, or compiler warnings without changing observable production behavior.
- Test contract: existing assertions and scenario structure that must keep proving behavior after unused cleanup.
- Module exposure: making an already-declared test dependency resolvable without widening runtime behavior.

## Boundary Review

The mapped files fall into three repair boundaries:

- `xtask/src/forbidden_scan.rs`: CI tooling boundary. Repairs may improve checked indexing, path API signatures, formatting, and count handling while preserving scanner findings.
- `crates/vb_cli/src/*`: CLI output/test-module boundary. Repairs may collapse linted conditionals and resolve test module imports while preserving CLI output semantics.
- `crates/vb_storage/tests/recovery_bdd_tests.rs`: test-only recovery behavior boundary. Repairs may remove unused imports/variables while preserving BDD assertions and setup effects.

## Invariant Fit

The required invariants are compatible with a minimal CI repair:

- Formatting changes are behavior-neutral.
- Import cleanup is behavior-neutral.
- Replacing unchecked indexing with explicit matching strengthens failure handling without changing successful matches.
- Collapsing nested `if` statements in output helpers must be semantics-preserving.
- Test unused cleanup must not remove assertions or scenario setup.

## Risk Review

- Workspace-wide rustfmt drift may expand the diff beyond initially mapped files. This is acceptable only if the expansion is formatting-only.
- Adding `mode_error.rs` could accidentally create production API surface. It must be private or test-scoped unless existing non-test code requires it.
- Removing unused variables in BDD tests could remove setup side effects if the binding expression performs work. Prefer `_name` bindings when side effects matter.
- Broad lint suppression would weaken governance and is rejected by this contract.

## Domain Verdict

Proceed to proof planning and implementation only under a minimal-diff CI repair contract. There is no domain justification for feature behavior changes, CI weakening, or assertion deletion.
