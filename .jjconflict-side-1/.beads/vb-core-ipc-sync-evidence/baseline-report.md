bead_id: vb-core-ipc-sync-evidence
bead_title: vb-core-ipc-sync-evidence
phase: 1
updated_at: 2026-05-15T19:35:57.329117+00:00
attempt: 1-of-7

# Baseline report

Baseline captured before proof/test/implementation edits in isolated workspace.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence
base_revision_command: `jj log -r @- --no-graph -T ...`

## bd show vb-core-ipc-sync-evidence --json
exit=1
```json
{
  "error": "no issues found matching the provided IDs"
}
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/.beads
Error fetching vb-core-ipc-sync-evidence: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues

```

## jj status
exit=0
```text
The working copy has no changes.
Working copy  (@) : ntyyovtt 45fc52bd (empty) (no description set)
Parent commit (@-): moyvrvsn c9c7eee4 main | Delete CHANGELOG.md

```

## jj base revision (@-)
exit=0
```text
moyvrvsnmmzt c9c7eee46ef4 Delete CHANGELOG.md

```

## Baseline classification note

All 20 P0 go-skill workspaces were created from `trunk()` before bead-local edits. The canonical machine gate will be executed and compared in State 11; this report preserves pre-edit bead/workspace identity and base revision evidence for regression classification.

## Corrected bd show vb-core-ipc-sync-evidence --json using source server-mode DB
updated_at=2026-05-15T19:37:45.053546+00:00
command=`bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-core-ipc-sync-evidence --json`
exit=0
```json
[
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
    "dependencies": [
      {
        "id": "vb-0253.1",
        "title": "runtime: Wrap shard command queue boundary",
        "description": "SECTION 0 CLARIFICATIONS\n- Scope is runtime shard command queue architecture only.\n- No channel dependency changes.\n\nSECTION 1 EARS REQUIREMENTS\n- THE SYSTEM SHALL expose shard command admission through a domain-named ShardCommandQueue boundary backed by crossbeam_queue::ArrayQueue.\n- WHEN producers enqueue ShardCommand, THE SYSTEM SHALL preserve existing nonblocking full-error behavior.\n- IF the ArrayQueue backend is full, THE SYSTEM SHALL NOT allocate, block, or silently drop the command.\n\nSECTION 2 CONTRACTS\n- Preconditions: read crates/vb_runtime/src/shard/types.rs and impl_parts/chunk_001.rs.\n- Postconditions: direct ArrayQueue use is isolated behind a runtime-owned queue wrapper/API.\n- Invariants: bounded capacity fixed at construction; QueueFull semantics preserved; tick still consumes at most one command per call unless contract explicitly changes.\n\nSECTION 2.5 RESEARCH\n- Files: crates/vb_runtime/src/shard/types.rs; crates/vb_runtime/src/shard/impl_parts/chunk_001.rs; crates/vb_runtime/src/shard/impl_parts/chunk_004.rs; velvet-ballistics-MASTER.md lines 209-225 and 987-999.\n\nSECTION 3 INVERSION\n- Failure: wrapper becomes generic infrastructure soup. Prevention: name/type it as shard command queue only.\n- Failure: full queue behavior changes. Prevention: contract tests for full/nonfull enqueue.\n\nSECTION 4 ATDD\n- Happy: queue accepts up to configured capacity and tick pops FIFO-observable commands.\n- Happy: status methods report len/capacity/full/remaining consistently.\n- Error: enqueue on full queue returns existing RuntimeError::QueueFull.\n- Error: zero/invalid capacities remain rejected by existing config validation.\n\nSECTION 5 E2E\n- Existing runtime/shard tests pass without public behavior change.\n\nSECTION 5.5 VERIFICATION\n- moon ci or scoped runtime tests per Go-skill proof/test plan.\n- No dependency audit required unless Cargo files change.\n\nSECTION 6 IMPLEMENTATION\n- Research current call sites.\n- Add domain wrapper/API in runtime shard module.\n- Replace direct field type/imports.\n- Add/adjust tests.\n\nSECTION 7 FAILURE MODES\n- Direct ArrayQueue import remains outside wrapper; hidden allocation introduced; command ordering altered.\n\nSECTION 8 COMPLETION\n- No broad vb_sync abstraction created.\n- Direct ArrayQueue use limited to wrapper internals.\n",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-14T03:28:19Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:11Z",
        "labels": [
          "architecture",
          "arrayqueue",
          "go-skill",
          "planner",
          "queue",
          "runtime",
          "standardization"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-0253.2",
        "title": "ipc: Finish ingress modularization and dedupe",
        "description": "SECTION 0 CLARIFICATIONS\n- Scope is vb_ipc module boundary cleanup; preserve public API compatibility.\n\nSECTION 1 EARS REQUIREMENTS\n- THE SYSTEM SHALL have one canonical MemoryIngress, IngressFrame, QueueCapacity, MaxPayloadBytes, and BoundedPayload implementation.\n- WHEN existing callers import public vb_ipc symbols, THE SYSTEM SHALL preserve stable re-exports or provide compile-time migration inside the workspace.\n- IF duplicate type definitions remain in lib.rs and split modules, THE SYSTEM SHALL NOT consider the refactor complete.\n\nSECTION 2 CONTRACTS\n- Preconditions: read crates/vb_ipc/src/lib.rs, ingress.rs, bounded.rs, tests.rs.\n- Postconditions: lib.rs is a facade/re-export layer for ingress/bounded/error/frame modules where feasible.\n- Invariants: bounded memory ingress remains bounded; Full/Disconnected/Empty behavior remains unchanged; payload size validation remains parse-don't-validate.\n\nSECTION 2.5 RESEARCH\n- Files: crates/vb_ipc/src/lib.rs lines around 654-798; crates/vb_ipc/src/ingress.rs; crates/vb_ipc/src/bounded.rs; crates/vb_ipc/src/tests.rs.\n\nSECTION 3 INVERSION\n- Failure: breaking public imports. Prevention: re-export old public names.\n- Failure: two definitions diverge again. Prevention: delete duplicate implementation and add tests through public API.\n\nSECTION 4 ATDD\n- Happy: MemoryIngress::bounded accepts and receives frames through public API.\n- Happy: payload limit allows valid payloads.\n- Error: full queue maps to IpcError::Full.\n- Error: oversized payload maps to IpcError::PayloadTooLarge.\n\nSECTION 5 E2E\n- IPC tests compile and pass; downstream crates using vb_ipc compile.\n\nSECTION 5.5 VERIFICATION\n- Scoped vb_ipc tests plus moon ci before landing.\n\nSECTION 6 IMPLEMENTATION\n- Make split modules authoritative.\n- Turn lib.rs into facade and re-export stable symbols.\n- Update imports/tests.\n\nSECTION 7 FAILURE MODES\n- Duplicate definitions remain; public API break; disconnected/full mapping changes.\n\nSECTION 8 COMPLETION\n- One canonical implementation only.\n- lib.rs line count reduced substantially.\n",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-14T03:28:19Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:11Z",
        "labels": [
          "architecture",
          "drift",
          "go-skill",
          "ipc",
          "modularization",
          "planner",
          "standardization"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-0253.5",
        "title": "state: Align StepState contract across runtime and proofs",
        "description": "SECTION 0 CLARIFICATIONS\n- No external FSM library in this bead.\n- Master Section 45 is authoritative: idempotent state == next is valid for all StepState values.\n\nSECTION 1 EARS REQUIREMENTS\n- THE SYSTEM SHALL define one canonical StepState transition contract matching master Section 45.\n- WHEN runtime, proof-kernel, Kani, or Verus checks evaluate the same transition pair, THE SYSTEM SHALL return the same validity result.\n- IF non-terminal self-transitions are tested, THE SYSTEM SHALL treat them as valid idempotent re-marks.\n\nSECTION 2 CONTRACTS\n- Preconditions: read master Section 45, vb_core frame transition logic, vb_proof_kernels step_state, Verus StepState spec, Kani harnesses.\n- Postconditions: runtime/proof/Verus/Kani transition validity agree on all 64 StepState pairs.\n- Invariants: illegal transitions return invalid_state_transition; terminal states cannot transition to different states; self-loop is legal for every state.\n\nSECTION 2.5 RESEARCH\n- Files: crates/vb_core/src/frame.rs; crates/vb_proof_kernels/src/step_state.rs; verification/verus/step_state_machine.rs; relevant Kani harnesses.\n\nSECTION 3 INVERSION\n- Failure: proof claims validate behavior that runtime does not implement. Prevention: exhaustive parity table/harness.\n- Failure: runtime changes but proof kernel remains stale. Prevention: single canonical table or generated projection plus tests.\n\nSECTION 4 ATDD\n- Happy: all valid Section 45 transitions accepted.\n- Happy: all eight self-transitions accepted.\n- Error: invalid transitions rejected with invalid_state_transition.\n- Error: proof-kernel tests fail if any runtime/proof pair diverges.\n\nSECTION 5 E2E\n- Proof/kernel tests and selected verifier gates pass or produce scoped tooling blockers.\n\nSECTION 5.5 VERIFICATION\n- Exhaustive 8x8 tests required.\n- Kani/Verus obligation planning required by Go-skill proof loop.\n\nSECTION 6 IMPLEMENTATION\n- Establish canonical transition table/source.\n- Update runtime/proof/Verus/Kani projections.\n- Add parity tests.\n\nSECTION 7 FAILURE MODES\n- Hidden duplicate transition logic remains; Verus still rejects Running-\u003eRunning; tests only cover happy paths.\n\nSECTION 8 COMPLETION\n- All 64 transition pairs covered by machine evidence.\n",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "bug",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-14T03:28:22Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:11Z",
        "labels": [
          "architecture",
          "contract-parity",
          "go-skill",
          "planner",
          "proof",
          "standardization",
          "state"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-0253.6",
        "title": "runtime: Route shard runtime states through apply API",
        "description": "SECTION 0 CLARIFICATIONS\n- Scope is Shard RuntimeState mutation discipline, not external FSM library adoption.\n\nSECTION 1 EARS REQUIREMENTS\n- THE SYSTEM SHALL route RuntimeState changes through a shard-owned transition API.\n- WHEN submit/resume/drive/fail paths change runtime state, THE SYSTEM SHALL apply a typed RuntimeEvent rather than direct runtime_states.insert or swap_remove calls.\n- IF an illegal RuntimeState transition is requested, THE SYSTEM SHALL return a typed error or invariant violation instead of mutating state.\n\nSECTION 2 CONTRACTS\n- Preconditions: read shard types, lifecycle chunk_001, lifecycle chunk_002, transitions.rs.\n- Postconditions: direct RuntimeState mutations are isolated behind RuntimeStateMachine or equivalent apply API.\n- Invariants: Resumable/Resuming/Running/Failed semantics remain unchanged; journal failure rollback to Resumable remains explicit; terminal cleanup remains explicit.\n\nSECTION 2.5 RESEARCH\n- Files: crates/vb_runtime/src/shard/types.rs; crates/vb_runtime/src/shard/lifecycle/chunk_001.rs; crates/vb_runtime/src/shard/lifecycle/chunk_002.rs; crates/vb_runtime/src/shard/transitions.rs.\n\nSECTION 3 INVERSION\n- Failure: direct insert bypasses lifecycle rules. Prevention: grep/test for direct mutation outside the state module.\n- Failure: RuntimeState confused with LifecycleState or StepState. Prevention: separate event vocabulary and module.\n\nSECTION 4 ATDD\n- Happy: submit initializes state through apply API.\n- Happy: awaiting action/timer moves to Resumable through apply API.\n- Error: resume from Running remains rejected.\n- Error: illegal transition does not mutate runtime_states.\n\nSECTION 5 E2E\n- Existing runtime lifecycle tests pass; new direct-mutation guard test or lint/check passes.\n\nSECTION 5.5 VERIFICATION\n- Contract tests plus Go-skill proof plan for state invariants.\n\nSECTION 6 IMPLEMENTATION\n- Define RuntimeEvent and apply function.\n- Replace direct insert/swap_remove call sites.\n- Add parity/illegal-transition tests.\n\nSECTION 7 FAILURE MODES\n- State cleanup lost; rollback wrong; direct mutation remains in lifecycle chunks.\n\nSECTION 8 COMPLETION\n- runtime_states mutation sites centralized and searchable.\n",
        "notes": "Landed commit 7bf72fe1 to origin/main. RuntimeEvent + apply() API implemented. D001/D001.2 defects repaired. 85 pre-existing failures classified as DEFERRED_GLOBAL.",
        "status": "closed",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-14T03:28:23Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T12:51:29Z",
        "closed_at": "2026-05-15T12:51:29Z",
        "labels": [
          "architecture",
          "ddd",
          "go-skill",
          "planner",
          "runtime",
          "standardization",
          "state"
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
    "dependents": [
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
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/.beads

```
