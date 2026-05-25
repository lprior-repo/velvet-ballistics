bead_id: vb-engine-yaml
bead_title: vb-engine-yaml
phase: 1
updated_at: 2026-05-15T19:35:59.585105+00:00
attempt: 1-of-7

# Baseline report

Baseline captured before proof/test/implementation edits in isolated workspace.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml
base_revision_command: `jj log -r @- --no-graph -T ...`

## bd show vb-engine-yaml --json
exit=1
```json
{
  "error": "no issues found matching the provided IDs"
}
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml/.beads
Error fetching vb-engine-yaml: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues

```

## jj status
exit=0
```text
The working copy has no changes.
Working copy  (@) : qrorzttn 07479848 (empty) (no description set)
Parent commit (@-): moyvrvsn c9c7eee4 main | Delete CHANGELOG.md

```

## jj base revision (@-)
exit=0
```text
moyvrvsnmmzt c9c7eee46ef4 Delete CHANGELOG.md

```

## Baseline classification note

All 20 P0 go-skill workspaces were created from `trunk()` before bead-local edits. The canonical machine gate will be executed and compared in State 11; this report preserves pre-edit bead/workspace identity and base revision evidence for regression classification.

## Corrected bd show vb-engine-yaml --json using source server-mode DB
updated_at=2026-05-15T19:37:45.053546+00:00
command=`bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-engine-yaml --json`
exit=0
```json
[
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
    "dependencies": [
      {
        "id": "vb-ahfl",
        "title": "engine: End-to-end YAML to IR semantic evidence",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-mvegej3o.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-mvegej3o.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509012640-mvegej3o\"\n  title: \"engine: End-to-end YAML to IR semantic evidence\"\n  type: \"feature\"\n  priority: 0\n  effort_estimate: \"4hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL guarantee semantic fidelity from YAML to execution.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN a YAML workflow is executed\\\", shall: \\\"THE SYSTEM SHALL produce the exact step events mandated by the YAML definition.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF a primitive behaves differently than its YAML spec\\\", shall_not: \\\"THE SYSTEM SHALL NOT silently accept it\\\", because: \\\"IR lowering must be lossless.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"YAML parser is strict.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"E2E test suite asserts YAML vs Journal output.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"YAML structure dictates exact journal signature.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/velvet_ballistics/tests/\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      {question: \\\"What is the best way to assert journal signatures from YAML?\\\", answered: false}\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_yaml_with_do/set/finish_maps_perfectly_to_journal.\\\", given: \\\"Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.\\\", when: \\\"YAML with Do/Set/Finish maps perfectly to journal.\\\", then: [\\\"YAML with Do/Set/Finish maps perfectly to journal.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: \\\"YAML with Do/Set/Finish maps perfectly to journal.\\\"},\\n      {name: \\\"test_yaml_with_collect_produces_correct_slot_extra_data.\\\", given: \\\"Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.\\\", when: \\\"YAML with Collect produces correct slot extra data.\\\", then: [\\\"YAML with Collect produces correct slot extra data.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: \\\"YAML with Collect produces correct slot extra data.\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_invalid_yaml_execution_topology_is_caught_before_execution.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Invalid YAML execution topology is caught before execution.\\\", then: [\\\"Invalid YAML execution topology is caught before execution.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: null, expected_error: \\\"Invalid YAML execution topology is caught before execution.\\\"},\\n      {name: \\\"test_type_mismatches_in_yaml_are_rejected_in_lowering_or_static_check.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Type mismatches in YAML are rejected in lowering or static check.\\\", then: [\\\"Type mismatches in YAML are rejected in lowering or static check.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: null, expected_error: \\\"Type mismatches in YAML are rejected in lowering or static check.\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of engine: End-to-end YAML to IR semantic evidence\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Wire into CI.\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Design YAML-to-Journal assertion harness.\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write E2E test cases for all major primitives.\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Wire into CI.\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_storage/src/journal.rs\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_storage/src/journal.rs\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509012640-mvegej3o/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      {path: \\\"crates/vb_storage/src/journal.rs\\\", relevance: \\\"Related implementation\\\"}\n    ]\n    similar_implementations: [\n      \\\"CLI simulate tests.\\\"\n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:26:42Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "core-priority",
          "engine",
          "master-gap",
          "mvp-feature-now",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
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
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-bd-reliability",
        "title": "ops: Prove bd Dolt graph reliability",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Prove bd/Dolt reliability for the P0 graph: reproducible readonly graph queries, no cycles, clean lock behavior, successful bd dolt push path, and no runtime database state committed to git.",
        "acceptance_criteria": "P0 graph can be queried reproducibly; bd dep cycles is empty; bd dolt push path has command evidence; lock/corruption failure modes have recovery steps; no .beads runtime state is tracked by git.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:23Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:10Z",
        "labels": [
          "beads",
          "core-priority",
          "dolt",
          "engine",
          "reliability"
        ],
        "dependency_type": "blocks"
      },
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
        "id": "vb-core-ipc-loom-property",
        "title": "ipc/orchestrator: Add production Loom property evidence",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Add production-connected Loom/property tests for cancel versus completion, shutdown drain reports, timer ordering, bounded queue backpressure, and slow-client IPC behavior.",
        "acceptance_criteria": "Tests exercise production queue/orchestrator paths or faithful adapters; cancel/completion, shutdown drain, timer ordering, queue backpressure, and slow-client behavior are deterministic and fail with typed errors rather than unbounded buffering.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:37Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:10Z",
        "labels": [
          "backpressure",
          "core-priority",
          "engine",
          "ipc",
          "loom",
          "orchestrator",
          "sync"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-ipc-sync-evidence",
        "title": "ipc/orchestrator: Prove local binary ingress synchronization",
        "description": "Prove local binary IPC ingress and runtime orchestration semantics for core engine use: compiled/artifact digest submit, bounded ingress, cancellation versus completion, timer ordering, shutdown drain, and slow-client/backpressure behavior.",
        "acceptance_criteria": "IPC SubmitRun or equivalent artifact-digest path reaches strict runtime admission; bounded queues reject with typed backpressure; cancel/completion/timer/shutdown races are deterministic and tested; no task-per-step or unbounded buffer behavior appears.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:01:13Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:33:45Z",
        "labels": [
          "backpressure",
          "core-priority",
          "engine",
          "ipc",
          "orchestrator",
          "runtime",
          "sync"
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
        "id": "vb-core-proof-gate-inputs",
        "title": "artifact: Derive all VerificationProof gate inputs",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Derive missing real 15-gate VerificationProof inputs from taint, action contracts, durability, observability, replay/admission, plus existing boundedness/idempotency/capability gates.",
        "acceptance_criteria": "Every proof gate has a concrete producer and failing test; default-true gates are impossible; missing taint/action/durability/observability/replay evidence rejects accepted artifacts with typed diagnostics.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:24Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:10Z",
        "labels": [
          "artifact",
          "core-priority",
          "engine",
          "proof",
          "verification"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-replay-divergence-recovery",
        "title": "recovery: Prove typed replay divergence and no-YAML recovery",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Prove recovery from persisted source/artifact/journal/snapshot data without YAML reparsing, including typed replay divergence, digest mismatch, snapshot+tail hydration, and fail-closed incomplete frame recovery.",
        "acceptance_criteria": "Restart/replay never reparses YAML; snapshot+tail hydrates full frame state; digest mismatch and semantic divergence produce typed errors; corrupt/incomplete frame recovery fails closed.",
        "status": "closed",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:24Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:49:11Z",
        "closed_at": "2026-05-15T05:49:11Z",
        "close_reason": "S1-S15 complete: recovery logic proven, 14 obligations (1 PASS, 13 FAIL_LOCAL waived), black-hat APPROVED, final-evidence-decision APPROVED",
        "labels": [
          "core-priority",
          "durability",
          "engine",
          "recovery",
          "replay",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-storage-artifact-store",
        "title": "runtime/storage: Use StorageArtifactStore for strict admission",
        "description": "Ensure strict and journaled production runtime construction loads accepted artifacts through StorageArtifactStore instead of AlwaysPresentArtifactStore. Keep dummy stores test-only or relaxed-only.",
        "acceptance_criteria": "Strict runtime rejects missing/malformed artifacts from storage; CLI strict and journaled paths construct runtime with storage-backed artifact loading; AlwaysPresentArtifactStore cannot satisfy strict production admission.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
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
          "runtime",
          "storage"
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
        "id": "vb-core-trigger-contract",
        "title": "yaml: Align manual schedule event webhook triggers",
        "description": "Align trigger support across vb_yaml, vb_validate, and vb_compile with the master v1 contract: manual, schedule, event, webhook. Remove YAML ipc as an authoring trigger or make it non-authoring IPC-only.",
        "acceptance_criteria": "All three crates share the same trigger contract; manual/schedule/event/webhook have positive tests; ipc/http or unsupported triggers fail with typed diagnostics; runtime still does not parse HTTP or YAML.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "velvet-ballistics agent",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:00:47Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T17:04:10Z",
        "labels": [
          "compiler",
          "core-priority",
          "engine",
          "no-codegen",
          "triggers",
          "validation",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-yaml-ast",
        "title": "yaml: Canonicalize authoring AST across parser validator compiler",
        "description": "Make vb_yaml WorkflowSource the canonical cold authoring AST, or intentionally remove the duplicate compiler YAML grammar. vb_compile and vb_validate must consume one trigger, step, value, and reference model before runtime admission.",
        "acceptance_criteria": "Parser, validator, and compiler accept/reject the same v1 YAML shapes; author YAML no longer requires low-level numeric slots/actions; runtime crates still have no YAML dependency; tests prove canonical AST handoff.",
        "notes": "Completed local vb_yaml hardening slice: strict top-level and step unknown-field rejection, non-string key shape errors, try_again YAML parsing with retry rejected as unknown, focused tests added/updated. Gates run: rtk cargo test -p vb_yaml PASS (271 passed), rtk cargo fmt --check -p vb_yaml PASS after formatting, rtk cargo check -p vb_yaml --all-targets --all-features PASS, strict rtk cargo clippy -p vb_yaml --lib --all-features PASS.\nImplemented additional canonical AST slices. vb_yaml: removed dead ast_parse/ast_helpers duplicate parser files; master v1 triggers now parse as manual/schedule/event/webhook with ipc/http rejected as unsupported YAML triggers; trigger maps reject unknown keys and extra body fields; top-level steps is required and must be a sequence; unknown top-level/step fields and non-string keys are rejected; exactly one step primitive is enforced across canonical names and aliases; aliases run-\u003edo, foreach-\u003efor_each, save-\u003eset are normalized; value-only save is rejected; YAML retry is rejected and try_again is parsed. vb_compile: added vb_yaml dependency and public compile_source(\u0026vb_yaml::ast::WorkflowSource) as a narrow canonical AST handoff seam while leaving compile_workflow(bytes) unchanged; supports integer set constants plus final finish slot references, validates workflow/step names and duplicate ids, rejects unsupported top-level declarations/controls/primitives, uses deterministic AST-handoff digest including trigger kind/payload, and validates produced IR. Main-thread focused gates PASS: rtk cargo test -p vb_yaml (276 passed); rtk cargo test -p vb_compile compile_source (9 passed); rtk cargo fmt --check -p vb_yaml; rtk cargo fmt --check -p vb_compile; rtk cargo check -p vb_yaml --all-targets --all-features; rtk cargo check -p vb_compile --all-targets --all-features; strict rtk cargo clippy for vb_yaml/vb_compile libs. Remaining scope: compile_source is intentionally narrow and does not yet replace compiler byte parser; vb_yaml still needs recursive AuthorValue/RawExpr/Reference, spans/source marks, mapping-style inputs/vars/secrets, and broader primitive/control lowering before deleting vb_compile's duplicate YAML grammar/profile path. Full moon ci not run.\nFinal delivery: canonical vb_yaml authoring AST is now the compiler admission path. YamlCompiler::compile is canonical-only (UTF-8 -\u003e vb_yaml::parse_workflow_source -\u003e compile_source) with no raw/Saphyr/phase-zero fallback. WorkflowSource is opaque with accessors; raw AST parser is crate-private; v1 triggers/manual/schedule/event.type/webhook are canonical; ipc/http rejected; recursive AuthorValue and mapping inputs/vars/secrets added; strict unknown-field/shape checks, exactly-one primitive, aliases save/run/foreach normalized; event-backed semantic source map with byte spans added. vb_compile has compile_source(\u0026WorkflowSource), named set.output -\u003e finish.result lowering to numeric slots, duplicate/unknown output errors, canonical digest sensitivity, and raw compiler examples hardened. Runtime crates remain YAML-free. Test-reviewer and black-hat reviewer approved the no-fallback/source-map/named-output path. moon ci PASS with TMPDIR=/home/lewis/src/velvet-ballistics/target/tmp: 20 tasks completed, 8346 tests passed, 6 skipped; miri, mutants-smoke, coverage, doc/doc-test, maxperf, hardened-build, fmt/lint/check all passed. Pushed verified jj change romqzowq/461167dd to remote main.\nTruth-serum cleanup after initial push: removed orphaned/dead vb_yaml AST test debris that still referenced legacy TriggerAst::Ipc; active parser tests remain in lib_tests.rs. Aligned vb_validate trigger schema with canonical v1 YAML: accepts manual/schedule/event/webhook, rejects ipc as UnsupportedTrigger, keeps http as HttpTriggerOutOfCore, preserves multiple-trigger rejection, and validates minimal trigger body shapes (schedule.cron, event.type, empty manual/webhook). Verified no TriggerAst::Ipc/canonical_can_fallback/text.find in vb_yaml source and no stale manual|ipc validator acceptance. moon ci PASS with TMPDIR=/home/lewis/src/velvet-ballistics/target/tmp: 19 tasks completed, 8358 tests passed, 6 skipped; fmt/lint/check/miri/coverage/fuzz-smoke/test/doc/doc-test/maxperf/hardened-build passed. Pushed cleanup change wumlxqnl/e71c14a3 to remote main.",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:00:47Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T15:47:40Z",
        "closed_at": "2026-05-15T13:19:41Z",
        "close_reason": "Completed canonical AST handoff; moon ci green; pushed to remote main",
        "labels": [
          "compiler",
          "core-priority",
          "engine",
          "no-codegen",
          "validation",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-yaml-e2e-chain",
        "title": "engine: Prove YAML-origin Fjall runtime inspect events recovery chain",
        "description": "Add executable evidence for the core engine chain: YAML validate, compile, accepted artifact, Fjall persistence, strict runtime execution, journal/events, inspect, replay, recovery, and no YAML reparsing during recovery.",
        "acceptance_criteria": "A YAML-origin strict run completes or suspends/resumes through runtime; persisted journal/events/inspect prove digest binding; restart/replay recovers without YAML parsing; corrupt or mismatched source/artifact digests fail with typed errors.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:01:13Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:33:45Z",
        "labels": [
          "core-priority",
          "e2e",
          "engine",
          "events",
          "fjall",
          "recovery",
          "runtime",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-iucs",
        "title": "P0 repair proof integration after verifier rejection",
        "description": "Formal-verifier and contract-verification-reviewer rejected current proof/code delta. Repair Rust build integration first, then tighten proof artifacts around production behavior.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-14T01:33:09Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-14T01:33:38Z",
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37",
        "title": "release: Master-doc completion gap plan",
        "description": "Parent epic for all remaining work discovered by comparing the current bead graph against velvet-ballistics-MASTER.md. Existing beads cover six P0 evidence/parity streams and most Makepad UI screens, but missing beads are required for recovery hydration, whole-workflow boundedness, idempotency/admission/capability gates, IR structural validation, CLI operator commands, UI token/screen gaps, and final quality evidence gates.",
        "acceptance_criteria": "All master-doc remaining implementation themes are represented by child beads or existing linked beads; dependencies order foundation work before evidence gates; bd ready exposes the next highest-risk work.",
        "notes": "Dependency hygiene 2026-05-13: reopened because this epic still has open child work and cannot honestly represent complete master-doc coverage. User requested a no-UI/no-codegen engine+YAML dependency setup; full master release remains open until UI/codegen/full evidence close or are explicitly split/deferred.\nPlanner correction 2026-05-13: removed vb-engine-yaml dependency edge because parent-blocking propagated through the epic tree and suppressed ready child work. Marked master epic blocked by status instead; actual implementation dependencies remain on vb-engine-yaml and child beads.\nPlanner correction 2026-05-13: changed master epic from blocked to in_progress because blocked status on an epic suppresses ready descendants. This epic is an umbrella tracker; leaf/parent dependency beads should drive bd ready.",
        "status": "closed",
        "priority": 0,
        "issue_type": "epic",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:34:26Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-13T16:48:48Z",
        "closed_at": "2026-05-13T16:48:48Z",
        "close_reason": "Planning epic complete: remaining implementation themes are represented by child beads and engine-only acceptance root vb-engine-yaml. Force-close is intentional because this umbrella epic has open children by design; implementation acceptance remains open on child beads.",
        "labels": [
          "master-gap",
          "release-plan"
        ],
        "dependency_type": "parent-child"
      },
      {
        "id": "vb-qi37.1",
        "title": "runtime/storage: Complete full live-frame recovery hydration",
        "description": "Close DRIFT-2. Persist deterministic step started/succeeded events plus every deterministic slot write value and taint; hydrate RunFrame from journal/snapshot data; reject unsupported or incomplete recovery instead of proceeding with empty frames; prove replay/recovery digest mismatch detection.",
        "acceptance_criteria": "Crash recovery reconstructs pc, slot handles, slot taints, step state, journal sequence, action tickets, waits/asks, and terminal result; hydrate_run_frame cannot silently return an empty live frame for a non-empty run; tests cover crash-before-ack, crash-after-ack, corrupt journal, and snapshot+journal replay.",
        "notes": "Source audit: storage replay/recovery summaries and digest mismatch checks exist, but object/list slot replay is explicitly unsupported in tests and full RunFrame hydration still needs fail-closed recovery evidence.\nWIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.\nDependency hygiene 2026-05-13: reopened because durable recovery blockers remain open: vb-qi37.1.4, vb-qi37.1.5, vb-qi37.1.6, and reliability blocker vb-qi37.12. Parent cannot close until fail-closed recovery, digest mismatch, crash restart evidence, and silent-discard elimination are complete.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:34:47Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "master-gap",
          "mvp-feature-now",
          "recovery",
          "release-plan",
          "runtime"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.12",
        "title": "runtime/storage: Eliminate silent discard paths",
        "description": "Cover the explicit silent-discard-elimination gap. Audit runtime/storage/compiler paths for ignored Results, dropped journal failures, swallowed action/recovery errors, and lossy diagnostics.",
        "acceptance_criteria": "No first-party runtime/storage/compiler path silently drops fallible outcomes; ignored results are forbidden or justified by typed discard APIs; tests inject journal/storage/action failures and assert typed errors reach the caller and evidence chain.",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "bug",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:36:18Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
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
        "id": "vb-qi37.16",
        "title": "cli/runtime: Implement durable lifecycle state transitions",
        "description": "Implement durable operator lifecycle controls: cancel, resume, retry, and answer. These must be backed by runtime state transitions and journal evidence, not UI-only or text command routing.",
        "acceptance_criteria": "cancel/resume/retry/answer commands mutate durable runtime state through typed Direct API/IPC paths; stale completions and duplicate answers are rejected; lifecycle actions survive recovery; tests cover happy paths, unauthorized/invalid transitions, and replay.",
        "notes": "Source audit: retry/resume report what would happen; answer is explicitly not implemented. Scope is durable cancel/resume/retry/answer state transitions with journal evidence.\nWIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:36:51Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-13T06:04:44Z",
        "closed_at": "2026-05-13T06:04:44Z",
        "close_reason": "Landed: cli/runtime: Implement durable lifecycle state transitions. Evidence packaging complete. All tests pass. Black-hat review APPROVED. Post-landing: vb-qi37.1 and vb-qi37.13 are unblocked independently.",
        "labels": [
          "cli",
          "core-priority",
          "durability",
          "lifecycle",
          "master-gap",
          "mvp-feature-now",
          "orchestration",
          "release-plan",
          "runtime"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.2",
        "title": "runtime: Prove whole-workflow boundedness and resource caps",
        "description": "Close DRIFT-3. Add aggregate resource accounting across nested workflow composition, per-run ValueStore/arena caps, hard step budget ceilings, bounded collect/reduce/repeat/together behavior, and safe defaults that are not effectively unbounded.",
        "acceptance_criteria": "A workflow-level bound certificate is computed before admission; runtime enforces arena/value/step/event/action budgets; nested composition cannot exceed aggregate caps; adversarial collect/reduce/repeat/together tests fail safely with typed errors and no panic or unbounded allocation.",
        "notes": "Source audit: WholeWorkflowBudget, BoundednessPolicy, ResourceContract, and validation checks exist; scope is adversarial aggregate proof, per-run arena enforcement, nested composition limits, and no effectively-unbounded defaults.\nWIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:34:53Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "boundedness",
          "core-priority",
          "engine",
          "master-gap",
          "mvp-feature-now",
          "orchestration",
          "release-plan",
          "runtime"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.3",
        "title": "runtime: Prove collect pagination durability and hydration",
        "description": "Implement the missing runtime collect next-page/pagination state required by the master doc. Replace any global or ambiguous collect state with per-run/per-node bounded state and verify resume/replay behavior.",
        "acceptance_criteria": "Collect continuation state survives waits, replay, and recovery; pagination is isolated by run and node; stale or duplicated page completions are rejected with typed errors; tests cover empty page, final page, repeated page, out-of-order page, and recovery mid-collect.",
        "notes": "Source audit: collect pagination/per-run state exists in vb_runtime primitives with durable extras and hydrate helpers; remaining work is integration proof across journal, recovery, resume/replay, ordering, and bounds.",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:01Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-11T12:54:42Z",
        "closed_at": "2026-05-11T12:54:42Z",
        "close_reason": "Completed Go-skill States 1-15 through final QA; code and artifacts pushed to remote bookmark vb-qi37-3-landing at 7c12b98a. Known unrelated global FORMAT/CLIPPY/vb_ui_model debts are DEFERRED_GLOBAL under vb-bkgo.",
        "labels": [
          "collect",
          "core-priority",
          "durability",
          "engine",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "runtime"
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
      },
      {
        "id": "vb-qi37.5",
        "title": "verifier: Idempotency verification gate",
        "description": "Replace the master-doc stub idempotency gate with real static/runtime verification for retry-safe actions and replay-safe workflow behavior.",
        "acceptance_criteria": "Verification rejects non-idempotent actions in retryable positions unless an explicit safe policy exists; action contracts expose idempotency metadata; generated certificates include idempotency results; tests cover retry, duplicate completion, stale completion, and non-idempotent action rejection.",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:20Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "core-priority",
          "engine",
          "idempotency",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "verification",
          "verifier"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.6",
        "title": "verifier/runtime: Capability model enforcement",
        "description": "Implement the non-UI capability model required by the master doc. Capabilities must be validated, certified, admitted, and enforced by runtime/action dispatch rather than only displayed in the UI.",
        "acceptance_criteria": "Action contracts declare required capabilities; verifier rejects missing or excessive capability grants; accepted artifacts carry capability certificates; runtime dispatch checks capabilities with typed denial errors; UI action registry consumes the same typed capability data.",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:27Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "capability",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "runtime",
          "verifier"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.7",
        "title": "ir: Structural validation for untrusted artifacts",
        "description": "Close DRIFT-4. Upgrade IR validation from bounds-only checks to full structural validation before artifact loading/admission. Treat every loaded artifact as untrusted input.",
        "acceptance_criteria": "Compiled artifact construction validates reachability, edge targets, loop pairing, symbol id ranges, accessor path segments, terminal nodes, and action/resource references; invalid artifacts fail with precise typed diagnostics; fuzz/property tests cannot construct invalid executable IR through public loaders.",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:35Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-13T15:40:29Z",
        "closed_at": "2026-05-13T15:40:29Z",
        "close_reason": "Completed structural validation for untrusted artifacts: child beads vb-qi37.7.1 through vb-qi37.7.5 are closed; latest public artifact harness covers run-compiled admission across reachability, edges, references, and accessors; moon ci passed (17 completed, 1 cached, 8296 tests passed).",
        "labels": [
          "ir",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "validation"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-qi37.8",
        "title": "validate/compile: Prove and complete shared validation pipeline",
        "description": "Close DRIFT-5. Remove duplicated validation logic between vb_validate and vb_compile by introducing or using a shared validated intermediate representation while preserving public APIs.",
        "acceptance_criteria": "vb_validate and vb_compile use one authoritative validation path for shared checks; diagnostics remain stable; duplicate rule drift tests prove the same invalid workflow cannot be accepted by one path and rejected by the other; public API compatibility is preserved unless explicitly updated in the master doc.",
        "notes": "Source audit: vb_validate::shared exists and is used by validation paths; scope is residual duplication removal, contract-gate coverage, compile/validate parity, and public API preservation.",
        "status": "closed",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:42Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-13T16:12:57Z",
        "closed_at": "2026-05-13T16:12:57Z",
        "close_reason": "Completed shared validation pipeline proof: dependencies vb-9ret and vb-yd5x are closed; public validate/compile adapters preserve APIs and diagnostics; exact acceptance tests and full pipeline pass; dead duplicate compile paths removed; moon ci passed (18 completed, 1 cached, 8301 tests passed).",
        "labels": [
          "compile",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "validation"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-x16d",
        "title": "ipc: Frame fuzzing and backpressure evidence",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-mrfxabfi.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-mrfxabfi.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509012640-mrfxabfi\"\n  title: \"ipc: Frame fuzzing and backpressure evidence\"\n  type: \"feature\"\n  priority: 0\n  effort_estimate: \"4hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL maintain IPC stability under adversarial load.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN the memory ingress queue is full\\\", shall: \\\"THE SYSTEM SHALL apply backpressure and reject new frames.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF malformed frames are sent\\\", shall_not: \\\"THE SYSTEM SHALL NOT crash the IPC server\\\", because: \\\"the server must be resilient.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"IPC socket loop exists.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"Fuzz targets exist for IPC frames.\\\",\n        \\\"Backpressure tests exist.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"Memory limits are strictly enforced.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/vb_ipc/src/\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      {question: \\\"How is the ingress queue bound defined?\\\", answered: false}\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_valid_frames_processed_normally.\\\", given: \\\"Fulfill MASTER.md requirement for IPC fuzzing and backpressure. Provide mechanical evidence of socket loop frame fuzzing and backpressure handling (memory ingress queues).\\\", when: \\\"Valid frames processed normally.\\\", then: [\\\"Valid frames processed normally.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for IPC fuzzing and backpressure. Provide mechanical evidence of socket loop frame fuzzing and backpressure handling (memory ingress queues).. Relevant files: crates/vb_ipc/src/server/.\\\", expected_output: \\\"Valid frames processed normally.\\\"},\\n      {name: \\\"test_valid_batched_frames_processed_in_order.\\\", given: \\\"Fulfill MASTER.md requirement for IPC fuzzing and backpressure. Provide mechanical evidence of socket loop frame fuzzing and backpressure handling (memory ingress queues).\\\", when: \\\"Valid batched frames processed in order.\\\", then: [\\\"Valid batched frames processed in order.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for IPC fuzzing and backpressure. Provide mechanical evidence of socket loop frame fuzzing and backpressure handling (memory ingress queues).. Relevant files: crates/vb_ipc/src/server/.\\\", expected_output: \\\"Valid batched frames processed in order.\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_queue_full_returns_backpressure_error.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Queue full returns backpressure error.\\\", then: [\\\"Queue full returns backpressure error.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for IPC fuzzing and backpressure. Provide mechanical evidence of socket loop frame fuzzing and backpressure handling (memory ingress queues).. Relevant files: crates/vb_ipc/src/server/.\\\", expected_output: null, expected_error: \\\"Queue full returns backpressure error.\\\"},\\n      {name: \\\"test_payload_exceeding_bounds_drops_connection_safely.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Payload exceeding bounds drops connection safely.\\\", then: [\\\"Payload exceeding bounds drops connection safely.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for IPC fuzzing and backpressure. Provide mechanical evidence of socket loop frame fuzzing and backpressure handling (memory ingress queues).. Relevant files: crates/vb_ipc/src/server/.\\\", expected_output: null, expected_error: \\\"Payload exceeding bounds drops connection safely.\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of ipc: Frame fuzzing and backpressure evidence\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Integrate fuzz targets into CI.\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Write cargo-fuzz targets for IPC frames.\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write deterministic backpressure tests.\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Integrate fuzz targets into CI.\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_ipc/src/server/\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_ipc/src/server/\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509012640-mrfxabfi/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      {path: \\\"crates/vb_ipc/src/server/\\\", relevance: \\\"Related implementation\\\"}\n    ]\n    similar_implementations: [\n      \\\"Existing payload validation tests.\\\"\n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:26:42Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-11T19:45:15Z",
        "closed_at": "2026-05-11T19:45:15Z",
        "close_reason": "Completed: IPC frame bounds/backpressure tests and fuzz entrypoints are present; fuzz-smoke and moon ci --force passed 19/19 after commit 6090845a6.",
        "labels": [
          "master-gap",
          "mvp-feature-now"
        ],
        "dependency_type": "blocks"
      }
    ],
    "parent": "vb-qi37"
  }
]
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml/.beads

```
