bead_id: vb-core-cli-accepted-path
bead_title: vb-core-cli-accepted-path
phase: 1
updated_at: 2026-05-15T19:35:58.424429+00:00
attempt: 1-of-7

# Baseline report

Baseline captured before proof/test/implementation edits in isolated workspace.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path
base_revision_command: `jj log -r @- --no-graph -T ...`

## bd show vb-core-cli-accepted-path --json
exit=1
```json
{
  "error": "no issues found matching the provided IDs"
}
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/.beads
Error fetching vb-core-cli-accepted-path: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues

```

## jj status
exit=0
```text
The working copy has no changes.
Working copy  (@) : xzkslrxw 98b06f50 (empty) (no description set)
Parent commit (@-): moyvrvsn c9c7eee4 main | Delete CHANGELOG.md

```

## jj base revision (@-)
exit=0
```text
moyvrvsnmmzt c9c7eee46ef4 Delete CHANGELOG.md

```

## Baseline classification note

All 20 P0 go-skill workspaces were created from `trunk()` before bead-local edits. The canonical machine gate will be executed and compared in State 11; this report preserves pre-edit bead/workspace identity and base revision evidence for regression classification.

## Corrected bd show vb-core-cli-accepted-path --json using source server-mode DB
updated_at=2026-05-15T19:37:45.053546+00:00
command=`bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-cli-accepted-path --json`
exit=0
```json
[
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
        "id": "vb-f04l",
        "title": "compiler: Safe v1 primitive source lowering",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-ainhmecv.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-ainhmecv.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509012640-ainhmecv\"\n  title: \"compiler: Safe v1 primitive source lowering\"\n  type: \"feature\"\n  priority: 0\n  effort_estimate: \"4hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL compile all v1 primitives from AST to numeric IR.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN a valid AST contains a v1 primitive\\\", shall: \\\"THE SYSTEM SHALL emit the mathematically equivalent CompiledNodeKind IR.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF existing test coverage would be removed\\\", shall_not: \\\"THE SYSTEM SHALL NOT delete legacy compiler files\\\", because: \\\"we must not regress compiler validation safety during the migration.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"YAML parser and AST validator are complete and strict.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"All v1 primitives successfully lower to valid IR.\\\",\n        \\\"Existing tests pass.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"Numeric IR indices are dense and valid.\\\",\n      \\\"No untested primitives are reachable.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/vb_compile/src/lower.rs\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/vb_compile/src/api_build2.rs\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      {question: \\\"Which specific tests were disabled in vb-ygy2?\\\", answered: false}\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_lower_foreach_to_ir.\\\", given: \\\"Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.\\\", when: \\\"Lower ForEach to IR.\\\", then: [\\\"Lower ForEach to IR.\\\"], real_input: \\\"Task scope: Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.. Relevant files: crates/vb_core/src/nodes.rs.\\\", expected_output: \\\"Lower ForEach to IR.\\\"},\\n      {name: \\\"test_lower_collect_to_ir.\\\", given: \\\"Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.\\\", when: \\\"Lower Collect to IR.\\\", then: [\\\"Lower Collect to IR.\\\"], real_input: \\\"Task scope: Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.. Relevant files: crates/vb_core/src/nodes.rs.\\\", expected_output: \\\"Lower Collect to IR.\\\"},\\n      {name: \\\"test_lower_together_to_ir.\\\", given: \\\"Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.\\\", when: \\\"Lower Together to IR.\\\", then: [\\\"Lower Together to IR.\\\"], real_input: \\\"Task scope: Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.. Relevant files: crates/vb_core/src/nodes.rs.\\\", expected_output: \\\"Lower Together to IR.\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_reject_ast_primitives_missing_required_fields_during_lowering_if_ast_validation_\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Reject AST primitives missing required fields during lowering if AST validation missed them.\\\", then: [\\\"Reject AST primitives missing required fields during lowering if AST validation missed them.\\\"], real_input: \\\"Task scope: Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.. Relevant files: crates/vb_core/src/nodes.rs.\\\", expected_output: null, expected_error: \\\"Reject AST primitives missing required fields during lowering if AST validation missed them.\\\"},\\n      {name: \\\"test_reject_invalid_internal_structures_during_foreach_construction.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Reject invalid internal structures during ForEach construction.\\\", then: [\\\"Reject invalid internal structures during ForEach construction.\\\"], real_input: \\\"Task scope: Unblock vb-ygy2 by implementing safe AST-to-IR lowering for all v1 primitives in vb_compile. Do NOT delete existing api_build2.rs or lower.rs files until the new paths are fully covered by identical or superseding test cases. Target full v1 primitive parity (ForEach, Together, Collect, Reduce, Repeat, etc.) while preserving all existing schema and lowering tests.. Relevant files: crates/vb_core/src/nodes.rs.\\\", expected_output: null, expected_error: \\\"Reject invalid internal structures during ForEach construction.\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of compiler: Safe v1 primitive source lowering\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Implement missing lowering logic.\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Audit existing lower.rs vs v1 primitive spec.\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write lowering tests for missing v1 primitives.\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Implement missing lowering logic.\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_core/src/nodes.rs\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_core/src/nodes.rs\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509012640-ainhmecv/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      {path: \\\"crates/vb_core/src/nodes.rs\\\", relevance: \\\"Related implementation\\\"}\n    ]\n    similar_implementations: [\n      \\\"Existing Choose and SetConst lowering.\\\"\n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:26:41Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T19:34:27Z",
        "labels": [
          "master-gap",
          "mvp-feature-now"
        ],
        "dependency_type": "blocks"
      }
    ],
    "dependents": [
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
      }
    ]
  }
]
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-cli-accepted-path/.beads

```
