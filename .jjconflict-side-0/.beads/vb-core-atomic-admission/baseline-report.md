bead_id: vb-core-atomic-admission
bead_title: vb-core-atomic-admission
phase: 1
updated_at: 2026-05-15T19:35:58.057644+00:00
attempt: 1-of-7

# Baseline report

Baseline captured before proof/test/implementation edits in isolated workspace.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission
base_revision_command: `jj log -r @- --no-graph -T ...`

## bd show vb-core-atomic-admission --json
exit=1
```json
{
  "error": "no issues found matching the provided IDs"
}
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/.beads
Error fetching vb-core-atomic-admission: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues

```

## jj status
exit=0
```text
The working copy has no changes.
Working copy  (@) : kqmwuzxr 3fa6735a (empty) (no description set)
Parent commit (@-): moyvrvsn c9c7eee4 main | Delete CHANGELOG.md

```

## jj base revision (@-)
exit=0
```text
moyvrvsnmmzt c9c7eee46ef4 Delete CHANGELOG.md

```

## Baseline classification note

All 20 P0 go-skill workspaces were created from `trunk()` before bead-local edits. The canonical machine gate will be executed and compared in State 11; this report preserves pre-edit bead/workspace identity and base revision evidence for regression classification.

## Corrected bd show vb-core-atomic-admission --json using source server-mode DB
updated_at=2026-05-15T19:37:45.053546+00:00
command=`bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-atomic-admission --json`
exit=0
```json
[
  {
    "id": "vb-core-atomic-admission",
    "title": "runtime/storage: Persist accepted run as atomic Fjall batch",
    "description": "Persist workflow_source, compiled_ir AcceptedArtifact, run_header, RunAccepted, and required indexes as one accepted-run durability boundary before acknowledgement. Remove mixed raw WorkflowParts versus AcceptedArtifact storage for strict admission paths.",
    "acceptance_criteria": "Strict accepted-run creation is one durable batch or fails before acknowledgement; source/artifact/header/RunAccepted/index records are all present after success; failure injection leaves no partially accepted run; accepted_at_seq records a real journal sequence.",
    "status": "in_progress",
    "priority": 0,
    "issue_type": "feature",
    "assignee": "Lewis",
    "owner": "priorlewis43@gmail.com",
    "created_at": "2026-05-15T04:01:02Z",
    "created_by": "Lewis",
    "updated_at": "2026-05-15T19:33:45Z",
    "labels": [
      "admission",
      "core-priority",
      "durability",
      "engine",
      "fjall",
      "runtime",
      "storage"
    ],
    "dependencies": [
      {
        "id": "vb-core-accepted-artifact-format",
        "title": "artifact: Define stable AcceptedArtifact format",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Define one stable accepted artifact format for strict runtime admission and persistence. Remove ambiguity between raw WorkflowParts, CompiledWorkflow, and AcceptedArtifact on production paths.",
        "acceptance_criteria": "Storage, CLI, verifier, and runtime agree on one accepted artifact schema/digest/proof envelope; raw WorkflowParts cannot satisfy strict admission; malformed or legacy formats have explicit rejection behavior and tests.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:23Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:10Z",
        "labels": [
          "admission",
          "artifact",
          "core-priority",
          "durability",
          "engine"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-proof-15-gate",
        "title": "artifact: Emit real 15-gate VerificationProof",
        "description": "Replace default-true or mismatched accepted-artifact proof data with a real 15-gate VerificationProof derived from boundedness, taint, action contracts, idempotency, durability, capability, observability, and replay/admission checks.",
        "acceptance_criteria": "Storage and runtime agree on gate count/schema; missing or failed gates reject with typed diagnostics; proof flags are derived from actual gate outputs; accepted artifacts from storage pass strict runtime admission without dummy proof stores.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:00:48Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:33:45Z",
        "labels": [
          "admission",
          "artifact",
          "core-priority",
          "engine",
          "verification"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-strict-ack-ordering",
        "title": "runtime/storage: Prove strict persistence before acknowledgement ordering",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Prove strict persistence-before-ack ordering for submit, action completion, action failure, wait resume, ask answer, retry, cancel, and terminal mutations.",
        "acceptance_criteria": "Every strict submit/action/wait/ask/retry/cancel/terminal mutation persists required journal/storage evidence before acknowledgement or externally visible state; injected persistence failure returns typed fail-closed error without in-memory-only acknowledged mutation; restart evidence matches acknowledged state.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:27:38Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T04:59:49Z",
        "labels": [
          "core-priority",
          "durability",
          "engine",
          "runtime",
          "storage",
          "strict"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.12.2",
        "title": "runtime/storage: Propagate journal and storage failures",
        "description": "Replace silent journal/storage failure paths with typed errors that preserve operation, run id, record kind, and persistence boundary. Runtime must not acknowledge success after a failed durable write.",
        "acceptance_criteria": "Journal and storage write failures return typed errors to the caller; tests cover failing writer/queue paths; no affected path logs-and-continues or drops Result.",
        "notes": "femdation BLOCK_LOCAL 2026-05-15: refused to resume because forbidden source-checkout artifact exists at /home/lewis/src/velvet-ballistics/.beads/vb-qi37.12.2/STATE.md. Source checkout is control-plane only; needs cleanup/recovery before isolated go-skill resume.",
        "status": "blocked",
        "priority": 0,
        "issue_type": "bug",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:48:27Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T17:13:47Z",
        "labels": [
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "reliability",
          "runtime",
          "storage"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.4.2",
        "title": "runtime: Enforce admission gate before run creation",
        "description": "Require accepted artifacts for run creation and reject raw/unverified/malformed artifacts with typed diagnostics before runtime state allocation.",
        "acceptance_criteria": "Run creation fails for raw, failed, stale, or digest-mismatched artifacts; valid accepted artifacts proceed without runtime YAML/JSON parsing.",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:52:59Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:26Z",
        "labels": [
          "admission",
          "master-gap",
          "release-plan",
          "runtime",
          "storage"
        ],
        "dependency_type": "blocks"
      }
    ],
    "dependents": [
      {
        "id": "vb-core-cli-accepted-path",
        "title": "cli/runtime: Route YAML run and submit through accepted artifacts",
        "description": "Make cmd_run, cmd_submit, and strict direct run paths persist verified YAML source and accepted artifacts before runtime admission. Runs must bind by artifact digest, not loose YAML or raw CompiledWorkflow bypass.",
        "acceptance_criteria": "Strict YAML run/submit persists source, accepted artifact, run header, and RunAccepted; runtime admits by artifact digest through storage-backed admission; raw WorkflowParts or unverified compiled input is rejected in strict mode.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:01:02Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:33:45Z",
        "labels": [
          "admission",
          "cli",
          "core-priority",
          "durability",
          "engine",
          "runtime",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-engine-yaml",
        "title": "engine: Durable YAML runtime acceptance without UI or generated Rust",
        "description": "Scope bead for the user-requested finish line: fully working durable workflow engine from strict YAML authoring through compiled numeric IR, accepted artifact admission, bounded runtime, Fjall/Postcard durability, replay/recovery, direct API and CLI operator evidence, excluding UI and generated Rust/codegen/maxperf. This bead is the engine-only acceptance root; it does not replace full master release beads that still include generated Rust/UI.",
        "acceptance_criteria": "Close only when all dependency beads are closed and direct evidence proves: strict YAML compile/validate/lowering, no runtime YAML/JSON/HTTP, accepted artifacts with idempotency/capability evidence, bounded deterministic runtime, typed durable recovery/fail-closed replay, CLI diagnostics/emits for operator workflow, IPC/direct API where already scoped, and engine-scoped CI/fuzz/Miri/coverage/mutation/supply/perf evidence. Do not require UI or generated Rust/codegen parity.",
        "notes": "Planner dependency correction 2026-05-13: converted to epic because it is an acceptance root/milestone, not an implementation feature. bd only permits epics to block epics, and this root must block master epic vb-qi37 so bd ready does not expose the master epic as implementable work.\nPlanner validation 2026-05-13: ran planner session vb-engine-yaml-planner through init, add-task CUE validation, quality-gate, generate-bead, validate, and report. Quality gate PASS 97/100. Generated planner bead spec velvet-ballistics-20260513114348-68dfgoqs and schema .beads/schemas/velvet-ballistics-20260513114348-68dfgoqs.cue. Intentionally did not run planner create because that would duplicate existing vb-engine-yaml and cannot preserve explicit bd id, parent, or dependency edges. vb-engine-yaml now blocks vb-qi37 to keep bd ready focused on implementable work.\nPlanner correction 2026-05-13: removed vb-engine-yaml -\u003e vb-qi37 dependency edge because it suppressed schedulable child work through parent inheritance. vb-engine-yaml remains parented under vb-qi37 and carries the engine-only dependency closure; master epic is status-blocked instead of dependency-blocked.\nDependency hygiene 2026-05-13: converted from epic back to feature because bd prevents epics from depending on task/feature implementation leaves. This bead is an engine-only acceptance gate, not a work item. Implementation leaves are kept schedulable by removing parent links that made open aggregate beads block their own children; explicit dependency edges preserve the aggregate closure.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-13T16:38:05Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:33:45Z",
        "labels": [
          "admission",
          "boundedness",
          "cli",
          "durability",
          "engine",
          "no-codegen",
          "no-ui",
          "quality",
          "recovery",
          "release-plan",
          "runtime",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.4",
        "title": "runtime/storage: Prove accepted-artifact admission and run-header persistence",
        "description": "Implement the accepted-artifact admission gate and prove run header persistence before runtime acknowledgement. Admission must reject unverified artifacts, persist run metadata/digests, and expose typed errors for durability failures.",
        "acceptance_criteria": "Runtime accepts only verified artifacts with matching digest/certificate/capabilities; run headers are persisted before ack; simulated storage failure returns a typed admission error and no partial runnable state; tests cover accepted, rejected, digest mismatch, duplicate run id, and crash-after-admission recovery.",
        "notes": "Source audit: storage admission/header APIs and submit_artifact tests exist; scope is strict runtime admission, before-ack run-header durability, typed failure propagation, and end-to-end evidence.\nWIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.\nDependency hygiene 2026-05-13: reopened because admission blockers remain open: vb-qi37.4.1, vb-qi37.4.2, plus prerequisite whole-workflow boundedness vb-qi37.2, idempotency vb-qi37.5, and capability enforcement vb-qi37.6. Parent cannot close until accepted-artifact admission is fully proven.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:11Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "admission",
          "core-priority",
          "durability",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "runtime",
          "storage",
          "verification"
        ],
        "dependency_type": "blocks"
      }
    ]
  }
]
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-atomic-admission/.beads

```
