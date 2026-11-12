# Test Plan: vb-xi2f.29 — Digest Covers Together Semantics

**Bead**: vb-xi2f.29
**Bridge**: p2i-vb-xi2f29-2026-05-25-001 (STATUS: APPROVED)
**Implementation status**: FIX APPLIED (part_05.rs:105, 158-167, 174-177)
**Test plan artifact**: tp-vb-xi2f29-2026-05-25-001
**Test-planner invocation**: p8-test-planner (State 8)

## Summary

- **Behaviors identified**: 18
- **Trophy allocation**: 9 unit / 6 integration (proptest) / 0 e2e / 3 static (kani)
- **Proptest invariants**: 6 existing, 2 planned
- **Fuzz targets**: 2 planned (none existing)
- **Kani harnesses**: 6 existing (3 passing, 3 blocked), 0 new planned
- **Mutation checkpoints**: 12 identified, target ≥90% kill rate
- **Existing tests passing**: 15 proptest + 5 unit + 1 kani = 21 verified
- **Test gaps identified**: 8 (see Section 9)

## 1. Behavior Inventory

Each behavior is a guarantee the system makes through its public API.

### B-001: Canonical name for Together
`canonical_primitive_name` returns `"together"` when given a `StepPrimitive::Together`.

### B-002: Canonical name for all primitives
`canonical_primitive_name` returns the correct static string for every `StepPrimitive` variant, never panics.

### B-003: Branch count affects digest
`canonical_digest` produces different `WorkflowDigest` values for workflows differing only in Together branch count.

### B-004: Branch labels affect digest
`canonical_digest` produces different digests for workflows differing only in Together branch labels.

### B-005: Sub-step set values affect digest
`canonical_digest` produces different digests when sub-step `Set` values within a Together branch differ.

### B-006: Sub-step output names affect digest
`canonical_digest` produces different digests when sub-step `Set` output names within a Together branch differ.

### B-007: Branch ordering affects digest
`canonical_digest` produces different digests for workflows with Together branches in different order.

### B-008: Digest determinism (same source)
`canonical_digest` produces identical digests for the identical `WorkflowSource`.

### B-009: Digest idempotency
`canonical_digest` produces identical digests across multiple independent calls with the same source.

### B-010: Non-Together regression
Workflows without Together steps produce the same digests as before the fix (modulo canonical name fix for non-Together primitives).

### B-011: Recursive sub-step hashing
`digest_sub_step` traverses and hashes `StepAst.id` and `StepAst.primitive` recursively for each sub-step within a Together branch.

### B-012: Branch count encoded as u16 LE
`digest_step_primitive` for Together encodes `branches.len()` as `u16` little-endian bytes.

### B-013: Single-branch together is hashable
A Together step with exactly one branch produces a deterministic, non-zero digest.

### B-014: Many-branch together is hashable
A Together step with many branches (up to valid limit) produces a deterministic digest without panic.

### B-015: Empty sub-steps within branch
A Together branch with zero sub-steps (`steps: []`) produces a deterministic, valid digest (per POST-DIGEST-004).

### B-016: No panic on any valid WorkflowSource
`canonical_digest` never panics for any `WorkflowSource` that passes validation (per INV-004, POST-006).

### B-017: `digest_step_primitive` other-arm coverage
All non-Together, non-Set, non-Finish primitives fall through to the `other` arm and hash their canonical name without panic.

### B-018: `digest_sub_step` with non-Together primitives
`digest_sub_step` correctly delegates to `digest_step_primitive` for any `StepPrimitive` variant (not just Together), enabling future use with other scoped primitives (ForEach, Collect, etc.).

## 2. Trophy Allocation

```
         [E2E]           ← 0 — no user-facing workflow engine execution needed
    [Integration]        ← 8 — proptest (6 existing + 2 planned) using real compile pipeline
    [Unit / Calc]        ← 9 — direct function tests (5 existing + 4 planned)
  [Static Analysis]      ← 3 — Kani model checking (1 passing, 2 blocked but present)
```

| Layer | Tests | Existing | Planned | Rationale |
|-------|-------|----------|---------|-----------|
| E2E | 0 | 0 | 0 | No user-facing CLI/API boundary affected. Digest is a compile-time artifact. |
| Integration (proptest) | 8 | 6 | 2 | Real compile pipeline (`compile_workflow` + `vb_yaml::parse`) exercises YAML→IR→digest end-to-end. No mocks. Prefer real deps per Google SWE Ch.13. |
| Unit | 9 | 5 | 4 | Pure functions (`canonical_primitive_name`, `digest_step_primitive`, `digest_sub_step`, `canonical_digest`) tested in isolation. |
| Static (Kani) | 6 | 6 (3 pass) | 0 | Formal verification for name fix, determinism, recursion bounds. Some blocked by tooling. |
| Fuzz | 2 | 0 | 2 | YAML parsing boundary (`compile_workflow`) and AST construction (`parse_workflow_source`) accept raw bytes. |
| Mutation | — | 0 | 12 checkpoints | Mutation kill-rate target: ≥90%. |

**Deviation justification**: E2E=0 is appropriate because digest computation has no user-visible behavior beyond the compile step. The proptest layer provides the integration coverage using the real compile pipeline. Fuzz targets fill the raw-bytes boundary gap.

## 3. BDD Scenarios

### Behavior B-001: Canonical name for Together

**Given**: a `StepPrimitive::Together { branches: [...] }` value
**When**: `canonical_primitive_name(&primitive)` is called
**Then**: the function returns `"together"`
**And**: the function does NOT return `"parallel"`

- [x] `fn canonical_name_together_harness()` — Kani VERIFIED (0/432 failed), file: `kani_canonical_name.rs:41`
- [ ] `fn canonical_primitive_name_together_returns_together_direct()` — **GAP**: direct unit test calling `canonical_primitive_name` on a constructed `Together` and asserting `== "together"` (existing PO-015 test in error_variant_tests.rs verifies indirectly via compilation but does not call the function directly; the Kani harness covers this but a Rust unit test is warranted for fast red-green feedback)

### Behavior B-002: Canonical name for all primitives

**Given**: any valid `StepPrimitive` variant
**When**: `canonical_primitive_name(&primitive)` is called
**Then**: the function returns the correct static string for that variant
**And**: the function never panics

- [x] `fn canonical_name_together_harness()` — Together specifically (kani_canonical_name.rs:41, PASS)
- [~] `fn canonical_name_all_harness()` — all 12 variants (kani_canonical_name.rs:137, BLOCKED: TIMED_OUT >10min)
- [ ] `fn canonical_primitive_name_returns_correct_names_all_variants()` — **GAP**: exhaustive unit test covering all 12 named variants + unknown fallback. Each variant: construct, call, assert exact string.

### Behavior B-003: Branch count affects digest

**Given**: two `WorkflowSource` values differing only in `Together.branches.len()` (2 vs 3)
**When**: both are compiled via `compile_source` → `workflow.digest()`
**Then**: the two digests are NOT equal

- [x] `fn proptest_together_branch_count_produces_different_digest()` — proptest 1000 cases (together_digest_sensitivity.rs:108, PASS)
- [x] `fn test_different_together_configurations_produce_different_digests()` sub-case (a) — unit test 2 vs 3 branches (error_variant_tests.rs:1139, PASS)
- [~] `fn together_branch_count_produces_different_digest_kani()` — Kani symbolic (together_digest_kani.rs:233, BLOCKED_TOOLING)

### Behavior B-004: Branch labels affect digest

**Given**: two `WorkflowSource` values with identical structure but one different branch label
**When**: both are compiled and digests compared
**Then**: the two digests are NOT equal

- [x] `fn proptest_together_branch_labels_produce_different_digest()` — proptest 1000 cases (together_digest_sensitivity.rs:146, PASS)
- [x] `fn test_different_together_configurations_produce_different_digests()` sub-case (b) (error_variant_tests.rs:1209, PASS)

### Behavior B-005: Sub-step set values affect digest

**Given**: two workflows with identical Together structure but different `Set.value` within a branch
**When**: both compiled
**Then**: digests differ

- [x] `fn proptest_together_sub_step_contents_produce_different_digest()` (together_digest_sensitivity.rs:183, PASS)

### Behavior B-006: Sub-step output names affect digest

**Given**: two workflows with identical Together structure but different `Set.output` within a branch
**When**: both compiled
**Then**: digests differ

- [x] `fn proptest_together_sub_step_output_produces_different_digest()` (together_digest_sensitivity.rs:212, PASS)

### Behavior B-007: Branch ordering affects digest

**Given**: two workflows with the same Together branches but in different order
**When**: both compiled
**Then**: digests differ

- [x] `fn proptest_together_branch_ordering_produces_different_digest()` (together_digest_sensitivity.rs:245, PASS)
- [x] `fn test_nested_together_produces_distinct_recursive_digest()` — reordering sub-case (error_variant_tests.rs:1035, PASS)

### Behavior B-008: Digest determinism (same source)

**Given**: a `WorkflowSource` with Together steps
**When**: `canonical_digest(source)` is called twice
**Then**: both calls return the identical `WorkflowDigest`

- [x] `fn proptest_together_digest_is_deterministic()` (together_digest_sensitivity.rs:279, PASS)
- [x] `fn proptest_equal_primitive_sources_compile_to_equal_digest_and_ir()` — existing regression (v1_primitive_lowering.rs:828, PASS)
- [~] `fn together_digest_step_deterministic_kani()` — Kani (together_digest_kani.rs:144, BLOCKED_TOOLING)

### Behavior B-009: Digest idempotency

**Given**: a `WorkflowSource` parsed 3 independent times
**When**: each is compiled and digests compared
**Then**: all 3 digests are equal

- [x] `fn test_canonical_digest_is_idempotent_with_together()` (error_variant_tests.rs:1084, PASS)

### Behavior B-010: Non-Together regression

**Given**: a `WorkflowSource` containing no Together steps (only Set, Save, Do, etc.)
**When**: compiled through all 3 public API paths (compile_source, compile_workflow, YamlCompiler::compile)
**Then**: produced digests match expected baseline, compiled IR matches expected shape

- [x] `fn proptest_equal_primitive_sources_compile_to_equal_digest_and_ir()` — 15/15 primitive cases (v1_primitive_lowering.rs:828, PASS)
- [x] `fn proptest_scoped_primitives_never_return_unsupported_step_primitive()` — shape check (v1_primitive_lowering.rs:837, PASS)

### Behavior B-011: Recursive sub-step hashing

**Given**: a `StepAst` with `id` and `primitive` fields
**When**: `digest_sub_step(hasher, step)` is called
**Then**: `hasher.update(step.id.as_bytes())` and `digest_step_primitive(hasher, &step.primitive)` are invoked

- [x] Indirectly verified by all proptest sensitivity tests (they exercise the compile→digest path which calls digest_sub_step)
- [~] `fn together_digest_sub_step_recursion_bounded_kani()` — Kani (together_digest_kani.rs:54, BLOCKED_TOOLING)
- [ ] `fn digest_sub_step_hashes_id_and_primitive()` — **GAP**: direct unit test constructing a known `StepAst`, calling `digest_sub_step` with a fresh hasher, and asserting the resulting digest is non-zero and deterministic

### Behavior B-012: Branch count encoded as u16 LE

**Given**: a `StepPrimitive::Together` with N branches
**When**: `digest_step_primitive(hasher, &primitive)` is called
**Then**: `hasher.update(&(N as u16).to_le_bytes())` is invoked (implied by branch-count sensitivity working)

- [x] Verified by implication in proptest branch-count sensitivity (PO-002)
- [ ] `fn digest_step_primitive_together_includes_u16_branch_count()` — **GAP**: direct test constructing two Together primitives with counts that differ only in the u16 representation (e.g., 256 vs 0 after wrapping) — but since `branches.len()` already differs, this is largely covered by B-003. Low priority.

### Behavior B-013: Single-branch together is hashable

**Given**: a `WorkflowSource` with a Together step containing exactly 1 branch
**When**: compiled
**Then**: produces a deterministic, non-zero, non-panicking digest

- [ ] **GAP**: Not explicitly tested. Proptest strategies only generate 2+ branches. Add a unit test with 1 branch.

### Behavior B-014: Many-branch together is hashable

**Given**: a `WorkflowSource` with a Together step containing 10+ branches with distinct labels
**When**: compiled
**Then**: produces a deterministic digest without panic

- [ ] **GAP**: Not tested. Proptest max is 3 branches. Add a unit test with 10 branches or a proptest with branch count up to, say, 20.

### Behavior B-015: Empty sub-steps within branch

**Given**: a Together branch with `steps: []` (zero sub-steps)
**When**: `digest_step_primitive` processes this branch
**Then**: the branch label is hashed, but the inner `for step in &branch.steps` loop executes zero times, producing a deterministic digest

- [ ] **GAP**: Not tested. Current error_variant_tests.rs comment at line 859 says "empty branch steps (steps: []) are currently rejected by validation (StepFieldShape error)." This means validation currently rejects empty steps. If validation is ever relaxed, the digest computation must handle it. Add a unit test that constructs a `TogetherBranch { label, steps: vec![] }` programmatically, calls `digest_step_primitive`, and asserts determinism.

### Behavior B-016: No panic on any valid WorkflowSource

**Given**: any `WorkflowSource` that passes validation
**When**: `canonical_digest(source)` is called
**Then**: the function returns a `WorkflowDigest` without panicking

- [x] Covered by all proptests (none panic)
- [~] `fn together_digest_sub_step_recursion_bounded_kani()` — Kani bounded proof (BLOCKED_TOOLING)
- [x] Validation rejects structurally invalid workflows before digest is computed

### Behavior B-017: digest_step_primitive other-arm coverage

**Given**: any non-Together, non-Set, non-Finish `StepPrimitive` (e.g., Do, Choose, ForEach, Collect, Aggregate, Repeat, Wait, Ask)
**When**: `digest_step_primitive(hasher, &primitive)` is called
**Then**: the `other` arm hashes `canonical_primitive_name(primitive).as_bytes()` without panic

- [x] Covered by v1_primitive_lowering.rs proptest (all primitives go through compile → digest)
- [ ] `fn digest_step_primitive_other_arm_no_panic()` — **GAP**: explicit unit test enumerating all non-special-cased variants and verifying no panic + deterministic output

### Behavior B-018: digest_sub_step with non-Together primitives

**Given**: a `StepAst` whose `primitive` is not `Together` (e.g., `ForEach`, `Collect`)
**When**: `digest_sub_step(hasher, step)` is called
**Then**: `step.id` and `step.primitive` are hashed correctly via the `digest_step_primitive` dispatch — even for primitives not (yet) handled by `canonical_digest`'s top-level traversal

- [x] The `digest_sub_step` function is structurally generic (hashes `id` + delegates to `digest_step_primitive`). All proptests that go through compile→digest exercise this path.
- [ ] `fn digest_sub_step_with_for_each_primitive()` — **GAP**: explicit unit test verifying that `digest_sub_step` correctly hashes a `StepAst` with a `ForEach` primitive (since ForEach is the next most likely scoped primitive to get the same fix). Ensures the `digest_sub_step` function is not accidentally Together-specific.

## 4. Proptest Invariants

### Existing (6, all PASS)

| # | Invariant | Function | Strategy | File | Status |
|---|-----------|----------|----------|------|--------|
| P-1 | Branch count → different digest | `canonical_digest` | random labels/outputs/values, 2 vs 3 branches | together_digest_sensitivity.rs:108 | PASS |
| P-2 | Branch labels → different digest | `canonical_digest` | random distinct labels, same other structure | together_digest_sensitivity.rs:146 | PASS |
| P-3 | Sub-step values → different digest | `canonical_digest` | random distinct Set values | together_digest_sensitivity.rs:183 | PASS |
| P-4 | Sub-step output names → different digest | `canonical_digest` | random distinct Set output names | together_digest_sensitivity.rs:212 | PASS |
| P-5 | Branch ordering → different digest | `canonical_digest` | same branches, swapped order | together_digest_sensitivity.rs:245 | PASS |
| P-6 | Same source → same digest (determinism) | `canonical_digest` | random labels/outputs/values, compile twice | together_digest_sensitivity.rs:279 | PASS |
| P-7 | All primitives → same digest on same source (regression) | `canonical_digest` + `compile_source` | select(PrimitiveCase), compile twice | v1_primitive_lowering.rs:828 | PASS |

### Planned (2)

| # | Invariant | Function | Strategy | Rationale |
|---|-----------|----------|----------|-----------|
| P-8 | Variable branch count (1..=20) → valid digest, no panic | `canonical_digest` via `compile_workflow` | `1..=20usize` branch count, random labels/outputs/values | Stress-test branch count range. Proptest cases=256. |
| P-9 | Branch label length variation (1..=256 chars) → valid digest | `canonical_digest` via `compile_workflow` | random alphanumeric labels 1-256 chars | Stress-test label hashing with long strings. Proptest cases=256. |

### Out-of-scope invariants (for future beads)

- **ForEach body sensitivity**: `digest_step_primitive` does not hash `ForEach.body` sub-steps. This is the same nested-step-blindness defect, deferred to a future bead.
- **Collect body sensitivity**: Same pattern, deferred.
- **Aggregate body sensitivity + canonical name**: Same pattern + `"aggregate"` → `"reduce"` fix, deferred.
- **Repeat body sensitivity**: Same pattern, deferred.

## 5. Fuzz Targets

### FT-001: compile_workflow (YAML byte input)

| Field | Value |
|-------|-------|
| **Target function** | `vb_compile::compile_workflow(source: &[u8])` |
| **Input type** | `&[u8]` — raw bytes |
| **Risk** | YAML parser panic on malformed input, excessive memory allocation, stack overflow from deeply nested structures, integer overflow in YAML integer parsing |
| **Fuzzer** | `cargo-fuzz` (libFuzzer) via `cargo fuzz` |
| **Corpus seeds** | Valid minimal workflows: `"version: velvet-ballastics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n"`. Together workflow: the 2-branch together from error_variant_tests.rs. Empty input: `""`. Truncated YAML: `"version:"`. |
| **Location** | `fuzz/fuzz_targets/compile_workflow_fuzz.rs` |
| **Coverage goal** | All error paths in `YamlCompiler::parse_ast`, `compile_source`, `canonical_digest`. No panics/exits. |

### FT-002: vb_yaml::parse_workflow_source (YAML string input)

| Field | Value |
|-------|-------|
| **Target function** | `vb_yaml::parse_workflow_source(yaml: &str)` |
| **Input type** | `&str` — YAML string |
| **Risk** | Saphyr YAML parser panic, `StepPrimitive` deserialization bugs, `ScalarValue` parsing edge cases (very large integers, empty strings), `TogetherBranch` deserialization from malformed YAML |
| **Fuzzer** | `cargo-fuzz` (libFuzzer) |
| **Corpus seeds** | All valid primitive workflow YAML strings from v1_primitive_lowering.rs PRIMITIVE_CASES. Edge cases: only `parallel:` with no branches, `parallel:` with 50 branches, `parallel:` with empty label, `parallel:` with duplicate labels. |
| **Location** | `fuzz/fuzz_targets/parse_workflow_source_fuzz.rs` |
| **Coverage goal** | All `From<&Yaml>` impls for `StepPrimitive`, `TogetherBranch`, `StepAst`. No panics. |

## 6. Kani Harnesses

All Kani harnesses already exist. No new harnesses planned.

### Existing (6)

| Harness | Obligation | File | Status |
|---------|-----------|------|--------|
| `canonical_name_together_harness` | PO-001 | kani_canonical_name.rs:41 | **VERIFIED** (0/432 failed) |
| `canonical_name_aggregate_harness` | PO-008b | kani_canonical_name.rs:84 | **DEFERRED** — Aggregate fix out of scope, production returns `"aggregate"` not `"reduce"` |
| `canonical_name_all_harness` | PO-008 | kani_canonical_name.rs:137 | **BLOCKED** (TIMED_OUT >10min) — 12-variant symbolic enumeration exceeds solver capacity |
| `together_digest_sub_step_recursion_bounded_kani` | PO-009 | together_digest_kani.rs:54 | **BLOCKED** (BLOCKED_TOOLING: blake3 InlineAsm) |
| `together_digest_step_deterministic_kani` | PO-010 | together_digest_kani.rs:144 | **BLOCKED** (BLOCKED_TOOLING: blake3 InlineAsm) |
| `together_branch_count_produces_different_digest_kani` | PO-010b | together_digest_kani.rs:233 | **BLOCKED** (BLOCKED_TOOLING: blake3 InlineAsm) |

### Blocked obligations with compensating evidence

| Obligation | Block reason | Compensating evidence |
|-----------|-------------|----------------------|
| PO-008 (all variants) | Kani solver timeout | PO-001 verifies Together specifically. Other 11 variants tested by v1_primitive_lowering proptest regression. |
| PO-009 (recursion) | blake3 InlineAsm unsupported | Proptest 1000+ cases exercise identical code path; `MAX_CONSTRUCT_DEPTH` validation prevents unbounded input. |
| PO-010 (determinism) | blake3 InlineAsm unsupported | Proptest determinism (P-6) + idempotency unit test (PO-013) both PASS. |
| PO-010b (branch count) | blake3 InlineAsm unsupported | Proptest (P-1) + unit test (PO-014 sub-case a) both PASS. |

## 7. Mutation Checkpoints

Critical mutations that must be caught by the test suite. `cargo-mutants` will attempt to introduce these and tests must kill them.

| # | Mutation | Must be caught by |
|---|----------|-------------------|
| MC-1 | Change `part_05.rs:105` from `"together"` to `"parallel"` (revert fix) | `canonical_name_together_harness` (kani) + P-1 through P-5 (proptest: digests would not differ) |
| MC-2 | Remove `hasher.update(b"together")` line (part_05.rs:159) | P-1 through P-5 (proptest: digests would not differ for any together change) |
| MC-3 | Remove `hasher.update(&(branches.len() as u16).to_le_bytes())` (part_05.rs:160) | P-1 (proptest: branch count change would not affect digest) |
| MC-4 | Replace `branches.len() as u16` with constant `1u16` | P-1 (proptest: 2-branch and 3-branch would produce same digest) |
| MC-5 | Remove `hasher.update(branch.label.as_bytes())` (part_05.rs:162) | P-2 (proptest: different branch labels would produce same digest) |
| MC-6 | Remove inner loop `for step in &branch.steps` (part_05.rs:163-165) | P-3, P-4 (proptest: sub-step changes would not affect digest) |
| MC-7 | Remove `digest_sub_step(hasher, step)` call (part_05.rs:164) | P-3, P-4 (proptest: sub-step changes would not affect digest) |
| MC-8 | Swap branch iteration order (e.g., `.rev()`) | P-5 (proptest: reordered branches would produce same digest when they should differ) |
| MC-9 | Change `digest_sub_step` to skip `step.id` hashing (part_05.rs:175) | P-3 (proptest: different sub-step IDs would not produce different digests when primitives differ) |
| MC-10 | Change `digest_sub_step` to not call `digest_step_primitive` (part_05.rs:176) | P-3, P-4 (proptest: sub-step contents would not affect digest) |
| MC-11 | Replace `branches.iter()` with empty vec on line 161 | All together proptests (P-1 through P-5) would fail |
| MC-12 | Remove entire `Together` match arm (lines 158-167), falling through to `other` | P-1 through P-5 fail (digests become insensitive to together structure) |

**Mutation kill rate target**: ≥90%. All 12 critical mutations must be killed.

**Execution command**:
```bash
cargo mutants -p vb_compile --test-tool nextest \
  --file crates/vb_compile/src/mod_compile_lowering/part_05.rs \
  -- --test together_digest_sensitivity --lib tests::error_variant_tests
```

**Note**: Mutation testing is not yet configured in this workspace. The `mutants.toml` at the workspace root should be checked for compatibility with `cargo-mutants` v24+.

## 8. Combinatorial Coverage Matrix

### Unit test group: canonical_primitive_name

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| Together variant | `StepPrimitive::Together {..}` | `"together"` | kani | PASS (PO-001) |
| Together variant | `StepPrimitive::Together {..}` | `"together"` (not `"parallel"`) | unit | **GAP** (direct unit test) |
| Set variant | `StepPrimitive::Set {..}` | `"set"` | unit (indirect via proptest) | Covered |
| Save variant | `StepPrimitive::Save {..}` | `"save"` | unit (indirect via proptest) | Covered |
| Do variant | `StepPrimitive::Do {..}` | `"do"` | unit (indirect via proptest) | Covered |
| Choose variant | `StepPrimitive::Choose {..}` | `"choose"` | unit (indirect via proptest) | Covered |
| ForEach variant | `StepPrimitive::ForEach {..}` | `"for_each"` | unit (indirect via proptest) | Covered |
| Collect variant | `StepPrimitive::Collect {..}` | `"collect"` | unit (indirect via proptest) | Covered |
| Aggregate variant | `StepPrimitive::Aggregate {..}` | `"aggregate"` (known bug: should be `"reduce"`) | unit (indirect) + kani BLOCKED | Deferred |
| Repeat variant | `StepPrimitive::Repeat {..}` | `"repeat"` | unit (indirect via proptest) | Covered |
| Wait variant | `StepPrimitive::Wait {..}` | `"wait"` | unit (indirect via proptest) | Covered |
| Ask variant | `StepPrimitive::Ask {..}` | `"ask"` | unit (indirect via proptest) | Covered |
| Finish variant | `StepPrimitive::Finish {..}` | `"finish"` | unit (indirect via proptest) | Covered |
| Unknown/non_exhaustive fallback | any non-enumerated variant | `"unknown"` | unit (indirect via wildcard) | **GAP** (no test constructs non-exhaustive variant) |

### Unit test group: canonical_digest (Together sensitivity)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| Happy: 2-branch together | 2 branches with Set sub-steps | valid, non-zero, deterministic digest | unit + proptest | PASS |
| Happy: 3-branch together | 3 branches with Set sub-steps | valid, non-zero digest ≠ 2-branch digest | unit + proptest | PASS |
| Happy: single-branch together | 1 branch with Set sub-step | valid, non-zero, deterministic digest | unit | **GAP** |
| Happy: many-branch together | 10+ branches | valid, non-zero digest, no panic | unit | **GAP** |
| Happy: no together steps | Workflow with only Set, Finish | unchanged digest from pre-fix baseline | proptest | PASS (regression) |
| Edge: empty sub-steps | branch with `steps: []` | valid digest (branch label hashed, no sub-steps) | unit | **GAP** (blocked by validation, but worth testing programmatically) |
| Edge: 0 branches | `Together { branches: vec![] }` | valid digest (hashes "together" + 0u16 + no labels) | unit | **GAP** (blocked by validation, test programmatically) |
| Error: same structure, different branch count | 2-branch vs 3-branch | `assert_ne!(digest_a, digest_b)` | unit + proptest | PASS |
| Error: same structure, different branch labels | label-a vs label-b | `assert_ne!(digest_a, digest_b)` | unit + proptest | PASS |
| Error: same structure, different sub-step values | Set value "1" vs "99" | `assert_ne!(digest_a, digest_b)` | unit + proptest | PASS |
| Error: same structure, different sub-step output name | output "x" vs "y" | `assert_ne!(digest_a, digest_b)` | unit + proptest | PASS |
| Error: same branches, different order | [a,b] vs [b,a] | `assert_ne!(digest_a, digest_b)` | unit + proptest | PASS |
| Invariant: determinism | same source, called twice | `assert_eq!(digest_a, digest_b)` | unit + proptest | PASS |
| Invariant: idempotency | same source, called 3 times | `assert_eq!(d1, d2) && assert_eq!(d2, d3)` | unit | PASS |

### Unit test group: digest_sub_step

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| Happy: Together primitive | `StepAst { id, primitive: Together }` | hashes id + delegates to digest_step_primitive | implicit (via compile path) | PASS |
| Happy: Set primitive | `StepAst { id, primitive: Set }` | hashes id + delegates to digest_step_primitive | implicit (via compile path) | PASS |
| Happy: ForEach primitive | `StepAst { id, primitive: ForEach }` | hashes id + delegates to digest_step_primitive (only id + name for now) | unit | **GAP** |
| Determinism | same `StepAst`, called twice with fresh hashers | same digest | unit | **GAP** |
| Non-zero digest | any valid `StepAst` | digest != [0u8; 32] | unit | **GAP** |

### Unit test group: digest_step_primitive (Together arm)

| Scenario | Input Class | Expected Output | Test Layer | Status |
|----------|-------------|-----------------|------------|--------|
| Happy: with branches | Together with branches | hashes "together" + count + labels + sub-steps | implicit (via compile path) | PASS |
| Edge: single branch | Together with 1 branch | hashes correctly, no panic | unit | **GAP** |
| Edge: empty sub-steps | branch with `steps: []` | hashes label, no sub-steps, no panic | unit | **GAP** |
| Edge: 0 branches | Together with `vec![]` | hashes "together" + 0u16, no labels, no panic | unit | **GAP** |
| Determinism | same Together, called twice with fresh hashers | same blake3 output | kani (BLOCKED) + unit | **GAP** (unit) |

## 9. Gap Summary

| ID | Gap | Priority | Location | Effort |
|----|-----|----------|----------|--------|
| GAP-1 | Direct unit test for `canonical_primitive_name(Together) == "together"` | HIGH | error_variant_tests.rs (new test) | S — 5 lines: construct Together, call fn, assert_eq |
| GAP-2 | Exhaustive unit test for `canonical_primitive_name` all 12+1 variants | MEDIUM | error_variant_tests.rs (new test) | M — ~40 lines: match-all table-driven test |
| GAP-3 | Single-branch together unit test | MEDIUM | error_variant_tests.rs (new test) | S — 20 lines: YAML with 1 branch, compile, assert deterministic + non-zero |
| GAP-4 | Many-branch together stress test (proptest or unit) | LOW | together_digest_sensitivity.rs (new proptest) or error_variant_tests.rs (unit) | S — proptest with `1..=20usize` branch count |
| GAP-5 | Empty sub-steps (`steps: []`) within branch — programmatic test | LOW | error_variant_tests.rs (new test) | S — construct TogetherBranch manually, call digest_step_primitive, assert no panic + deterministic |
| GAP-6 | Zero-branch Together — programmatic test | LOW | error_variant_tests.rs (new test) | S — construct Together { branches: vec![] }, call digest_step_primitive, assert no panic |
| GAP-7 | `digest_sub_step` with non-Together primitive (ForEach) | MEDIUM | error_variant_tests.rs (new test) | S — construct StepAst with ForEach primitive, call digest_sub_step, assert non-zero + deterministic |
| GAP-8 | Fuzz targets for `compile_workflow` and `parse_workflow_source` | MEDIUM | fuzz/fuzz_targets/ (2 new files) | M — ~30 lines each + fuzz corpus seeding |
| GAP-9 | `digest_step_primitive` other-arm unit test | LOW | error_variant_tests.rs (new test) | S — loop over non-Together, non-Set, non-Finish variants, assert no panic |
| GAP-10 | Mutation testing configuration and execution | MEDIUM | mutants.toml + cargo-mutants run | M — verify mutants.toml compatibility, run, capture kill rate |
| GAP-11 | Branch label length stress proptest | LOW | together_digest_sensitivity.rs (new proptest) | S — proptest with 1..=256 char labels |
| GAP-12 | Direct `digest_sub_step` unit test (non-compile-path) | MEDIUM | error_variant_tests.rs (new test) | S — construct StepAst, call fn directly, assert hash ≠ [0; 32] |

### Priority ordering for test-writer

1. **GAP-1** (HIGH) — Direct name assertion is the only missing strong assertion for the primary fix
2. **GAP-3** (MEDIUM) — Single-branch edge case is a realistic usage pattern
3. **GAP-7** (MEDIUM) — Ensures `digest_sub_step` is generic, not Together-specific
4. **GAP-12** (MEDIUM) — Direct `digest_sub_step` test (non-compile-path) isolates the new function
5. **GAP-8** (MEDIUM) — Fuzz targets harden the YAML parsing boundary
6. **GAP-10** (MEDIUM) — Mutation testing proves tests aren't vacuous
7. **GAP-2** (MEDIUM) — Exhaustive name coverage (nice to have; Kani covers when not blocked)
8. **GAP-4**, **GAP-5**, **GAP-6**, **GAP-9**, **GAP-11** (LOW) — Edge cases and defense-in-depth

## 10. Test Execution Commands

```bash
# Proptest — together sensitivity (PO-002 through PO-006)
cargo test -p vb_compile --test together_digest_sensitivity -- --nocapture

# Proptest — regression gate (PO-007)
cargo test -p vb_compile --test v1_primitive_lowering -- --nocapture

# Unit — all together-specific tests (PO-011 through PO-015)
cargo test -p vb_compile --lib tests::error_variant_tests -- --nocapture

# Kani — canonical name regression (PO-001)
TMPDIR=target/tmp cargo kani -p vb_compile --harness canonical_name_together_harness --no-unwinding-checks

# Fuzz — compile_workflow (FT-001, after file created)
cargo fuzz run compile_workflow_fuzz -- -max_total_time=300

# Fuzz — parse_workflow_source (FT-002, after file created)
cargo fuzz run parse_workflow_source_fuzz -- -max_total_time=300

# Mutation — vb_compile critical paths (MC-1 through MC-12)
cargo mutants -p vb_compile --test-tool nextest \
  --file crates/vb_compile/src/mod_compile_lowering/part_05.rs \
  -- --test together_digest_sensitivity --lib tests::error_variant_tests

# Full moon CI gate
moon ci
```

## 11. Non-Goals (Excluded from Test Scope)

Per the bead contract non-goals section:

- Testing `for_each`, `collect`, `aggregate`, `repeat` nested-step sensitivity — these primitives have the same defect but are out of scope
- Testing `Aggregate` canonical name fix (`"aggregate"` → `"reduce"`) — deferred
- Testing `compute_compiled_digest` (byte-level digest) — different function, not modified
- Testing dead code in `compile/mod.rs` — not compiled, cleanup in separate bead
- Testing `StepAst` field-level hashing beyond `id` and `primitive` (i.e., `condition`, `with`, `retry`, `on_error`, `then`)
- E2E tests through a running workflow engine — digest is a compile-time artifact

## 12. Open Questions

1. **Should mutation testing be configured and run as part of this test plan execution, or deferred?** The `mutants.toml` exists at the workspace root but may need updates for `cargo-mutants` v24+. Recommend: verify configuration, run on `part_05.rs` scope, and record kill rate. If obstacles prevent execution within the test-writer state, record a follow-up bead.

2. **Should empty sub-steps (`steps: []`) be supported in validation?** Currently validation rejects empty branch steps. If this is a known future enhancement, the test for B-015 should be written against the programmatic API (construct `TogetherBranch` directly) rather than the YAML parser. If empty steps will always be rejected, B-015 can be documented as "blocked by validation" and tested only programmatically.

3. **Should the `canonical_name_all_harness` Kani timeout be accepted as a permanent waiver?** The 12-variant symbolic enumeration times out. Together is verified by the targeted harness. Accepting the waiver saves CI time without reducing safety. If accepted, add to `waiver-candidates.jsonl`.

4. **Should fuzz targets be created now or in a separate bead?** Fuzz targets require `cargo-fuzz` (libFuzzer) setup which includes a `fuzz/Cargo.toml`. The `fuzz/` directory exists at the workspace root. Verify the infrastructure and create targets if ready, otherwise file a follow-up bead.

## 13. Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario
- [x] Every pure function with multiple inputs has at least one proptest invariant
- [ ] Every parsing/deserialization boundary has a fuzz target **(GAP-8)**
- [x] Every error variant in the Error enum has an explicit test scenario (existing error_variant_tests.rs covers this; Together-specific errors are addressed)
- [x] The mutation threshold target (≥90%) is stated
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value (all existing tests assert specific digests or `assert_ne!`/`assert_eq!` with exact values)
- [ ] 12 test gaps are documented and prioritized (Section 9)

**Status**: 6/8 criteria met. Two gaps remain: fuzz targets (GAP-8) and gap documentation (which is done in this plan but execution deferred to test-writer).
