# Current V1 Backend Acceptance Audit — Gaps vs Requirements

**Audit scope:** Backend/IR Interpreter Complete (per velvet-ballistics-MASTER.md §0)
**Source baseline:** main d5f38934 (read-only audit snapshot; later work is not covered)
**Audit date:** 2026-10-22
**Auditor:** local3090 scout (read-only)

## Executive Summary

Five P0 beads were open at this audit snapshot. The findings below are candidate delivery gaps, not release acceptance. Source presence and absence of a named type do not prove behavior coverage. Implementation status must be read from live beads; vb-ny068 was subsequently claimed.

**Suggested investigation order (not verified dependency edges or delivery estimates):**

| Rank | Bead | Feature | Proposed relationship, not tracker edge | Unvalidated estimate |
|------|------|---------|--------------|--------|
| 1 | vb-ny068 | Replay policy separation | None | S |
| 2 | vb-hk1j8 | Capability grant lifecycle | None | M |
| 3 | vb-ifn5b | Manual gate receipts | vb-hk1j8 | M |
| 4 | vb-f8lko | Compatibility algebra | None | L |
| 5 | vb-8vknz | Resource budget dimensions | vb-ny068 | L |

## Detailed Gap Analysis

### 1. vb-8vknz — Resource Budget Dimensions [P0] [OPEN]

**Requirement:** Master §77.18. Add missing minimum accounting for cumulative journal bytes, payload bytes, concurrent tickets, open gates, signal subscriptions, incident emissions, cost units, tenant quota, global quota. Runtime remaining-budget counters with typed exhaustion and release.

**Current state:**
- `WholeWorkflowBudget` (crates/vb_core/src/budget.rs:12) has 21 dimensions covering: steps, slots, fanout, nesting, action tickets, retries, gather pages/items, for-each iterations, together branches, repeat attempts, run time, result bytes, slots written, timer entries, trace events, journal batch bytes, queue depth, IPC payload bytes, blob bytes, input bytes
- `AggregateResourceBudget` (budget.rs:571) tracks per-run aggregate
- `AggregateResourceUsage` (budget.rs:624) tracks shard usage
- Admission checks exist in crates/vb_runtime/src/admission.rs

**Missing dimensions (not in WholeWorkflowBudget):**
- `max_cumulative_journal_bytes` (only `max_journal_batch_bytes` exists — per-batch, not cumulative)
- `max_concurrent_tickets` (only `max_action_tickets` total, not concurrent in-flight)
- `max_open_gates`
- `max_signal_subscriptions`
- `max_incident_emissions`
- `cost_units` (per-action or per-workflow)
- `tenant_quota` (multi-tenant resource isolation)
- `global_quota` (server-wide aggregate limit)

**Missing runtime behavior:**
- Missing resource dimensions need a separate call-path audit; do not infer absence of all runtime accounting.
- Existing runtime admission validates aggregate usage and capacity in `crates/vb_runtime/src/admission.rs:98` (`validate_requested_budget`). Typed overflow, underflow, policy and capacity errors are mapped in `admission/parts/chunk_006_admit_budget.rs`.
- `AggregateResourceUsage::try_add_budget` and subtraction/policy/capacity operations exist in `crates/vb_core/src/budget.rs`; this alone does not prove reservations are maintained over the full run lifecycle.
- `StepBudget` consumption/exhaustion and retry budget behavior already have tests. The initial broad claim that only compile-time validation exists was incorrect.
- Tenant/global and the listed missing dimensions, including lifecycle reservation/release, remain a scoped audit requirement.

**Tests:** Existing aggregate arithmetic/admission and step-budget tests are present. This audit did not execute or prove coverage for each missing dimension or reservation lifecycle.

---

### 2. vb-f8lko — Compatibility Algebra [P0] [OPEN]

**Requirement:** Master §§77.5,77.12,77.17. Typed six-class algebra: Identical, BackwardCompatible, ForwardCompatible, MigrationRequired, ReplayOnly, Rejected. Decision record binding with rule digest, old/new schema digests. Apply at all trust boundaries.

**Current state:**
- Basic schema version validation in crates/vb_storage/src/codec/header.rs
- `decode_record_header` validates magic, schema version, kind, family, length
- `decode_envelope_only` (codec/envelope.rs) validates but returns raw payload
- No compatibility classification — only "is this version supported?" binary check

**Missing:**
- Six-class compatibility enum with bounded rule table
- Decision record type with: deciding-rule digest, old schema digest, new schema digest, decision code
- Shared decision seam applied at: IR/action ABI, input-output schemas, policy input, signal schemas, journal/snapshot/debug-map/resource-contract/incident/SDK-ABI/artifact-envelope versions
- `ReplayOnly` never authorizes new live effects
- `MigrationRequired` requires explicit migration before dispatch
- Journal records for compatibility decisions
- Typed public errors for each incompatibility class

**Tests:** None for six-class algebra. No table-driven boundary matrix tests.

---

### 3. vb-ifn5b — Manual Gate Receipts [P0] [OPEN]

**Requirement:** Master §77.15. Distinct typed manual-gate policy/decision/receipt with five decision kinds: approve, deny, request_more_evidence, delegate, break_glass. Separation of duties, evidence bundle digest, prompt digest, timeout/escalation, break-glass permission, required reason, receipt digest, replay rule.

**Current state:**
- `AskTicket` (crates/vb_runtime/src/shard/types.rs:295) carries run/ask_step/resume_step
- `AskAnswer` (`types.rs:358`) carries value/taint/slot — generic suspension/resumption
- No approval semantics, no role checking, no evidence bundle
- `RecoveryClass::ManualGate` exists (crates/vb_core/src/action_pack.rs:36) as enum variant only

**Missing:**
- `ManualGatePolicy` type: risk class, approver role, separation-of-duties rule, evidence requirements, timeout/escalation policy
- `ManualGateDecision` type: approve/deny/request_more_evidence/delegate/break_glass with approver identity, role proof, reason, evidence acknowledgment
- `ApprovalReceipt` type: digest-bound record with gate id, decision, timestamp, evidence bundle digest
- Separation-of-duties enforcement (requester ≠ approver for high-risk)
- Break-glass permission model and audit trail
- Timeout/escalation policy enforcement
- Receipt cannot mint capabilities (digest-bound)
- Replay rule: reconstruct historical decisions without external effects

**Tests:** None for manual gate decisions, roles, evidence, timeout, break-glass, replay.

---

### 4. vb-hk1j8 — Capability Grant Lifecycle [P0] [OPEN]

**Requirement:** Master §77.10. Authority-bound grant value with grant id, issuer, subject, artifact/run/action binding, capability/resource scopes, issue/expiry time, delegability, revocation epoch, break-glass marker, approval receipt digest. Reject wrong issuer/subject/scope/expired/revoked before dispatch.

**Current state:**
- `Capability` (crates/vb_core/src/capability.rs:10) has name and action only
- `CapabilitySet` (capability.rs:31) is Vec of Capabilities
- `check_capability` (crates/vb_runtime/src/admission.rs:116) checks name/action membership at admission
- No issuer, no subject, no scopes, no expiry, no revocation, no delegability
- No grant lifecycle — granted at admission, never expires or revokes

**Missing:**
- `CapabilityGrant` type with all lifecycle fields
- Grant issuance flow with issuer identity
- Subject binding (which run/actor holds the grant)
- Scope validation (resource-level scoping)
- Expiry checking at dispatch time
- Revocation mechanism and revocation checking
- Delegability rules
- Break-glass marker
- Approval receipt digest linkage
- Narrowed scope passed to executor (not full grant)
- Replay preserves historical grant evidence; revoked grant blocks next live effect

**Tests:** None for grant lifecycle, expiry, revocation, scope validation, delegation.

---

### 5. vb-ny068 — Replay Policy Separation [P0] [OPEN]

**Requirement:** Master §§77.3,77.11. One typed policy decision seam around action outbox dispatch. Pin admission/replay policy digest; evaluate current live-effect policy plus monotonic emergency deny overlay before new effects. Journal policy digest, input digest, decision code, policy layer.

**Current state:**
- `RuntimePolicy` (crates/vb_core/src/policy.rs:7) is static enum: Strict, Journaled, Relaxed
- `RuntimePolicy` controls admission verification/durability; it is not a live-effect authorization decision
- No policy evaluation at dispatch time
- No emergency deny overlay
- Artifact admission already has a `policy_digest`; the missing claim is a production binding from pinned replay/admission authorization to the separate current dispatch decision
- `replay_policy_blocked` field exists in proof kernels (crates/vb_core/src/proof_kernels/vb_kyyf_normalization/) but only for normalization comparison, not runtime enforcement

**Missing:**
- Policy decision seam at action outbox dispatch
- Admission/replay policy pinned at artifact admission with digest
- Live-effect policy evaluated at dispatch time
- Monotonic emergency deny overlay (can only tighten, never loosen)
- Journal event with: policy digest, input digest, decision code, policy layer
- Replay reconstructs historical decisions without external effects
- Old admission does not authorize future effects after revocation/deny
- Typed suspension/denial error when policy rejects dispatch
- No runtime policy-name lookup, YAML evaluation, dynamic code

**Tests:** None for policy separation, overlay, replay reconstruction, revocation blocking.

---

## What's NOT Missing (Stale Tracking / Already Implemented)

- **Compiler/IR:** Whole workflow budget computation exists (budget.rs)
- **Admission infrastructure:** Run admission with artifact digest, capability checks, budget validation exists
- **Ask/suspension:** Generic AskTicket/AskAnswer suspension/resumption exists
- **Idempotency tracking:** IdempotencyTracker with policy-aware tracking exists
- **Recovery/replay infrastructure:** Journal replay, snapshot-plus-tail recovery, digest mismatch detection exists
- **Codec/envelope:** Record envelope validation with schema version checking exists

---

## Smallest Safe Unblocked P0 Scope

**vb-ny068 (replay policy separation)** was selected for the next implementation; its size is not established by this source audit:
- Requires adding a policy decision seam at action dispatch
- Keep durability/admission `RuntimePolicy` separate from effect authorization
- Requires typed authority, decision and journal/recovery contracts; exact blast radius needs implementation analysis
- Tests can use existing test workflows

**Recommended contract inputs for vb-ny068:**
- Define a concrete typed decision seam at the actual effect/outbox boundary; no trait or dynamic dispatch is prescribed by this audit
- Add `PolicyDecision` enum: Allow, Suspend, Deny
- Add `EmergencyDenyOverlay` type with monotonic tighten-only operations
- Add journal event `PolicyEvaluated` with policy digest, input digest, decision, overlay applied
- Tests: admit then deny before dispatch, overlay cannot loosen, replay reconstruction, crash/reopen preservation

---

## Notes for Implementation

1. **No new crates needed** — all five features fit in existing crates (vb_core, vb_runtime, vb_storage)
2. **Schema migrations required** — policy decisions, manual gate receipts, capability grants, compatibility decisions all need journal event types and potentially storage keyspace entries
3. **Replay determinism maintained** — policy inputs must be journaled as typed facts, no ambient clock/network in deterministic core
4. **Wire schema compatibility** — new journal event types must not break existing event kind numbering
5. **Kani/proof harnesses** — each new type and decision path needs verification harnesses per AGENTS.md rules

## Coordinator corrections and limits

Corrected on 2026-10-22 after inspecting production admission/core budget operations. The first local audit overstated absence of runtime budget support, proposed unverified dependency edges, and conflated durability policy with authorization. These statements are not acceptance evidence. No runtime command or behavioral test was executed for this audit. The five P0 beads remain responsible for their own production wiring, behavioral verification and independent reviews. The current code must be rechecked before implementing any listed gap.
