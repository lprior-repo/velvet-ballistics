# Agent Skill Routing

Use this guide to choose skills for `velvet-ballistics` work. The rule is simple: invoke the narrowest skill that matches the bead and current phase; skip skills that are only keyword-near.

## Selection Rules

1. Load a skill before material work when its trigger matches the task.
2. Do not stack broad skills “just in case.” Every loaded skill must change what you do next.
3. Prefer phase-specific specialists over umbrella workflows when the bead already has a contract, plan, proof, or test artifact.
4. Review skills do not create proof. Execution skills do not review their own output.
5. If no row matches, use normal repo docs, source inspection, and scoped verification.

## Workspace And Task State

| Skill | Invoke when | Do not invoke when | Required handoff |
|------|-------------|--------------------|------------------|
| `beads` | Discovering, claiming, updating, closing, or linking bead work. | Pure read-only code/doc inspection with no bead mutation. | Active bead id and status are known before edits. |
| `dolt` | `bd` fails, server mode is wrong, Dolt push/pull fails, database routing is unclear. | Normal successful `bd` commands. | Bead DB works in server mode or blocker is explicit. |
| `jujutsu` | Creating JJ workspaces, checking `jj root`, rebasing, bookmarking, pushing, or undoing VCS operations. | No VCS operation is needed. | Work stays outside golden checkout; push uses `jj git push`. |
| `moon-v2` | Editing `.moon/`, Moon tasks, CI orchestration, cache/task design, or repairing Moon gate behavior. | Ordinary Rust source/docs work where existing commands are enough. | Moon task changes have command evidence or an explicit skipped-gate reason. |

## Rust Implementation

| Skill | Invoke when | Do not invoke when | Required handoff |
|------|-------------|--------------------|------------------|
| `explore` | Starting a non-trivial bead, mapping files/APIs/risks before contracts, proofs, tests, or implementation. | The task is a one-file obvious docs edit. | Code map, risk notes, and likely verification blast radius. |
| `functional-rust` | Writing, fixing, or reviewing first-party Rust implementation. | Docs-only work or verifier artifact-only work. | Safe, minimal Rust aligned with no-panic/no-unsafe policy. |
| `rust-contract` | New Rust domain capability, workflow, state machine, error taxonomy, or type contract before tests/proofs. | Mechanical bug fix under an existing accepted contract. | Domain contract with invariants, hazards, and proof seeds. |
| `async-rust-reviewer` | Tokio/futures, spawned tasks, async APIs, cancellation, streams, or runtime migration. | Synchronous runtime/core/storage changes with no async boundary. | Async hazards, cancellation, ownership, observability, and shutdown risks are explicit. |
| `fjall` | `vb_storage`, Fjall keyspaces, compaction, snapshots, journal/replay persistence, storage performance. | Non-storage Rust or generic docs. | Storage invariants, durability profile, and recovery evidence needs are explicit. |
| `holzman-rust` | Performance-sensitive Rust, NASA/JPL discipline review, benchmark-backed speed work. | Routine Rust edits with no performance claim. | Benchmark/profiling requirements are named before claiming speed. |
| `architectural-drift` | File-size drift, module cohesion drift, dependency direction drift, or architecture conformance audit. | Opportunistic cleanup outside the bead. | Drift findings become scoped fixes or follow-up beads. |
| `scott-ddd-refactor` | Refactoring to make illegal states unrepresentable or improve type-driven domain design. | Cosmetic cleanup or broad rewrite without a bead. | Refactor has explicit before/after invariants and tests. |

## Proof And Verification

| Skill | Invoke when | Do not invoke when | Required handoff |
|------|-------------|--------------------|------------------|
| `go-skill` | User asks for full proof-first bead lifecycle end-to-end. | A phase artifact already exists and a narrower specialist can continue. | Lifecycle phase, active bead, and next specialist are explicit. |
| `proof-planner` | Accepted Rust contract needs proof obligations and lane selection. | Writing proof code or production code. | Machine-readable obligations and exact commands. |
| `proof-plan-reviewer` | A proof plan exists and needs independent pre-proof review. | Reviewing written proofs or tests. | Accepted/rejected plan with required fixes. |
| `proof-writer` | Approved proof plan needs Verus/Kani/Flux/Loom/Miri/proptest/fuzz artifacts. | Production Rust implementation. | Proof artifacts and raw verifier commands are ready for review. |
| `proof-reviewer` | Written proof artifacts or proof evidence need adversarial review. | Reviewing behavior test suites. | Accepted proof evidence or rejected findings. |
| `proof-to-implementation` | Accepted proof claims need mapping to Rust source refs, behavior tests, and verification commands. | Before proof artifacts are reviewed. | Bridge map ready for independent Black Hat review. |
| `formal-verifier` | Approved proof/test/refinement obligations need execution and ledger evidence. | Planning, writing, or self-reviewing proof artifacts. | Raw command logs, pass/fail status, and verification ledger. |
| `kani` | Kani harnesses, `kani::Arbitrary`, bounded model checks, or counterexamples are in scope. | Generic safety review with no Kani lane. | Harness avoids hardcoded shapes and has exact command evidence. |
| `verus` | Verus specs/proofs, loop invariants, ghost/exec binding, or verifier failures are in scope. | Non-Verus Rust work. | Proof binds to production behavior or blocker is explicit. |
| `flux-rs` | Flux refinements, `cargo flux`, Flux attributes, trusted/ignored boundaries are in scope. | Reactive UI “Flux” or non-Flux proof work. | Flux checks and trusted boundaries are explicit. |
| `miri` | Miri, unsafe UB checks, Stacked/Tree Borrows, invalid values, alignment, or provenance are in scope. | Safe Rust behavior proof with no UB lane. | Miri command evidence is labeled as UB evidence, not whole-crate proof. |
| `loom` | Loom schedule exploration, memory ordering, lock-free/waker/thread interleavings are in scope. | General async Rust review or proof planning. | Model covers the concurrency risk and has bounded execution evidence. |
| `rust-fuzzer` | Safe Rust fuzz targets, structured fuzz input, parser/decoder/interpreter fuzzing are in scope. | Property-test-only or non-Rust fuzzing. | Fuzz harness is safe Rust and tied to target behavior. |
| `tla-plus` | TLA+/PlusCal temporal model, TLC checks, liveness/fairness/deadlock, scheduler/recovery protocols are in scope. | Rust implementation proof. | Model has bounded hardware/error states and command evidence. |

Proof chain rule: `proof-planner` -> `proof-plan-reviewer` -> `proof-writer` -> `proof-reviewer` -> `proof-to-implementation` -> `black-hat-reviewer` bridge review -> `formal-verifier`. Do not skip independent review gates.

## Tests And QA

| Skill | Invoke when | Do not invoke when | Required handoff |
|------|-------------|--------------------|------------------|
| `test-planner` | A behavior/test strategy is needed from code, bead, or contract. | Proof plan review or proof artifact review. | Test plan with unit/integration/proptest/mutation coverage. |
| `test-reviewer` | A test plan or written test suite needs adversarial assertion/determinism review. | Proof artifacts. | Accepted tests or concrete findings before landing. |
| `test-writer` | Approved test plan needs actual tests/properties/harnesses. | No approved plan for complex behavior. | Tests run or compile, then return to `test-reviewer`. |
| `bdd-enforcer` | End-to-end behavior scenarios are required after implementation. | Low-level internal-only changes with no user-visible behavior. | Executable Given/When/Then evidence or scoped waiver. |
| `qa-enforcer` | Product/CLI/API behavior needs ruthless executed QA. | Static source review only. | Real command/API evidence and failures. |
| `hands-on-qa` | User asks to manually smoke test CLI/API paths. | Writing or modifying code. | Terminal evidence for happy and failure paths. |
| `red-queen` | Adversarial state-space or evolutionary QA is needed for behavior resilience. | Simple docs or mechanical fixes. | Mutated/adversarial scenario findings or pass evidence. |

Test chain rule: `test-planner` -> `test-reviewer` -> `test-writer` -> `test-reviewer`. Written tests do not approve themselves.

## Review, Evidence, And Landing

| Skill | Invoke when | Do not invoke when | Required handoff |
|------|-------------|--------------------|------------------|
| `black-hat-reviewer` | Contract parity, architecture, DDD, Rust safety, bridge review, or final adversarial gate. | As replacement for command execution. | Findings by severity or explicit approval with residual risks. |
| `truth-serum` | AI-generated work needs hallucination, evidence, path, or raw-command audit. | As implementation or proof-writing skill. | Execution evidence with exact commands or explicit blockers. |
| `evidence-packaging` | Proof/test/review gates passed and assurance bundle is needed. | Before verification and Black Hat review are done. | Requirement-to-evidence map with raw logs. |
| `landing-skill` | Accepted work needs final gates, push, cleanup, and handoff. | Before open findings are resolved. | Pushed branch/commit and clean handoff evidence. |

## Planning And Documentation

| Skill | Invoke when | Do not invoke when | Required handoff |
|------|-------------|--------------------|------------------|
| `skill-writer` | Editing agent skills, strengthening skill routing docs, or evaluating trigger behavior. | Application code changes. | Compact skill contract, trigger/non-trigger rules, and verification. |
| `arch-design-qa` | Ambiguous architecture/product design needs adversarial discovery before spec work. | Implementing an already accepted bead. | Clarified domain model, hazards, and open decisions. |
| `planner` | Complex work needs atomic beads from accepted scope. | Single focused fix with existing bead. | Concrete bead decomposition. |
| `decomposer` | Architecture spec needs molecular task shredding and plan critique loop. | No accepted architecture spec. | Shredded task plan ready for bead creation. |
| `plan-shredder` | Task decomposition needs adversarial review. | Direct implementation. | Rejected or hardened plan. |
| `doc-to-beads` | User says read a document and make beads without interactive architecture loop. | Code changes. | Persisted bead set from the document. |
| `arch-spec-to-beads` | Existing `architecture-spec.md` needs decomposition into beads. | General docs cleanup. | Validated persisted beads. |

## Do Not Route Here

This repository’s current scope is backend/IR-interpreter work. Do not route ordinary repo work to platform, desktop, fleet, or deferred-UI skills. If the user explicitly asks for such work, confirm that it is outside this backend scope before proceeding.
