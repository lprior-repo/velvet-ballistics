# Independent Test Review: vb-kwsk1

**Reviewer:** local3090 (fresh independent session)
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-kwsk1-local3090-test-review
**Bead:** vb-tphef (typed policy foundation)
**Candidate:** 478a03105c4feee111fda8be2c098110a3bc1caf
**Base:** 8707f8e976f9a1f936a65341c924468be57f2920
**Source freeze verified:** foundation-source-freeze-v2.json (9 files, all hashes match)

## Evidence (Actual Gate Runs)

All gate runs used `python3 /tmp/vb-run-gate.py`; raw logs in `.beads/vb-kwsk1/evidence/`.

### Test Gate
```
python3 /tmp/vb-run-gate.py 120 .beads/vb-kwsk1/evidence/testrun-1 \
  cargo test -p vb_core -p vb_runtime --lib -- policy --test-threads=1
```
- exit_code: 0
- seconds: 33.457
- vb_core: running 49 tests; test result: ok. 49 passed; 0 failed; 0 ignored
- vb_runtime: running 92 tests; test result: ok. 92 passed; 0 failed; 0 ignored
- Total: 141 tests (includes pre-existing policy-related tests per freeze audit)
- Zero tests rejected: confirmed non-zero match counts

### Clippy Gate (Strict)
```
python3 /tmp/vb-run-gate.py 120 .beads/vb-kwsk1/evidence/clippy-1 \
  cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings
```
- exit_code: 0
- seconds: 6.566

### Fmt Gate (Pinned, Read-Only)
```
python3 /tmp/vb-run-gate.py 60 .beads/vb-kwsk1/evidence/fmt-1 \
  cargo fmt -p vb_core -p vb_runtime --check
```
- exit_code: 0
- seconds: 0.578

## Test Suite Assessment

### Core Tests (49 total: 7 original + 8 new policy + other policy-related)

**Original tests preserved:** All 7 RuntimePolicy tests in `original_tests.rs` retained with identical assertions (variant distinctness, copy semantics, debug output, clone equality). No weakened assertions relative to base.

**New typed policy tests (new_tests.rs):**
- Digest roundtrip: checks first/last bytes — adequate for transparency wrapper
- Discriminant tests: verify encoding stability for DecisionCode and PolicyLayer
- Unconfigured/configured state tests: verify digest() and rule() accessors
- Decision record field preservation: comprehensive field assertions
- Pinned admission variant test: verifies Unverified is not a digest
- InfluenceState value tests: verify equality semantics

### Runtime Tests (92 total: 12 main + 9 ticket variation + 6 taint + 3 emergency/admission + other policy-related)

**Main policy evaluator tests (tests.rs):**
- Unconfigured deny: outcome + layer asserted ✓
- Explicit permit/deny/suspend: all three DecisionCode values tested ✓
- Unknown action deny: non-matching rule defaults to deny ✓
- Emergency overlay denies all: outcome + layer + influence state asserted ✓
- Overlay monotonic after configure: emergency survives subsequent configure ✓
- Ticket identity preservation: specific fields asserted (run, action) ✓
- Policy identity preservation: admission identity asserted ✓
- Ticket field variation → digest change ✓
- Taint variation → digest change ✓
- Unverified vs Artifact admission distinguishable ✓

**Ticket field variation tests (ticket_field_variation_tests.rs):**
- All 8 ticket fields tested individually for digest sensitivity ✓
- Each test varies exactly one field, asserts digest change and varied field value ✓
- Input bytes variation tested with three distinct values ✓

**Taint tests (taint_tests.rs):**
- All 5 Taint variants produce valid (non-zero) digests ✓
- All 5 variants produce pairwise-distinct digests ✓

**Emergency and admission tests (emergency_and_admission_tests.rs):**
- Per-run artifact pin remains exact ✓
- Emergency overlay cannot be loosened by future config ✓
- Configured policy after emergency still denied ✓

## Findings

### 1. Tautology in new_tests.rs (MINOR)
**File:** `crates/vb_core/src/policy/new_tests.rs`
**Test:** `decision_record_preserves_all_fields`
**Line:** `assert_eq!(record.ticket, record.ticket);`

This assertion compares the ticket to itself — always true regardless of implementation. The test has other meaningful assertions that verify actual field preservation, so the test is still valid overall. The tautology adds no value and could mask a regression if other assertions were later removed.

### 2. Tautology in tests.rs (MINOR)
**File:** `crates/vb_runtime/src/policy/tests.rs`
**Test:** `decision_record_preserves_ticket_identity`
**Line:** `assert_eq!(record.ticket, record.ticket);`

Same tautology as finding 1. This test also has meaningful assertions for specific fields (run, action), so it remains valid. The tautology is redundant.

### 3. No direct test of encode_taint_canonical in core (OBSERVATION)
The `encode_taint_canonical` function in `vb_core` is tested indirectly through runtime digest sensitivity tests. No core-level test directly calls this function. This is acceptable given the runtime tests exercise all 5 variants, but a direct core test would improve isolation.

### 4. No zero-digest prohibition test (OBSERVATION)
The contract states PolicyDigest should never be a fabricated zero digest. No test explicitly verifies that a zero-digest is not auto-generated or that the type rejects it. Given PolicyDigest is a transparent wrapper with no validation, this is a caller responsibility — consistent with the contract.

## Fault Sensitivity Analysis

Mutation thought experiment (no actual mutation tool run):
- **Delete emergency check:** Caught by `emergency_overlay_denies_all` (would return Allowed at Live)
- **Delete unconfigured check:** Caught by `unconfigured_deny` (would panic or return wrong code)
- **Delete rule matching:** Caught by `explicit_permit`, `unknown_action_deny`
- **Delete taint encoding:** Caught by `changing_taint_changes_digest`, `all_taint_variants_produce_distinct_digests`
- **Delete ticket field encoding:** Each field has dedicated test in `ticket_field_variation_tests.rs`
- **Delete admission preservation:** Caught by `decision_record_preserves_policy_identities`
- **Delete monotonic emergency:** Caught by `overlay_remains_restrictive_after_configure`
- **Delete input bytes encoding:** Caught by `varying_input_bytes_changes_digest`

## Scope Limitations

This is a PURE TYPED FOUNDATION only. The following are NOT tested and NOT claimed:
- Actual dispatch integration
- Durable storage/replay
- Admission pin persistence across runs
- Durable emergency overlay persistence
- Compatibility influence evaluation (always NotEvaluated)
- Multi-rule policy (single rule per configuration)
- Arc/Mutex/atomic concurrency (plain owned state only)

## Verdict

**APPROVED_FOR_SCOPED_FOUNDATION**

All 141 tests pass (49 core + 92 runtime). Strict clippy and pinned fmt pass. All original 7 RuntimePolicy tests preserved verbatim. The typed policy foundation is well-tested for its declared scope: live policy evaluation, decision record identity preservation, digest sensitivity to all ticket fields and taint variants, emergency overlay monotonicity, and admission pin distinction. Two minor tautologies exist in ticket identity preservation tests but do not invalidate the test suites. The scope is narrow by design (foundation only), and the tests appropriately cover that narrow scope without overclaiming parent-level acceptance.