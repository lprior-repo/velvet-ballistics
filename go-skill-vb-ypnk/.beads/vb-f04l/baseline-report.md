bead_id: vb-f04l
bead_title: vb-f04l
phase: 1
updated_at: 2026-05-15T19:36:04.923662+00:00
attempt: 1-of-7

# Baseline report

Baseline captured before proof/test/implementation edits in isolated workspace.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l
base_revision_command: `jj log -r @- --no-graph -T ...`

## bd show vb-f04l --json
exit=1
```json
{
  "error": "no issues found matching the provided IDs"
}
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l/.beads
Error fetching vb-f04l: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues

```

## jj status
exit=0
```text
The working copy has no changes.
Working copy  (@) : szqvvmzs 06b41c17 (empty) (no description set)
Parent commit (@-): moyvrvsn c9c7eee4 main | Delete CHANGELOG.md

```

## jj base revision (@-)
exit=0
```text
moyvrvsnmmzt c9c7eee46ef4 Delete CHANGELOG.md

```

## Baseline classification note

All 20 P0 go-skill workspaces were created from `trunk()` before bead-local edits. The canonical machine gate will be executed and compared in State 11; this report preserves pre-edit bead/workspace identity and base revision evidence for regression classification.

## Corrected bd show vb-f04l --json using source server-mode DB
updated_at=2026-05-15T19:37:45.053546+00:00
command=`bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-f04l --json`
exit=0
```json
[
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
    "dependencies": [
      {
        "id": "vb-core-lower-control-primitives",
        "title": "compiler: Lower v1 control primitives from YAML AST",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Implement and test YAML AST to numeric IR lowering for control primitives: for_each, together, collect, reduce, repeat, wait, and ask as applicable. Excludes generated Rust.",
        "acceptance_criteria": "Each control primitive has positive lowering tests, invalid-shape diagnostics, dense numeric IR indexes, and runtime-compatible compiled output; no synthetic id-plus-one body assumptions remain.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:54Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:09Z",
        "labels": [
          "compiler",
          "core-priority",
          "engine",
          "ir",
          "no-codegen",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-lower-coverage-matrix",
        "title": "compiler: Prove v1 lowering coverage matrix",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Add a coverage matrix proving every v1 YAML construct is accepted/rejected consistently across vb_yaml, vb_validate, and vb_compile, excluding codegen/generated mode.",
        "acceptance_criteria": "Every v1 construct has parser/validator/compiler parity tests; unsupported codegen/UI paths are explicitly excluded; no parser/compiler grammar drift remains.",
        "status": "in_progress",
        "priority": 0,
        "issue_type": "task",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:54Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T05:00:09Z",
        "labels": [
          "compiler",
          "core-priority",
          "engine",
          "ir",
          "no-codegen",
          "tests",
          "yaml"
        ],
        "dependency_type": "blocks"
      },
      {
        "id": "vb-core-lower-values-actions-refs",
        "title": "compiler: Lower v1 values actions and references",
        "description": "Planner session core-engine-p0-audit PASS 97/100. Implement and test YAML AST to numeric IR lowering for values, expressions, action references, capability references, slot references, accessors, and taint metadata.",
        "acceptance_criteria": "Author YAML no longer requires low-level slots/actions; invalid references fail before runtime; lowered IR preserves value/action/ref/taint semantics and runtime core receives numeric/handle data only.",
        "notes": "femdation BLOCK_LOCAL 2026-05-15: refused to resume because forbidden source-checkout artifact exists at /home/lewis/src/velvet-ballistics/.beads/vb-core-lower-values-actions-refs/STATE.md. Source checkout is control-plane only; needs cleanup/recovery before isolated go-skill resume.",
        "status": "blocked",
        "priority": 0,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-15T04:26:54Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T17:13:33Z",
        "labels": [
          "compiler",
          "core-priority",
          "engine",
          "ir",
          "no-codegen",
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
        "id": "vb-qi37.9",
        "title": "expr: Complete F64 semantics and helper parity evidence",
        "description": "Fix expression gaps called out by the master doc: F64 literal/evaluator/typechecker mismatch; helper bugs in empty/unique/merge/sum; missing helpers contains, starts_with, ends_with, has, append, append_if, and merge semantics.",
        "acceptance_criteria": "Expression parser/typechecker/evaluator/codegen agree for F64 where supported; unsupported F64 paths reject cleanly; helper functions match spec arity/types/results; property tests cover helper equivalence and typed errors; no helper returns placeholder/no-op behavior.",
        "notes": "Source audit: helper ops mostly exist in interpreter and bytecode lowering; remaining gap is F64 source literal/eval/codegen parity plus helper type/eval/generated evidence, not implementing helpers from zero.\nWIP cleanup 2026-05-11: not closed because this bead's own acceptance scope is not fully proven or its notes identify remaining implementation/evidence. main is green at commit 6090845a6 (moon ci --force: 19/19 completed, 7994/7994 tests passed). Reset from in_progress to open so WIP reflects only active work; reclaim before resuming.\nDemoted 2026-05-14: parent includes generated/helper parity; core interpreter F64 blocker remains P0 as vb-qi37.9.2.",
        "status": "open",
        "priority": 2,
        "issue_type": "feature",
        "assignee": "Lewis",
        "owner": "priorlewis43@gmail.com",
        "created_at": "2026-05-09T06:35:50Z",
        "created_by": "Lewis",
        "updated_at": "2026-05-15T04:00:28Z",
        "labels": [
          "expr",
          "master-gap",
          "mvp-feature-now",
          "release-plan",
          "semantics"
        ],
        "dependency_type": "blocks"
      }
    ],
    "dependents": [
      {
        "id": "vb-ahfl",
        "title": "engine: End-to-end YAML to IR semantic evidence",
        "description": "# CUE Validation Schema\n# Validate implementation: cue vet /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-mvegej3o.cue implementation.cue\n# Schema location: /home/lewis/src/Velvet-ballistics/.beads/schemas/velvet-ballistics-20260509012640-mvegej3o.cue\n\n\n#EnhancedBead: {\n  id: \"velvet-ballistics-20260509012640-mvegej3o\"\n  title: \"engine: End-to-end YAML to IR semantic evidence\"\n  type: \"feature\"\n  priority: 0\n  effort_estimate: \"4hr\"\n  labels: [\"planner-generated\"]\n\n  clarifications: {\n    clarification_status: \"RESOLVED\"\n  }\n\n  ears_requirements: {\n    ubiquitous: [\n      \\\"THE SYSTEM SHALL guarantee semantic fidelity from YAML to execution.\\\"\n    ]\n    event_driven: [\n      {trigger: \\\"WHEN a YAML workflow is executed\\\", shall: \\\"THE SYSTEM SHALL produce the exact step events mandated by the YAML definition.\\\"}\n    ]\n    unwanted: [\n      {condition: \\\"IF a primitive behaves differently than its YAML spec\\\", shall_not: \\\"THE SYSTEM SHALL NOT silently accept it\\\", because: \\\"IR lowering must be lossless.\\\"}\n    ]\n  }\n\n  contracts: {\n    preconditions: {\n      auth_required: false\n      required_inputs: []\n      system_state: [\n        \\\"YAML parser is strict.\\\"\n      ]\n    }\n    postconditions: {\n      state_changes: [\n        \\\"E2E test suite asserts YAML vs Journal output.\\\"\n      ]\n      return_guarantees: []\n    }\n    invariants: [\n      \\\"YAML structure dictates exact journal signature.\\\"\n    ]\n  }\n\n  research_requirements: {\n    files_to_read: [\n      {path: \\\"crates/velvet_ballastics/tests/\\\", what_to_extract: \\\"All patterns and implementations\\\", document_in: \\\"research_notes.md\\\"}\n    ]\n    research_questions: [\n      {question: \\\"What is the best way to assert journal signatures from YAML?\\\", answered: false}\n    ]\n    research_complete_when: [\n      \"All research_requirements files have been read\"\n    ]\n  }\n\n  inversions: {\n    usability_failures:     [\n      {failure: \"Implementation diverges from the task contract\", prevention: \"Write the named failing tests first, then implement only enough code to satisfy them\", test_for_it: \"test_contract_alignment\"}\n    ]\n  }\n\n  acceptance_tests: {\n    happy_paths:     [\n      {name: \\\"test_yaml_with_do/set/finish_maps_perfectly_to_journal.\\\", given: \\\"Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.\\\", when: \\\"YAML with Do/Set/Finish maps perfectly to journal.\\\", then: [\\\"YAML with Do/Set/Finish maps perfectly to journal.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: \\\"YAML with Do/Set/Finish maps perfectly to journal.\\\"},\\n      {name: \\\"test_yaml_with_collect_produces_correct_slot_extra_data.\\\", given: \\\"Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.\\\", when: \\\"YAML with Collect produces correct slot extra data.\\\", then: [\\\"YAML with Collect produces correct slot extra data.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: \\\"YAML with Collect produces correct slot extra data.\\\"}\n    ]\n    error_paths:     [\n      {name: \\\"test_invalid_yaml_execution_topology_is_caught_before_execution.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Invalid YAML execution topology is caught before execution.\\\", then: [\\\"Invalid YAML execution topology is caught before execution.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: null, expected_error: \\\"Invalid YAML execution topology is caught before execution.\\\"},\\n      {name: \\\"test_type_mismatches_in_yaml_are_rejected_in_lowering_or_static_check.\\\", given: \\\"Error precondition derived from contracts and invariants\\\", when: \\\"Type mismatches in YAML are rejected in lowering or static check.\\\", then: [\\\"Type mismatches in YAML are rejected in lowering or static check.\\\"], real_input: \\\"Task scope: Fulfill MASTER.md requirement for end-to-end primitive semantics. Build executable parity evidence proving that a source YAML workflow exactly matches the intended final numeric IR behavior during execution.. Relevant files: crates/vb_storage/src/journal.rs.\\\", expected_output: null, expected_error: \\\"Type mismatches in YAML are rejected in lowering or static check.\\\"}\n    ]\n  }\n\n  e2e_tests: {\n      pipeline_test: {{\n      name: \"test_full_pipeline\"\n      description: \"End-to-end test of engine: End-to-end YAML to IR semantic evidence\"\n      setup: {{}}\n      execute: {{\n        command: \"Validate final behavior after: Wire into CI.\"\n      }}\n      verify: {{\n        exit_code: 0\n      }}\n    }}\n  }\n\n  verification_checkpoints: {\n    gate_0_research: {\n      name: \"Research Gate\"\n      must_pass_before: \"Writing code\"\n      checks: [\"All research_requirements files have been read\"]\n      evidence_required: [\"Research notes documented\"]\n    }\n    gate_1_tests: {\n      name: \"Test Gate\"\n      must_pass_before: \"Implementation\"\n      checks: [\"All tests written and failing\"]\n      evidence_required: [\"Test files exist\"]\n    }\n    gate_2_implementation: {\n      name: \"Implementation Gate\"\n      must_pass_before: \"Completion\"\n      checks: [\"All tests pass\"]\n      evidence_required: [\"CI green\"]\n    }\n    gate_3_integration: {\n      name: \"Integration Gate\"\n      must_pass_before: \"Closing bead\"\n      checks: [\"E2E tests pass\"]\n      evidence_required: [\"Manual verification complete\"]\n    }\n  }\n\n  implementation_tasks: {\n    phase_0_research: {\n      parallelizable: true\n      tasks: [\n        {task: \\\"Design YAML-to-Journal assertion harness.\\\", done_when: \\\"Documented\\\", parallel_group: \\\"research\\\"}\n      ]\n    }\n    phase_1_tests_first: {\n      parallelizable: true\n      gate_required: \"gate_0_research\"\n      tasks: [\n        {task: \\\"Write E2E test cases for all major primitives.\\\", done_when: \\\"Test exists and fails\\\", parallel_group: \\\"tests\\\"}\n      ]\n    }\n    phase_2_implementation: {\n      parallelizable: false\n      gate_required: \"gate_1_tests\"\n      tasks: [\n        {task: \\\"Wire into CI.\\\", done_when: \\\"Tests pass\\\"}\n      ]\n    }\n    phase_4_verification: {\n      parallelizable: true\n      gate_required: \"gate_2_implementation\"\n      tasks: [\n        {task: \"Run moon run :ci\", done_when: \"CI passes\", parallel_group: \"verification\"}\n      ]\n    }\n  }\n\n  failure_modes: {\n      failure_modes: [\n      {symptom: \"Tests fail\", likely_cause: \"Implementation does not match specification\", where_to_look: [{{file: \"crates/vb_storage/src/journal.rs\", what_to_check: \"Compare implementation and tests with contracts postconditions and invariants\"}}], fix_pattern: \"Re-read task specification, fix the failing test, then repair the implementation to satisfy the contract\"}\n    ]\n  }\n\n  anti_hallucination: {\n    read_before_write: [\n      {file: \\\"crates/vb_storage/src/journal.rs\\\", must_read_first: true, key_sections_to_understand: [\\\"All existing implementations\\\"]}\n    ]\n    apis_that_exist: []\n    no_placeholder_values: [\"Use real data from codebase\"]\n    git_verification: {\n      before_claiming_done: \"git status \u0026\u0026 git diff \u0026\u0026 moon run :test\"\n    }\n  }\n\n  context_survival: {\n    progress_file: {\n      path: \".bead-progress/velvet-ballistics-20260509012640-mvegej3o/progress.txt\"\n      format: \"Markdown checklist\"\n    }\n    recovery_instructions: \"Read progress.txt and continue from current task\"\n  }\n\n  completion_checklist: {\n    tests: [\n      \"Acceptance tests from this bead are implemented and passing\",\n      \"Error-path tests from this bead are implemented and passing\",\n      \"Pipeline verification exercises real project inputs for this task\",\n      \"No fake placeholders remain in test inputs or assertions\"\n    ]\n    code: [\n      \"Implementation uses Result\u003cT, Error\u003e throughout where fallible\",\n      \"No unwrap or expect calls remain in production paths\"\n    ]\n    ci: [\n      \"moon run :ci passes after the task changes\"\n    ]\n  }\n\n  context: {\n    related_files: [\n      {path: \\\"crates/vb_storage/src/journal.rs\\\", relevance: \\\"Related implementation\\\"}\n    ]\n    similar_implementations: [\n      \\\"CLI simulate tests.\\\"\n    ]\n  }\n\n  ai_hints: {\n    do: [\n      \"Use functional patterns: map, and_then, ?\",\n      \"Return Result\u003cT, Error\u003e from all fallible functions\",\n      \"READ files before modifying them\"\n    ]\n    do_not: [\n      \"Do NOT use unwrap or expect\",\n      \"Do NOT use panic!, todo!, or unimplemented!\",\n      \"Do NOT modify clippy configuration\"\n    ]\n    constitution: [\n      \"Zero unwrap law: NEVER use .unwrap or .expect\",\n      \"Test first: Tests MUST exist before implementation\"\n    ]\n  }\n}\n",
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
      }
    ],
    "parent": "vb-qi37"
  }
]
Warning: /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l/.beads

```
