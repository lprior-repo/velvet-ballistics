# vb-yenns — Independent Test Review (DeepSeek v4 Pro)

**Bead**: vb-yenns
**Reviewer**: independent test reviewer (DeepSeek v4 Pro via OMP)
**Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-vb-yenns-deepseek-review`
**Mode**: read-only; assertion-strength + evidence review. Frozen-source assertions read via `git show` at exact SHAs; run evidence read from `/tmp` (not re-run — delivery A source not in working tree, per scope).

---

# Delivery A — direct-API acceptance suite (043d4528)

**STATUS: APPROVED**

## Suite surface

File `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` at `043d4528`:
- `#[test]` count: **14**
- `#[ignore]` count: **0** (base `90ab7706` had **1** — `test_direct_api_action_completion_resumes_correct_run`)
- The previously-ignored completion test is now a live acceptance test.

## Assertion strength (checked against production at base 90ab7706)

| Gate | Result | Evidence |
| --- | --- | --- |
| Integration tests use public API only | ✅ | `Runtime::submit_direct`, `submit_direct_with_inputs_grants_and_contracts`, `lookup_pending_action_ticket`, `complete_action_with_output`, `fail_action`, `lookup_pending_ask_ticket`, `answer_ask`, `list_events`, `snapshot_run`, `tick_all` — all public |
| Authentic ticket authority (no fabricated fields) | ✅ | `lookup_pending_action_ticket(run, 0)` is the sole ticket source; fabricated `action_ticket` helper removed |
| Negative tickets derive from real issuance | ✅ | struct-update `..ticket` / `..lookup_pending_action_ticket(unrelated, 0)?`, not field-fabricated |
| Exact error variants (not `is_err()`/`Some(_)`) | ✅ | `Err(RuntimeError::InvalidActionCompletion)`, `Err(RuntimeError::AdmissionArtifactNotFound { digest })`, `Err(RuntimeError::ShutdownInProgress)` — exact variants |
| Concrete assertions (not boolean smoke) | ✅ | exact `SlotValue`, exact `Taint`, exact `TraceEvent` vectors, exact journal envelopes |
| Failed completion no journal mutation | ✅ | full `journal_events(&journal)? == journal_before_rejections` equality + `counters_snapshot()` equality after both rejections and `tick_all()` |
| Boundary cases | ✅ | known/absent run, stale/terminal ask ticket, evicted-trace terminal, wrong-run/wrong-action ticket, wrong answer slot |
| Determinism | ✅ | `tick_all()` explicit, no sleeps, no shared mutable state, volatile journal per-test |

## The suspicious `[0; 32]` digest — proven expected, not a false pin

The test asserts `action_abi_digest: WorkflowDigest::from_bytes([0; 32])`. Production `apply_action_completion` (`shard/lifecycle/chunk_001.rs:551` at 90ab7706) hardcodes exactly that value because the direct-API path records no per-action ABI digest at schedule boundary. The assertion is a faithful mirror of current behavior, and it is a **mutation canary**: when the integration author wires real ABI recording, this test fails until the assertion is updated. Not a defect; correct "no-call-contract" semantics.

`value_digest` and `encoded_len` are likewise exact: `preflight_action_completion` computes `value_digest = *blake3::hash(&encoded_value).as_bytes()` and rejects any `encoded_len` mismatch against the real postcard length — the patch's `encoded_len = u32::try_from(encoded(&value)?.len())` is required for the completion to be accepted (the old hardcoded `8` would fail for a 2-byte `I64(4242)`).

## Mutation resistance spot-check

- Delete the `lookup_pending_action_ticket` boundary → every positive completion test loses its ticket source and fails. Caught.
- Restore the fabricated `action_ticket` helper with dummy `seq/attempt/idempotency_key` → `reject_invalid_ticket_key` (idempotency mismatch) fails the completion. Caught.
- Reintroduce `#[ignore]` on the completion test → gate "0 ignored" fails and the completion behavior is no longer exercised. Caught.
- Drop the `journal_before_rejections` equality → a regression that mutates the journal on a rejected completion would pass. Caught by the exact-equality assertion.
- Hardcode `encoded_len: 8` again → `reject_encoded_len_mismatch` fails. Caught.

## Run evidence

Prior worker's actual run log at `/tmp/test_stdout.log` (mtime 2026-10-21 19:18, after commit 043d4528 at 19:12) shows the real result — not re-run by this reviewer (delivery A source is not in this working tree; the runtime it exercises is the frozen base, not HEAD):

```
running 14 tests
... 14 x ok ...
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

`test_direct_api_action_completion_resumes_correct_run ... ok` is present (the un-ignored test passes). No stderr/exit artifact contradicting the `ok` line was found; the log is a complete `cargo test` summary block.

**Explicit limitation**: this test review does not claim to have independently re-executed the suite. Verdict rests on (a) assertion-strength analysis of the frozen source and (b) the prior worker's verified run log. The parent owns re-execution after source freeze.

## Finding

### T-A1 (OBSERVATION): sibling `ask_answer` helper still hardcodes `encoded_len: 8`

**Location**: `vb_vt2f_direct_runtime_api_acceptance.rs` `fn ask_answer` (unchanged by 043d4528).

**Problem**: `action_output` was corrected to compute the real postcard `encoded_len`, but the sibling `ask_answer` helper still emits `encoded_len: 8`. The ask path does not reject on this today (the 14/14 pass), so it is not a failing assertion.

**Impact**: cosmetic inconsistency, no behavioral gap, pre-existing and out of this patch's blast radius (the patch touched action completion, not ask answer).

**Disposition**: owner_approved_no_action (pre-existing, not introduced by 043d4528).

---

# Delivery B — OMP smoke evidence (716db2475)

**STATUS: APPROVED**

Review of `.beads/vb-18fhg/smoke-evidence.md` against the actual `/tmp/vb-omp-smoke/smoke-out.json` stream (parsed as JSONL, 493 events — thoughts/full stream not dumped).

## Smoke evidence verified against the stream

| Claim in smoke-evidence.md | Verified in stream |
| --- | --- |
| Three roles dispatched via `task` in one batch | `task` tool args `tasks[0..2].agent = vb-deepseek-rust / vb-deepseek-code-review / vb-deepseek-test-review` |
| `resolvedModel` = `deepseek/deepseek-v4-pro:high` | ✅ all three `result.details.results[*].resolvedModel` |
| `resolvedModelIdentity` = `deepseek/deepseek-v4-pro` | ✅ all three |
| `resolvedThinkingLevel` = `high` | ✅ all three |
| `resolvedModelIsFallback` = `false` | ✅ all three (`False`) |
| `agentSource` = `user` | ✅ all three (`user`) |
| exitCode 0 | ✅ all three `exitCode: 0` |
| All three reported the exact token | ✅ outputs contain `vb18fhg-bf7eb0eca00112f64e9b02ab` verbatim; rust/test-review add "32 characters, no trailing newline" |
| Parent async disabled | ✅ `async-off.yml` = `async.enabled: false` |
| Projectless run | ✅ session event `cwd: /tmp/vb-omp-smoke` |
| No stderr | ✅ `smoke-err.txt` = 0 bytes |

`resolvedModelIsFallback: false` is the decisive evidence that the child model resolves from the agent frontmatter `model: [deepseek/deepseek-v4-pro]` (not the parent model or a fallback alias). Model-pin config alone would be insufficient; the runtime resolution metadata closes it.

## Finding

None. The smoke is a bounded, benign, read-only token-read proof with real resolution metadata. No tautological or hardcoded-response evidence.

---

## Verdicts

- **Delivery A test**: **APPROVED** — 14/14, 0 ignored, authentic-ticket authority, strong exact-equality no-mutation assertions, `[0;32]` proven faithful to production; T-A1 is a pre-existing observation only.
- **Delivery B test/smoke**: **APPROVED** — all resolved-model claims verified against the raw stream; no findings.
