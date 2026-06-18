# Tier A Scope Audit

**Date:** 2026-06-17
**Author:** Tier A scope auditor (opencode session)
**Source of truth:** `velvet-ballistics-MASTER.md` §44 (24 DoD), §76 (UI/Makepad deferred), §58 (codegen deferred), §78 (Tier A waves 0–13)

## Inventory summary

| Bucket | Count |
|---|---|
| Total beads inventoried | 61 |
| Already-labeled `tier-a` (NEW) | 22 (tier-a-0-001 … tier-a-12-022) |
| Wrongly-labeled `tier-a` (mislabeled vb-calg) | 1 (vb-calg) |
| Deferred P4 residue (target of audit) | 39 |

The 22 NEW `tier-a-*` beads are sacred (KEEP). One bead (vb-calg) was
incorrectly labeled `tier-a+wave-5` by a prior subagent and must be cleaned
up. The 39 deferred P4 beads are the candidates for nuke / move.

## §78 reference (Tier A scope)

Tier A is the milestone that satisfies all 24 DoD points of §44
(Backend / IR Interpreter). It explicitly does **not** include:
- §76 UI / Makepad command center (deferred)
- §58 codegen residue (deferred)
- §74 multi-binary split (single binary remains)

Wave table from §78 (excerpt):
- Wave 0: gate repair (2 existing + 6 NEW)
- Wave 1: storage envelope + digest (6)
- Wave 2: core semantics, taint, step state, ResourceContract (3)
- Wave 3: compiler source-to-IR (10 existing + 2 NEW)
- Wave 4: verification gate wiring (12 + 1 NEW)
- Wave 5: runtime decomposition (5) — covers `vb-9kwz.*` and `vb-pymh`
- Wave 6: security & concurrency (5 NEW)
- Wave 7: hot-path bounds + recovery hydration (5 + 1 NEW)
- Wave 8: benchmark evidence (8)
- Wave 9: Kani God-Rule (3 + 1 NEW)
- Wave 10: CLI parity typed Postcard + action inspect (6)
- Wave 11: idempotency hardening (3)
- Wave 12: release artifacts (5 NEW)
- Wave 13: final verification (2)

`vb-calg` ("cli: Split vb_cli app_impl below 300 lines") is **not** in the
§78 wave 5 list. CLI refactor is not in §44 DoD and is not part of wave 5
"Runtime decomposition". The 300-line cap (master §23) is enforced
externally by `scripts/source-length-gate.py` (a wave-0 gate), not by
bead-driven work.

## Beads to KEEP in tier-a (22 NEW + 0 wrongly-classified)

| ID | Title | Wave |
|---|---|---|
| tier-a-0-001 | cli: install source-length CI gate via moon ci | wave-0 |
| tier-a-0-002 | cli: install residue quarantine CI gate via moon ci | wave-0 |
| tier-a-0-003 | cli: install spelling allowlist CI gate via moon ci | wave-0 |
| tier-a-0-004 | ci: lock nightly feature allowlist | wave-0 |
| tier-a-0-005 | release: add Tier A master amendment §78 | wave-0 |
| tier-a-0-006 | release: write empty RELEASE_CHECKLIST.md stub | wave-0 |
| tier-a-0-007 | release: write empty CHANGELOG.md stub | wave-0 |
| tier-a-3-008 | compile: implement compile-to-IR roundtrip proptest | wave-3 |
| tier-a-3-009 | compile: implement IR primitive-coverage matrix | wave-3 |
| tier-a-4-010 | verify: bind Kani/Verus/Flux/TLC/proptest/fuzz substrate | wave-4 |
| tier-a-6-011 | ipc: enforce CallerCapabilities envelope + Unix peer creds | wave-6 |
| tier-a-6-012 | ipc: chmod 0o600 Unix socket on bind | wave-6 |
| tier-a-6-013 | runtime: TOCTOU shutdown CAS on terminal state | wave-6 |
| tier-a-6-014 | runtime: terminal-runs LRU + ttl + codec_miri_tests fix | wave-6 |
| tier-a-6-015 | verify: cfg(loom) gating + Holzman unsafe waiver | wave-6 |
| tier-a-7-016 | runtime: implement WholeWorkflowBudget analyzer | wave-7 |
| tier-a-9-017 | verify: kani::Arbitrary for WorkflowParts/RunFrame/ActionContract | wave-9 |
| tier-a-12-018 | release: finalize RELEASE_CHECKLIST.md | wave-12 |
| tier-a-12-019 | release: finalize CHANGELOG.md | wave-12 |
| tier-a-12-020 | release: quarantine vb_codegen/vb_ui_model/vb_ui_makepad | wave-12 |
| tier-a-12-021 | release: write §17 dead-letter recovery plan | wave-12 |
| tier-a-12-022 | release: tag v0.1.0 + push origin + verify moon ci | wave-12 |

## Beads to NUKE (closed as out-of-scope) — 21 beads

P4 + title/label contains UI, makepad, codegen, command-center, splash,
or marketing keywords. All fall under §76 (UI/Makepad) or §58 (codegen)
deferred scope, none touch §44 DoD points.

| ID | Title | Reason |
|---|---|---|
| vb-0tq8 | makepad: Pin dependency and record supply-chain evidence | §76 UI/Makepad deferred |
| vb-3c1l | ui: Extract canonical token source of truth | §76 UI/Makepad deferred |
| vb-3f88 | ui: Build typed workflow graph authoring shell | §76 UI/Makepad deferred |
| vb-7i3g | ui: Generate Makepad Rust/Splash and contract tokens | §76 UI/Makepad deferred |
| vb-7nr3 | ui: Implement required custom widget catalog | §76 UI/Makepad deferred |
| vb-7p5y | ui: Add keyboard navigation and accessibility-equivalent constraints | §76 UI/Makepad deferred |
| vb-b1hq | makepad: Implement Splash AppShell contract | §76 UI/Makepad deferred |
| vb-fjx5 | ui: Add Makepad performance smoke benchmarks | §76 UI/Makepad deferred |
| vb-g61i | ui: Implement remaining command-center screens | §76 UI/Makepad deferred |
| vb-h3fx | formal: Bind Verus proofs to generated store APIs | §58 codegen deferred |
| vb-h8h0 | Codegen: compare_generated_to_ir equivalence | §58 codegen deferred |
| vb-mnv0 | formal: Add Kani generated support harness | §58 codegen deferred |
| vb-o083 | ui: Implement AI context companion panel with citation rules | §76 UI/Makepad deferred |
| vb-qi37.19 | ui: Generate canonical Figma/Makepad token bridge | §76 UI/Makepad deferred |
| vb-qi37.20 | ui: Complete workflow graph authoring screen | §76 UI/Makepad deferred |
| vb-qi37.21 | ui: Release hardening and visual overlap gates | §76 UI/Makepad deferred |
| vb-qi37.9.4 | expr/codegen: Close remaining helper parity gaps | §58 codegen deferred |
| vb-qi37.9.4.1 | expr/codegen: Add generated-mode symbol store for text helpers | §58 codegen deferred |
| vb-qi37.9.5 | expr/codegen: Add F64 and helper parity tests | §58 codegen deferred |
| vb-r90d | quality: Enforce CLI UI typed artifact parity matrix | §76 UI/Makepad deferred |
| vb-w20g | formal: Add bounded TLA model for generated parity | §58 codegen deferred |

## Beads to MOVE to deferred-backlog label — 17 beads

P4 deferred infra that does not touch §44/§78 DoD path. Not nuked
because some have legitimate "post-Tier-A" or "tooling cleanup"
character that may be revived in a future tier. They are explicitly
demoted from any current scope.

| ID | Title | Reason |
|---|---|---|
| vb-2pwp | Follow up: decompose pre-existing oversized vb_compile source files | infra cleanup, not on §44 DoD; source-length-gate is the enforcement point |
| vb-4ki5 | cli: Add build/test evidence for vb alias | post-Tier-A CLI alias polish; wave 10 covers typed Postcard + action inspect only |
| vb-i7xn | Add max-speed xtask proof/test orchestrator | automation, not on §44 DoD |
| vb-i8vq | Cargo-vet Operationalize | supply-chain; §44.23 marks supply-chain/unsafe reports as advisory under owner waiver |
| vb-mq96 | ipc: Remove unused write_frame import warning | lint cleanup, not on §44 DoD |
| vb-qi37.14 | cli: Prove explain, diff, graph, and run-step contracts | mvp-post-core-cli, not in §78 wave 10 |
| vb-qi37.14.4 | cli: Add graph export command | mvp-post-core-cli |
| vb-qi37.15 | cli: Complete submit, simulate, trace, observability contracts | mvp-post-core-cli |
| vb-qi37.15.4 | cli: Add observability flags and filters | mvp-post-core-cli |
| vb-qi37.15.5 | cli: Add deliver sinks | mvp-post-core-cli |
| vb-qi37.17 | cli: Complete incident, doctor, action, system, agent-context | mvp-post-core-cli |
| vb-qi37.17.2 | cli: Add doctor command | mvp-post-core-cli |
| vb-qi37.17.3 | cli: Add action list and inspect commands | mvp-post-core-cli |
| vb-qi37.17.4 | cli: Add system status command | mvp-post-core-cli |
| vb-qi37.18 | cli: Prove mandatory vb binary alias and canonical naming | post-Tier-A CLI |
| vb-u8yg | Follow up: repair local direct Miri rust-src tooling path | tooling, not on §44 DoD |
| vb-ygy2 | Compiler full v1 primitive source lowering | deferred-duplicate, redundant with tier-a-3-008/009 + §44.3/17 |

## Beads that are TIer A scope but were MISSED from prior labeling

**None.** Every bead whose title or labels touch §44 DoD points is
already one of the 22 NEW `tier-a-*` beads. The `vb-9kwz.*` and `vb-pymh`
runtime-decomposition beads referenced by §78 wave 5 do not exist in
this tracker snapshot — they will be created as part of wave 5 work
itself, per §78's "21 NEW beads + ~55 existing tracker beads
(re-closed with Tier A acceptance criteria)" language. Wave 5 is
"parallel after 2" and not in scope for this audit.

## Special: vb-calg cleanup

`vb-calg` ("cli: Split vb_cli app_impl below 300 lines") was incorrectly
labeled `tier-a+wave-5` by a prior subagent. Per §78 wave 5, runtime
decomposition is covered by `vb-9kwz.*` and `vb-pymh`, not by a CLI
300-line refactor. vb_cli is the CLI binary, not the runtime; the 300-line
cap is enforced by `scripts/source-length-gate.py` (a wave-0 gate, already
covered by `tier-a-0-001`). vb-calg is out of Tier A scope.

Actions:
1. `bd update vb-calg --remove-label tier-a --remove-label wave-5`
2. `bd close vb-calg --reason "Out of Tier A scope per §78 wave table; runtime split covered by vb-9kwz.* and vb-pymh. Master §76 (UI/codegen deferred) + §23 (300-line cap)."`

## Total actions

| Action | Count |
|---|---|
| `bd close` (NUCLEAR) | 21 |
| `bd close` (special: vb-calg) | 1 |
| `bd update` to relabel to deferred-backlog | 17 |
| `bd update` to remove tier-a/wave-5 from vb-calg | 1 |

## Totals after audit

| Bucket | Pre-audit | Post-audit |
|---|---|---|
| Open tier-a beads | 22 (NEW) + 1 (vb-calg wrong) | 22 |
| Deferred P4 beads (any status) | 39 | 17 (moved to deferred-backlog) + 22 (closed as out-of-scope) = 39 accounted for |
| Beads with `deferred-backlog` label | 0 | 17 |
| Beads with `tier-a` label | 23 | 22 |
| Beads with any `wave-N` label | 23 | 22 |
