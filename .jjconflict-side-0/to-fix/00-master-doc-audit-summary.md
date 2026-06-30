# Master Doc Audit Summary

Audit bead: `vb-3tew`

Scope: report-only audit against `velvet-ballistics-MASTER.md` for the current Backend / IR Interpreter milestone. No production code, tests, configs, or proof artifacts were modified.

Subagents used: 10 specialist reviewers covering YAML/validation, compiler/IR, runtime engine, Fjall durability, IPC/CLI, Rust policy, tests/proofs, architecture drift, black-hat review, and audit meta-review.

## Verdict

Backend / IR Interpreter Complete is not releasable yet.

The tree has a lot of good substrate: numeric IR types, handle-based values, strict YAML-profile machinery, Fjall keyspace/envelope code, bounded shard queues, runtime primitive dispatch surfaces, typed errors, fuzz harness implementation functions, and Moon task skeletons.

The blockers are still real:

- Compiler lowering is not full v1: `do` and `choose` are rejected by the canonical lowering path, and nested primitive bodies only accept `set`.
- Runtime action completion mutates frame state before durable evidence append and does not enforce full idempotency, output-size, duplicate-completion, or taint policy.
- Durable action journal records lose the full `ActionTicket`, idempotency key, and real attempt number.
- Storage/recovery has envelope, compiled-IR digest, pending-action hydration, and digest-check completeness gaps.
- CI/formal gates are materially miswired or smoke-only: Moon uses nonexistent formal task names, Miri/coverage are one-test smokes, fuzz target names do not match the required Section 37 names, and TLC/Kani/Verus evidence has fail-open or vacuity problems.
- Master policy drift remains around taint shape, step-state terminal transitions, resource defaults, Cargo profiles, workspace shape, hot function/file lengths, and deferred-codegen residue.

## Current-Scope Exclusions Applied

These were intentionally not counted as current blockers:

- UI/Makepad/native UI work: deferred by master Sections 0, 23, 33, 42, 69, and `docs/deferred-ui.md`.
- Rust workflow codegen/maxperf/PGO/generated-vs-IR ratios: deferred by master Sections 0, 6, 22, 32, 41.
- Advisory dependency report warnings: non-blocking under the 2026-05-23 owner waiver unless a specific bead opts back in.
- Cold-path JSON in CLI/agent-context: runtime-core JSON remains forbidden, but cold CLI JSON is not automatically a defect.
- Cold parser/validator/compiler maps and formatting: forbidden API checks must be hot-path aware.

<!-- RESOLUTIONS 2026-05-24: 5 beads closed this session — see to-fix/08-resolution-tracker.md for full mapping -->

## Report Files

- `to-fix/01-compiler-yaml-ir-defects.md`
- `to-fix/02-runtime-action-durability-defects.md`
- `to-fix/03-storage-recovery-defects.md`
- `to-fix/04-ci-formal-evidence-defects.md`
- `to-fix/05-architecture-drift-defects.md`
- `to-fix/06-ipc-cli-defects.md`
- `to-fix/07-good-news.md`

## Evidence Limits

This was a static/adversarial audit. I did not run `moon ci`, full Cargo builds, nextest, Miri, fuzz campaigns, coverage, mutants, sanitizers, or benchmarks in this pass. Findings are based on direct source inspection plus delegated subagent inspections. Any release claim still requires executable command evidence.
