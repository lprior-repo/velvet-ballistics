# Contract: vb-eirmk.13.9

The existing `relaxed_policy_skips_evidence_validation` test retains its FQN, inputs, relaxed policy, artifact digest, run ID, and all post-admission assertions.

Replace only its success assertion with `assert_eq!(result.as_ref().map(|_| ()), Ok(()), same_message)` so success is checked as the exact unit result and any `Err` fails the mandatory typed assertion.

Preserve the original result value for every subsequent digest and run-identity assertion, and leave every other test case unchanged.

Format only `crates/workspace_tests/tests/runtime_version_barrier_tests.rs` with the pinned formatter and execute the pinned native test suite for that exact file.

Do not change production code, policies, test inputs, dependencies, gates, or other files; report whole-candidate blockers as outside this bead's scope.
