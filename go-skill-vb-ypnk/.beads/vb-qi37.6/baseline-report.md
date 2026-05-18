bead_id: vb-qi37.6
bead_title: vb-qi37.6
phase: 1
updated_at: 2026-05-15T19:36:02.444269+00:00
attempt: 1-of-7

# Baseline report

Baseline captured before proof/test/implementation edits in isolated workspace.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6
base_revision_command: `jj log -r @- --no-graph -T ...`

## bd show vb-qi37.6 --json
exit=1
```json
{
  "error": "no issues found matching the provided IDs"
}
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6/.beads
Error fetching vb-qi37.6: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues

```

## jj status
exit=0
```text
The working copy has no changes.
Working copy  (@) : zosumxrs 52c0fda2 (empty) (no description set)
Parent commit (@-): moyvrvsn c9c7eee4 main | Delete CHANGELOG.md

```

## jj base revision (@-)
exit=0
```text
moyvrvsnmmzt c9c7eee46ef4 Delete CHANGELOG.md

```

## Baseline classification note

All 20 P0 go-skill workspaces were created from `trunk()` before bead-local edits. The canonical machine gate will be executed and compared in State 11; this report preserves pre-edit bead/workspace identity and base revision evidence for regression classification.

## Corrected bd show vb-qi37.6 --json using source server-mode DB
updated_at=2026-05-15T19:37:45.053546+00:00
command=`bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.6 --json`
exit=0
```json
[
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
    "dependencies": [
      {
        "id": "vb-7ode",
        "title": "verifier/runtime: Enforce capabilities at action dispatch",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509015830-3fxfjfsu.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509015830-3fxfjfsu.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509015830-3fxfjfsu\"\n  title: \"verifier/runtime: Enforce capabilities at action dispatch\"\n  type: \"feature\"\n  priority: 1\n  effort_estimate: \"2hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL enforce capabilities before external action dispatch.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN runtime dispatches an action\\\", shall: \\\"THE SYSTEM SHALL compare certified requirements against granted run capabilities.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF a profile lacks a required capability\\\", shall_not: \\\"THE SYSTEM SHALL NOT dispatch the action.\\\", because: \\\"admission evidence must protect runtime side effects.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"Capability contract schema exists.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"Runtime dispatch rejects missing or revoked capabilities with typed diagnostics.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"Runtime cannot add capabilities not present in the accepted artifact and run profile.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/vb_runtime/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/vb_ipc/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/velvet_ballastics/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      \"No specific research questions defined\"\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_granted_capability_allows_dispatch\\\", given: \\\"Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.\\\", when: \\\"granted capability allows dispatch\\\", then: [\\\"granted capability allows dispatch\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: \\\"granted capability allows dispatch\\\"},\\n      {name: \\\"test_multiple_granted_capabilities_allow_dispatch\\\", given: \\\"Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.\\\", when: \\\"multiple granted capabilities allow dispatch\\\", then: [\\\"multiple granted capabilities allow dispatch\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: \\\"multiple granted capabilities allow dispatch\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_missing_capability_rejects_dispatch\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"missing capability rejects dispatch\\\", then: [\\\"missing capability rejects dispatch\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: null, expected_error: \\\"missing capability rejects dispatch\\\"},\\n      {name: \\\"test_revoked_capability_rejects_resumed_run\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"revoked capability rejects resumed run\\\", then: [\\\"revoked capability rejects resumed run\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: null, expected_error: \\\"revoked capability rejects resumed run\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of verifier/runtime: Enforce capabilities at action dispatch\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Implement to make tests pass\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Read relevant files and understand existing patterns\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write failing tests\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Implement to make tests pass\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_runtime/src\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_runtime/src\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509015830-3fxfjfsu/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      \n    ]\n    similar_implementations: [\n      \n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:58:32Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-10T19:50:01Z",
        "closed_at": "2026-05-10T19:50:01Z",
        "close_reason": "Closed",
        "labels": [
          "capability",
          "master-gap",
          "planner-shred",
          "release-plan"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-nsnc",
        "title": "verifier/runtime: Define capability contract schema",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509015830-ru0upgzt.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509015830-ru0upgzt.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509015830-ru0upgzt\"\n  title: \"verifier/runtime: Define capability contract schema\"\n  type: \"feature\"\n  priority: 1\n  effort_estimate: \"2hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL model capabilities as typed data, not stringly runtime checks.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN verification inspects an action\\\", shall: \\\"THE SYSTEM SHALL derive required capabilities and include them in the certificate.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF an action requires an undeclared capability\\\", shall_not: \\\"THE SYSTEM SHALL NOT admit the artifact.\\\", because: \\\"runtime must not execute unauthorized effects.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"ActionContract and verification certificate surfaces are known.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"Capability requirements are encoded in verifier output and accepted artifact metadata.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"A capability requirement is stable across compile, verify, admission, runtime, CLI, and UI views.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/vb_validate/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/vb_runtime/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/vb_core/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      \"No specific research questions defined\"\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_declared_capability_passes_verification\\\", given: \\\"Define typed capability requirements for actions and workflows, including resource, secret, storage, IPC, and external action capabilities carried through verification certificates and accepted artifacts.\\\", when: \\\"declared capability passes verification\\\", then: [\\\"declared capability passes verification\\\"], real_input: \\\"Task scope: Define typed capability requirements for actions and workflows, including resource, secret, storage, IPC, and external action capabilities carried through verification certificates and accepted artifacts.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: \\\"declared capability passes verification\\\"},\\n      {name: \\\"test_multiple_declared_capabilities_are_preserved\\\", given: \\\"Define typed capability requirements for actions and workflows, including resource, secret, storage, IPC, and external action capabilities carried through verification certificates and accepted artifacts.\\\", when: \\\"multiple declared capabilities are preserved\\\", then: [\\\"multiple declared capabilities are preserved\\\"], real_input: \\\"Task scope: Define typed capability requirements for actions and workflows, including resource, secret, storage, IPC, and external action capabilities carried through verification certificates and accepted artifacts.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: \\\"multiple declared capabilities are preserved\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_missing_capability_fails_verification\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"missing capability fails verification\\\", then: [\\\"missing capability fails verification\\\"], real_input: \\\"Task scope: Define typed capability requirements for actions and workflows, including resource, secret, storage, IPC, and external action capabilities carried through verification certificates and accepted artifacts.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: null, expected_error: \\\"missing capability fails verification\\\"},\\n      {name: \\\"test_unknown_capability_kind_fails_verification\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"unknown capability kind fails verification\\\", then: [\\\"unknown capability kind fails verification\\\"], real_input: \\\"Task scope: Define typed capability requirements for actions and workflows, including resource, secret, storage, IPC, and external action capabilities carried through verification certificates and accepted artifacts.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: null, expected_error: \\\"unknown capability kind fails verification\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of verifier/runtime: Define capability contract schema\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Implement to make tests pass\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Read relevant files and understand existing patterns\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write failing tests\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Implement to make tests pass\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_validate/src\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_validate/src\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509015830-ru0upgzt/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      \n    ]\n    similar_implementations: [\n      \n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
        "notes": "WIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:58:32Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-14T03:00:22Z",
        "closed_at": "2026-05-14T03:00:22Z",
        "close_reason": "Closed",
        "labels": [
          "capability",
          "master-gap",
          "planner-shred",
          "release-plan"
        ],
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
      }
    ],
    "dependents": [
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
      },
      {
        "id": "vb-snwg",
        "title": "ui-action-registry: Screen 7 - Action list, ActionContract inspector, capability panel",
        "description": "## Summary\nImplement Action Registry / Contract Inspector screen (Screen 7): action list, ActionContract inspector, capability panel, failure codes.\n\n## Why\nBrowse and inspect all registered actions with their contracts and capabilities.\n\n## What\n- Action list: searchable, filterable list of actions\n- ActionContract inspector: pre/post conditions, invariants display\n- Capability panel: what the action can/cannot do\n- Failure codes: lookup table for action failure modes\n\n## Acceptance\n- [ ] Action list with search and filter\n- [ ] Contract inspector shows full contract details\n- [ ] Capability panel visualizes permissions\n- [ ] Failure codes linked to actions\n\n## Dependencies\n- Blocked by: ui-model-artifacts (ActionDescriptionView)\n\n## Risks\n- Action count: handle 1000+ registered actions",
        "status": "closed",
        "priority": 3,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:06:39Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-09T07:26:37Z",
        "closed_at": "2026-05-09T07:26:37Z",
        "close_reason": "Closed",
        "labels": [
          "master-gap"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-7ode",
        "title": "verifier/runtime: Enforce capabilities at action dispatch",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509015830-3fxfjfsu.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509015830-3fxfjfsu.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509015830-3fxfjfsu\"\n  title: \"verifier/runtime: Enforce capabilities at action dispatch\"\n  type: \"feature\"\n  priority: 1\n  effort_estimate: \"2hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL enforce capabilities before external action dispatch.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN runtime dispatches an action\\\", shall: \\\"THE SYSTEM SHALL compare certified requirements against granted run capabilities.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF a profile lacks a required capability\\\", shall_not: \\\"THE SYSTEM SHALL NOT dispatch the action.\\\", because: \\\"admission evidence must protect runtime side effects.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"Capability contract schema exists.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"Runtime dispatch rejects missing or revoked capabilities with typed diagnostics.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"Runtime cannot add capabilities not present in the accepted artifact and run profile.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/vb_runtime/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/vb_ipc/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"},\\n      {path: \\\"crates/velvet_ballastics/src\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      \"No specific research questions defined\"\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_granted_capability_allows_dispatch\\\", given: \\\"Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.\\\", when: \\\"granted capability allows dispatch\\\", then: [\\\"granted capability allows dispatch\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: \\\"granted capability allows dispatch\\\"},\\n      {name: \\\"test_multiple_granted_capabilities_allow_dispatch\\\", given: \\\"Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.\\\", when: \\\"multiple granted capabilities allow dispatch\\\", then: [\\\"multiple granted capabilities allow dispatch\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: \\\"multiple granted capabilities allow dispatch\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_missing_capability_rejects_dispatch\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"missing capability rejects dispatch\\\", then: [\\\"missing capability rejects dispatch\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: null, expected_error: \\\"missing capability rejects dispatch\\\"},\\n      {name: \\\"test_revoked_capability_rejects_resumed_run\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"revoked capability rejects resumed run\\\", then: [\\\"revoked capability rejects resumed run\\\"], real_input: \\\"Task scope: Enforce verified capability requirements at runtime admission and action dispatch so actions cannot execute unless the run profile grants the certified capabilities.. Use the research_requirements files_to_read as the concrete input surface.\\\", expected_output: null, expected_error: \\\"revoked capability rejects resumed run\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of verifier/runtime: Enforce capabilities at action dispatch\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Implement to make tests pass\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Read relevant files and understand existing patterns\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write failing tests\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Implement to make tests pass\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_runtime/src\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_runtime/src\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509015830-3fxfjfsu/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      \n    ]\n    similar_implementations: [\n      \n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
        "status": "closed",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:58:32Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-10T19:50:01Z",
        "closed_at": "2026-05-10T19:50:01Z",
        "close_reason": "Closed",
        "labels": [
          "capability",
          "master-gap",
          "planner-shred",
          "release-plan"
        ],
        "dependency_type": "parent-child"
      }
    ],
    "parent": "vb-qi37"
  }
]
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-6/.beads

```
