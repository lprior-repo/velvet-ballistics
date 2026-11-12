# Local5090 Code Review: vb-g6d0j

**Bead:** vb-tphef (typed policy foundation)  
**Reviewer:** LOCAL5090 (fresh independent)  
**Candidate:** 478a03105c4feee111fda8be2c098110a3bc1caf  
**Base:** 8707f8e976f9a1f936a65341c924468be57f2920  
**Date:** 2026-10-22

## Verdict

**APPROVED_FOR_SCOPED_FOUNDATION**

The typed policy foundation is correct within its explicitly bounded scope: pure value types in `vb_core` and an actor-owned live evaluator in `vb_runtime`. No actual runtime authorization, admission persistence, emergency recovery, or v1 closure is claimed or required. The nine-file manifest matches the freeze hashes (verified by Python Python blake3 module; CLI tool unavailable on this machine).

All three hard gates pass on actual source with no stale evidence substitution: tests (49 core + 92 runtime, includes existing policy-related tests), strict lib clippy (`-D warnings`), and pinned fmt check.

## Gate Evidence

| Gate | Command | Exit | Seconds | Log |
|------|---------|------|---------|-----|
| Tests | `cargo test -p vb_core -p vb_runtime --lib -- policy --test-threads=1` | 0 | 14.58 | `.beads/vb-g6d0j/evidence/REV-1.*` |
| Clippy | `cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings` | 0 | 9.34 | `.beads/vb-g6d0j/evidence/REV-2.*` |
| Fmt | `cargo fmt -p vb_core -p vb_runtime --check` | 0 | 2.77 | `.beads/vb-g6d0j/evidence/REV-3.*` |

## Findings by Phase

### PHASE 1: Contract & Bead Parity

The implementation matches the contract exactly:

- **PolicyDigest**: 32-byte transparent wrapper, caller-supplied, never fabricated (lines 20-44 of `crates/vb_core/src/policy.rs`)
- **DecisionCode**: Allowed/Denied/Suspended as u8 repr enum with exhaustive coverage
- **PolicyLayer**: Live/Admission/Emergency u8 repr enum
- **PinnedAdmissionPolicy**: `Artifact(PolicyDigest)` or explicit `Unverified` — no fabricated zero digest
- **CurrentLivePolicy**: `Unconfigured` (explicit default-deny) or `Configured { digest, rule }` — bounded single rule per slice
- **PolicyRule**: `{ action: ActionId, code: DecisionCode }` — newtype `ActionId`, not raw u16
- **DecisionRecord**: Full 8-field ActionTicket, pinned admission identity, current live identity (Unconfigured vs Configured digest), input digest, taint, outcome, layer, typed emergency and compatibility influence enums
- **RuntimePolicy**: Preserved verbatim from base (Strict/Journaled/Relaxed) for durability semantics only

**LivePolicy evaluator** (`crates/vb_runtime/src/policy.rs`):
- Constructor `new()` takes no admission pin (per-run only) — correct
- `configure(digest, rule)` is infallible plain actor state assignment — correct
- `activate_emergency_deny()` is monotonic with no clear/reset API — correct
- `evaluate()` computes input digest via streaming BLAKE3 with domain separation over caller-supplied canonical bytes, taint, and all 8 ticket fields — correct

### PHASE 2: Farley Engineering Rigor

- Maximum function size: `compute_input_digest` at 27 lines including docstring. Logical statements are well under 25.
- Maximum parameters: `evaluate` has 4 parameters (`ticket`, `pinned_admission`, `input_bytes`, `taint`) — under 5
- Pure functional core: evaluation logic has no I/O, side effects, or global state

### PHASE 3: Holzman Rust (The Big 6)

- **Illegal states unrepresentable**: `PinnedAdmissionPolicy::Unverified` prevents fabricated digest; `CurrentLivePolicy::Unconfigured` is explicit default-deny
- **Parse, don't validate**: `encode_taint_canonical` exhaustively matches all 5 Taint variants; unknown variant becomes `InternalInvariantViolation`, not silently ignored
- **Types as documentation**: No boolean flags; enums name every state
- **Workflows**: Explicit evaluation layer ordering: Emergency → Live → record
- **Newtypes**: `PolicyDigest`, `ActionId`, `RunId`, `SeqNo`, `StepIdx` — no raw primitives in domain model

### PHASE 4: Ruthless Simplicity & DDD

- No Option-based state machines; enums encode state explicitly
- No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, or `dbg!()` in new code
- All error paths return `RuntimeResult` with typed `CoreError::InternalInvariantViolation` for the single unknown-taint case
- CUPID: Composable evaluator, Unix-philosophy single-responsibility modules, predictable evaluation ordering, idiomatic Rust, domain-based types

### PHASE 5: The Bitter Truth

- Code is boring, obvious, and readable — no cleverness, no "junior developer trying to prove smart" patterns
- YAGNI: No future-proofing traits, single-implementer abstract layers, or "while you're at it" extras
- Sniff test passes: a veteran reader can understand the 300-line core + 200-line evaluator + tests in minutes

## Test Coverage Assessment

**Core (7 original + 9 new = 16):**
- Original 7 RuntimePolicy tests preserved verbatim in `original_tests.rs`
- New: policy digest roundtrip, decision code discriminants, layer discriminants, unconfigured/configured digest accessors, decision record field preservation, admission pin types, influence state values

**Runtime (13 main + 12 ticket-field-variation + 10 taint + 3 emergency/admission = 38 policy-specific):**
- Unconfigured deny, explicit permit/deny/suspend, unknown action deny
- Emergency overlay: deny all, remains active after configure, influence state
- Decision record: preserves ticket identity, policy identities
- Input digest: changes on ticket field variation, taint variation, input bytes
- Admission: per-run artifact pin preserved, missing vs artifact distinguishable
- All 8 ticket fields individually tested for digest sensitivity
- All 5 taint variants produce distinct valid digests

Total filtered test run: 49 core + 92 runtime = 141 passing, 0 failed, 0 ignored.

## Scope Limitations (Not Defects)

- No actual runtime authorization enforcement; evaluator produces `DecisionRecord` that caller must act on
- No admission pin persistence across runs
- No emergency overlay persistence or recovery
- No retry/reauthorization semantics
- `compat_influence` is always `NotEvaluated` — shared `InfluenceState` enum has no embedded code (explicit foundation choice per coordinator freeze audit)
- Evaluator not yet called by dispatch; no executor-boundary enforcement
- No historical replay or v1 closure

These are all explicitly deferred parent integration obligations per the contract and freeze audit, not child defects.

## Real Child Findings

None. No fabricated policy, no authority minted by records, no hidden Arc/lock/mutable global, no unbounded payload clone, no test deletion, no forbidden constructs.
