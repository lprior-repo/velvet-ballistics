# Lean Contract Projection: vb-l2d7

## Boundary
- Lean-owned kernel: abstract taint lattice and abstract data-flow propagation for resolved nodes.
- Rust/runtime shell: actual source code, tests, CI, generated Rust parity, journal behavior, filesystem, bead commands, and doc editing mechanics.
- External systems excluded from Lean proof: repository state, git history, bd database, Moon lanes, source inspection evidence, and manual documentation review.

## Lean-Owned Clauses
- INV-002 -> `Velvet.TaintLattice::join_is_lub`
- POST-001 -> `Velvet.TaintPropagation::resolved_nodes_use_joined_taint`
- INV-003 -> `Velvet.TaintPropagation::data_flow_join_does_not_track_control_flow`

## Theorem Obligations

### THM-INV-002
- Contract clause: INV-002
- Rust/spec target: master-doc taint lattice specification, not production code.
- Lean module: `Velvet.TaintLattice`
- Theorem shape: `join_is_lub`
- Model: finite inductive taint values `Clean`, `DerivedFromSecret`, `Secret` with order relation and binary/list join.
- Refinement: master-doc statements about `join_taint` must be abstractable to this lattice model; no source-code conformance is claimed by this theorem alone.
- Shell exclusions: source inspection, tests, CI, generated mode, runtime engine behavior, doc file mutation.
- Evidence command: `moon run :verify-proof` or `lake build` in a later proof state.
- Required executable companion evidence: `cargo test -p vb_core taint_join_laws`; if a Kani harness is present or added, `KANI_REQUIRED=1 KANI_CMD='cargo kani -p vb_core --harness taint_join_laws' moon run :verify-proof`.

### THM-POST-001
- Contract clause: POST-001
- Rust/spec target: abstract node-taint table for `EvalExpr`, `BuildObject`, `BuildList`, `Finish`.
- Lean module: `Velvet.TaintPropagation`
- Theorem shape: `resolved_nodes_use_joined_taint`
- Model: a node kind, input slot taints, output taint, and finish signal taint.
- Refinement: reconciled documentation must map each resolved node to the abstract propagation rule; Lean does not certify the implementation.
- Shell exclusions: expression evaluator internals, object/list allocation, run-frame mutation, EngineSignal source definitions, tests, generated Rust parity.
- Evidence command: `moon run :verify-proof` or `lake build` in a later proof state.
- Required executable companion evidence: `cargo test -p vb_runtime joined_taint_propagation` or equivalent named workspace test that exercises `EvalExpr`, `BuildObject`, `BuildList`, and `Finish`, plus `moon run :verify-standard`.

### THM-INV-003
- Contract clause: INV-003
- Rust/spec target: v1 design distinction between data-flow taint and control-flow taint.
- Lean module: `Velvet.TaintPropagation`
- Theorem shape: `data_flow_join_does_not_track_control_flow`
- Model: explicit data dependencies contribute to join; branch-choice dependency is outside the v1 taint relation.
- Refinement: reconciled documentation must not infer control-flow tracking from joined data-flow taint wording.
- Shell exclusions: workflow execution, branch evaluation, generated Rust parity, leak analysis beyond v1 scope.
- Evidence command: `moon run :verify-proof` or `lake build` in a later proof state.
- Required executable companion evidence: checked-in doc consistency command, preferred name `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md`, proving data-flow joined-taint wording does not imply v1 control-flow taint tracking, plus `moon run :verify-standard`.

## Waivers
- Clause ID: PRE-001, PRE-002, PRE-003. Waived layer: Lean. Reason: workspace/scope restrictions are process-shell properties, not deterministic kernels. Compensating evidence: manual diff/path review and `git diff --name-only` restricted to approved files. Owner: State 2 documentation agent for bead `vb-l2d7`. Expiry/follow-up: expires before State 2 completion; revoke if any production, test, proof, bead-status, commit, or push action enters scope.
- Clause ID: PRE-004, POST-002, POST-005, ERR-003, ERR-005, ERR-006. Waived layer: Lean for document-text realization. Reason: stale wording and vocabulary conflicts are textual artifact properties. Compensating evidence: checked-in static doc scan, preferred command `python scripts/check-doc-taint-consistency.py velvet-ballistics-MASTER.md`, plus manual QA. Owner: State 2 documentation agent for bead `vb-l2d7`. Expiry/follow-up: expires when doc consistency scan is implemented and passing, or immediately if no executable scan is provided by State 2.
- Clause ID: PRE-005, POST-003, POST-004, INV-001, INV-004, ERR-004. Waived layer: Lean for evidence-bounded implementation claims. Reason: evidence existence and source/test provenance are repository-shell facts, not pure taint algebra. Compensating evidence: evidence-bounded wording report mapping every implementation claim to concrete cited artifact or pending marker. Owner: State 2 documentation agent and independent reviewer for bead `vb-l2d7`. Expiry/follow-up: expires before approving State 2; revoke if any sentence claims unverified tests, CI, generated parity, formal proof, or release readiness.
- Clause ID: INV-005. Waived layer: Lean. Reason: repo-rule enforcement is an executable Rust lint/CI shell property, not a pure taint theorem. Compensating evidence: `moon run :lint-src`, `moon run :check`, `moon run :verify-standard`, and audit that `Cargo.toml` keeps the listed deny/forbid lints. Owner: any downstream implementation agent touching code for bead `vb-l2d7`. Expiry/follow-up: expires immediately when code is touched; downstream must run the named gates and preserve lint configuration.
- Clause ID: ERR-001, ERR-002, ERR-007. Waived layer: Lean. Reason: these errors govern workflow misuse and artifact traceability, not deterministic domain behavior. Compensating evidence: manual review of paths, diff scope, JSONL parse, and traceability completeness. Owner: State 1 contract agent and independent reviewer for bead `vb-l2d7`. Expiry/follow-up: expires at independent contract approval; revoke if any required artifact is missing or JSONL fails to parse.

## Non-Claim
Lean proof of the abstract taint model is not implementation evidence. Any implementation claim still requires source inspection, executable tests, and appropriate verification gates.
