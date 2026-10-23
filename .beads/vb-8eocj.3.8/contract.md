# Rust contract: Jump direction

Jump targets SHALL be strictly forward in compiled node order. validate_kind_edges SHALL validate Jump via validate_forward_target instead of accepting any target. Sanctioned loop constructs (ForEach, Collect, Reduce, Repeat, Together) retain their existing edge validation. This is a pre-implementation validation change; runtime behavior is unchanged. Source acceptance tests remain RED as before. New tests use compiler-derived mutated IR to verify constructor-level BackwardEdge errors, not source acceptance. This validates forward direction only; it is not a closed region proof.

Master reference: section Phase 46, requirement 2 at line 3364. Contract persisted before applying the source replacement. Earlier V65 constructor-admitted runtime failure is retained unchanged in the published native receipt.
