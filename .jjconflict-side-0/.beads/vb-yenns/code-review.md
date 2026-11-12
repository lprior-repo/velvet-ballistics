# vb-yenns — Independent Code Review (DeepSeek v4 Pro)

**Bead**: vb-yenns
**Reviewer**: independent code reviewer (DeepSeek v4 Pro via OMP)
**Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-vb-yenns-deepseek-review`
**Mode**: read-only source review; no source edits. Frozen-source evidence via `git show`/`git diff`/`grep` at exact SHAs. No `cargo` gates run (see test-review for run evidence).

Reviewed two frozen deliveries:

| Delivery | Commit | Parent |
| --- | --- | --- |
| A — all-branch audit + direct-API patch | `043d4528185e0ad1bbb0c8080669d755fd9f6706` | `90ab770633c2130501ae2db889374c205a960d69` |
| B — reusable OMP DeepSeek agent roles | `716db247558995f6cfb5c9b5674957b4bdc80307` | `1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` |

---

# Delivery A — 043d4528 branch audit + direct-API patch

**STATUS: APPROVED** (direct-API patch correct; two audit-classification findings for parent to correct, neither blocks the patch and neither mutates author artifacts here).

## Scope

Patch edits only `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` (test) plus three `.beads/vb-1nx31/*` audit artifacts. No runtime/core source changed. All runtime semantics cited below are read from the frozen base `90ab7706`.

## Contract parity (verified, not asserted)

1. **Authentic runtime-issued ActionTicket only** — CONFIRMED. The fabricated `action_ticket` helper (dummy `seq/attempt/idempotency_key/capacity`) is removed. Tests now obtain tickets via `Runtime::lookup_pending_action_ticket(run, 0)` (`vb_runtime/src/runtime.rs:961` at 90ab7706), the authority boundary that returns the shard's canonical `pending_actions` ticket and fails closed (`None`) on wire-step mismatch.
2. **Un-ignore real completion acceptance** — CONFIRMED. `#[ignore]` removed from `test_direct_api_action_completion_resumes_correct_run`. Base had exactly 1 `#[ignore]`; 043d4528 has 0. File has 14 `#[test]`.
3. **Negative tickets derived from real issuance** — CONFIRMED. Both negative completions are struct-update mutations of a genuinely issued ticket: `ActionTicket { run: unrelated, ..ticket }` and `ActionTicket { run: unrelated, action: ActionId::new(99), ..lookup_pending_action_ticket(unrelated, 0)? }`. Neither constructs fields from scratch.
4. **Accurate encoded length/digest** — CONFIRMED. `action_output` now computes `encoded_len = u32::try_from(encoded(&value)?.len())` (postcard) instead of the hardcoded `8`. The journal assertion computes `value_digest = *blake3::hash(&expected_bytes).as_bytes()`. Both match production exactly:
   - `preflight_action_completion` (`chunk_003_chunks/chunk_001_preflight_completion.rs:90`): `value_digest: *blake3::hash(&encoded_value).as_bytes()`.
   - `reject_encoded_len_mismatch` rejects any `ActionOutputReady.encoded_len` that does not equal the real postcard length — the old hardcoded `8` would have failed for `I64(4242)` (2-byte postcard varint), so the patch is load-bearing, not cosmetic.
5. **Failed completion no journal mutation** — CONFIRMED. Negative test snapshots `journal_events(&journal)?` and `counters_snapshot()` before the two rejections, then asserts exact equality after `tick_all()` (full-event-list equality, not `.len()`).

## The `[0; 32]` ABI digest — verified expected, not a defect

The `ActionCompletedEnvelope { action_abi_digest: WorkflowDigest::from_bytes([0; 32]) }` assertion is **faithful to production at the frozen base**, not an invented value:

`vb_runtime/src/shard/lifecycle/chunk_001.rs:551` (90ab7706), inside `apply_action_completion`:

```rust
self.append_journal_event(RuntimeJournalEvent::ActionCompletedEnvelope {
    ...
    action_abi_digest: vb_core::ids::WorkflowDigest::from_bytes([0; 32]),
})?;
```

The direct-API path (`submit_direct_with_inputs_grants_and_contracts`) records no per-action ABI digest at schedule boundary, so the runtime emits the zero digest. This is exactly the "no-call-contract" semantics the task asked to prove. The test pins current behavior and will fail when the integration author wires real ABI recording (a canary, not a false pin). Root integration author owns digest repair — not duplicated here.

## Findings

| # | Severity | Location | Disposition |
| --- | --- | --- | --- |
| A-1 | MEDIUM | `.beads/vb-1nx31/branch-disposition.{md,json}` | open — parent corrects classification |
| A-2 | LOW | `.beads/vb-1nx31/branch-disposition.{md,json}` | open — parent refreshes tipSHA |

### A-1 (MEDIUM): `vb-7x0ol-independent-integration-review` (18ef1b9c) misclassified as digest source

**Location**: `.beads/vb-1nx31/branch-disposition.md` "Blocked" section; `branch-disposition.json` entry `scope: "source", code_paths_count: 5`.

**Problem**: The disposition classifies `18ef1b9c2b91` as `scope: source` with 5 digest code paths ("5 source files all canonical DIGEST"). That is factually wrong. `18ef1b9c` is a **merge commit** with parents `1e7aeee24` and `c789b1b35`; its own delta is `.beads/vb-7x0ol/{code-review.md,test-review.md,evidence/c6_retained_input_failure.log}` — review evidence only. The 5 digest source files (`part_05_digest.rs`, `digest_unit_tests.rs`, `foreach_digest_tests.rs`, `wait_digest_unit_tests.rs`, `digest_framing_collision.rs`) are inherited from its **second parent `c789b1b35`**, which is the actual tip of `origin/codex/vb-a2l02-digest-boundaries-candidate`.

**Evidence**:
- `git show --stat 18ef1b9c2b91` → merge `Merge: 1e7aeee24 c789b1b35`, 8 files (3 `.beads/*` + 5 digest).
- `git merge-base --is-ancestor c789b1b35 18ef1b9c2b91` → YES.
- `git show --stat c789b1b35` → the 5 digest files, 725 ins / 249 del (the actual digest candidate).

**Impact**: Audit-accuracy only — the bottom line ("digest is out of scope, owned by c789b1b, blocked") is correct and no source was wrongly integrated or dropped. But a future auditor reading the disposition would misread 18ef1b9c as a source ref and under-count the review-vs-source split.

**Required fix (parent, not this review)**: Reclassify `vb-7x0ol-independent-integration-review` as `scope: review` (own-delta code_paths 0), and attribute the 5 digest files to `vb-a2l02-digest-boundaries-candidate` at its true tip.

### A-2 (LOW): `vb-a2l02-digest-boundaries-candidate` tipSHA stale

**Location**: `.beads/vb-1nx31/branch-disposition.md` "Blocked" section; `branch-disposition.json` entry `tipSHA: "29b9f85a5d596371791c0f9e66dc7522e282fda4", code_paths_count: 2`.

**Problem**: The disposition records tipSHA `29b9f85a` ("add boundary-safe framing", 2 files). The actual current tip of `origin/codex/vb-a2l02-digest-boundaries-candidate` is `c789b1b35` ("complete injective canonical encoding for semantic digest", 5 files, 593-line `part_05_digest.rs` rewrite).

**Evidence**: `git show-ref origin/codex/vb-a2l02-digest-boundaries-candidate` → `c789b1b35cb5d19a60cf2f786cbda5947109dcb1`.

**Impact**: The audit under-reports the digest delta scope (2 files vs 5 files + 593-line rewrite). Still correctly "blocked"/out-of-scope, so no integration consequence.

**Required fix (parent, not this review)**: Refresh tipSHA to `c789b1b35cb5d19a60cf2f786cbda5947109dcb1` and code_paths_count to 5. This is the same commit the disposition already names as "current counterpart: c789b1b digest repair" — the audit knew of it but recorded the pre-`c789b1b3` tip.

## Verified non-findings (checked, no defect)

- **Superseded refs** `59aa6d2e` (vb-8iilq parity) and `4ae3e8cb` (vb-3l40a recovery assertions): genuinely superseded, base carries the equivalent work.
  - `4ae3e8cb` sharpened assertions all present in base: `error: "duplicate action ID"` (`contract_validation_tests.rs:218,548`), `error: "artifact IR decode failed"` (`recovery_context_tests.rs:38`), `"duplicate completion must preserve the entire journal"` (`action_recovery_tests.rs:281`).
  - `59aa6d2e` "removed" lines are refactors: `hydrate_shape_checked_run_frame` relocated to `recovery/frame.rs:22` (base) with `RecoveredOpenAsk` moved to `recovery/full/product.rs`.
- **Redundant refs** `fd06cd11`/`b994b995` (vb-xx331): all 6 `naming_scan/*` + `vb_37lc_canonical_spelling_red/*` files byte-identical to base (`git diff --quiet <tip> base -- <path>` per-file).
- **Pre-existing `.unwrap()`** at `vb_vt2f_direct_runtime_api_acceptance.rs:205` (`ActionName::new("test-action").unwrap()`) predates the patch (present in base 90ab7706, line 205); the 043d4528 diff introduces zero `unsafe`/`unwrap`/`expect`/`panic`/`todo`/`dbg` — only `u32::try_from(...).map_err(...)` and `.ok_or(...)?`.
- **Integrated ref** `bb124563` (vb-cvuau) is not an ancestor of 043d4528 (divergent specialist delivery); its test-file content was integrated on top of base, not merged wholesale. The integrated version correctly retains base's `WorkflowParts.semantic_source_identity` field and `Finish` node output slot (`Some(1)`), which the specialist branch lacked.

---

# Delivery B — 716db2475 reusable OMP DeepSeek agent roles

**STATUS: APPROVED** (one LOW operational observation, non-blocking).

## Verified

1. **Byte-identical installed copies** — CONFIRMED. `cmp` on `.omp/agents/vb-deepseek-{rust,code-review,test-review}.md` vs `~/.omp/agent/agents/vb-deepseek-{rust,code-review,test-review}.md` → all three IDENTICAL.
2. **Concrete model pin + thinking** — CONFIRMED. All three frontmatter declare `model: [deepseek/deepseek-v4-pro]` and `thinkingLevel: high`.
3. **No fallback aliases / hidden cloud routing** — CONFIRMED. No `@slow`/`@smol`/`@plan`/`fallback`/`alias` keys in any frontmatter; the only "fall back"/"cloud" strings are the contract prohibition text ("Do not fall back to any other model or to cloud/secret-log routing").
4. **No override user config/secrets** — CONFIRMED. No `env`, `modelOverrides`, or secret references in any agent file.
5. **Unique roles, no self-approval** — CONFIRMED. Three distinct names/purposes; rust role contract: "must not self-approve its own code or tests"; both review roles: "must not be the same agent that wrote the change and must not self-approve".
6. **Docs accuracy (OMP roles only, not Codex picker)** — CONFIRMED. `docs/omp-deepseek-agent-roles.md` states roles are "OMP backend agents, not Codex native-picker models … They do not register any model in the Codex native picker or any global model config." Accurate.
7. **CLI task shape usable / examples pinned** — CONFIRMED. The documented `{"context", "tasks":[{"agent","task","solutionSpace"}]}` shape matches the actual smoke `task` tool args (`smoke-out.json` line 329: `args.context`, `args.tasks[].agent/task/solutionSpace`). Parent-pin flags documented and used.
8. **Real smoke resolves the pin** — CONFIRMED against `/tmp/vb-omp-smoke/smoke-out.json` (493-line JSONL). Final `tool_execution_end` result: all three roles `resolvedModel: deepseek/deepseek-v4-pro:high`, `resolvedModelIdentity: deepseek/deepseek-v4-pro`, `resolvedThinkingLevel: high`, `resolvedModelIsFallback: false`, `agentSource: user`, `exitCode: 0`. `resolvedModelIsFallback: false` proves the model came from the agent frontmatter, not a parent/fallback.
9. **No hardcoded response** — CONFIRMED. All three child final outputs contain the exact fixture token `vb18fhg-bf7eb0eca00112f64e9b02ab` (rust/test-review wrap it in a fence and note "32 characters, no trailing newline"; code-review returns it verbatim). Token is random-generated, not embedded in any agent file.
10. **Bounded/projectless smoke** — CONFIRMED. `async-off.yml` = `async.enabled: false`; `smoke-err.txt` = 0 bytes; session `cwd: /tmp/vb-omp-smoke`.

## Finding

### B-1 (LOW): read-only review roles cannot execute the suite under review

**Location**: `.omp/agents/vb-deepseek-code-review.md` / `vb-deepseek-test-review.md` contract: "NEVER edit files or trigger builds"; `tools` omit `edit`/`write`.

**Problem**: The independent test-review role is deliberately read-only and cannot run `cargo test` to satisfy the test-reviewer skill's "compile and execute deterministically" gate, or independently re-execute a suite (as in "independently execute lifecycle suites"). The `bash` tool is present, but the contract text forbids builds.

**Impact (real operational, not hypothetical)**: Dynamic verification in this flow must come from the implementation worker's committed evidence (e.g. `/tmp/test_stdout.log` for delivery A), which the read-only reviewer then verifies statically. No correctness gap — static assertion-strength/mutation review is fully doable read-only — but "independent re-execution" is not available to the review role as configured.

**Judgment**: Deliberate design; not a defect. No policy is invented; the role description ("reports test gaps, tautologies, wiring/mock-echo tests") is satisfiable read-only. Flagged so the parent is aware the "independent test review" step is static-only unless a separate executor is assigned.

---

## Verdicts

- **Delivery A code**: **APPROVED** — direct-API patch is correct and load-bearing; A-1/A-2 are audit-accuracy findings for the parent to correct (no source mutation here).
- **Delivery B code**: **APPROVED** — agent roles are correct, byte-identical, concretely pinned, and the pin is proven by real smoke resolution metadata; B-1 is a non-blocking operational observation.
