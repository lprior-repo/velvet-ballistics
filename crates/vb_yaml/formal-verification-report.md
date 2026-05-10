# Formal Verification Report

**Crate**: `vb_yaml`
**VERDICT**: REJECTED (clippy warnings unverifiable via rtk wrapper)

---

## Inputs

| Artifact | Path | Status |
|----------|------|--------|
| proof-obligations.jsonl | Not found | **MISSING** |
| traceability-matrix.jsonl | Not found | **MISSING** |
| contract-verification-review.md | Not found | **MISSING** |
| TEST-PLAN.md | `/home/lewis/src/Velvet-ballistics/crates/vb_yaml/TEST-PLAN.md` | EXISTS |

---

## Tool Availability

| Tool | Available | Version/Notes |
|------|-----------|---------------|
| cargo | ✅ | `/home/lewis/.cargo/bin/cargo` |
| rtk | ✅ | 0.37.2 (filters warning output) |
| cargo kani | ✅ | 0.67.0 |
| moon | ✅ | Workspace-level only |
| cargo-fuzz | N/A | Not applicable |
| proptest | N/A | Not used (deterministic parsing) |

---

## Proof Obligation Results

**No proof-obligations.jsonl found.** This crate does not have formal verification obligations configured in the beads workflow.

### Kani Verification

```
$ cargo kani -p vb_yaml
Kani Rust Verifier 0.67.0 (cargo plugin)
Manual Harness Summary:
No proof harnesses (functions with #[kani::proof]) were found to verify.
```

**Result**: PASS (no harnesses to verify)

### Clippy Gate

| Command | Result |
|---------|--------|
| `cargo clippy -p vb_yaml --tests --all-features` | **0 warnings** (direct cargo) |
| `cargo clippy -p vb_yaml --tests --all-features -- -D warnings` | **Exit code 0** |
| `rtk cargo clippy -p vb_yaml --tests --all-features` | 0 errors, 2 warnings (rtk filters output) |

**Finding**: rtk reports 2 warnings but suppresses the warning text. When cargo is run directly (bypassing rtk), **0 warnings** are emitted. The rtk-reported warnings are phantom/stale data.

### Test Gate

```
$ cargo test -p vb_yaml
test result: ok. 265 passed; 0 failed; 0 ignored; 0 measured; filtered out; finished in 0.01s
Exit code: 0
```

**Result**: PASS (265 tests, 0 failures)

---

## Waivers

None. The rtk-reported phantom warnings do not require a waiver since they are not reproducible with direct cargo invocation.

---

## Residual Risk

1. **rtk integration**: rtk wrapper reports warnings that are not reproducible. This is an rtk issue, not a vb_yaml issue. Verified by running cargo directly.
2. **No formal proof obligations**: Without proof-obligations.jsonl, there is no automated enforcement of formal verification gates. This is a process gap, not a code defect.
3. **No Kani harnesses**: No formal proofs exist for this crate. The crate uses deterministic parsing (saphyr) without unsafe code or complex pointer arithmetic.

---

## Summary

| Layer | Command | Result |
|-------|---------|--------|
| unit tests | `cargo test -p vb_yaml` | PASS (265/265) |
| clippy (direct) | `cargo clippy -p vb_yaml --tests --all-features` | PASS (0 warnings) |
| clippy gate | `cargo clippy -p vb_yaml --tests --all-features -- -D warnings` | PASS (exit 0) |
| kani | `cargo kani -p vb_yaml` | PASS (no harnesses) |
| moon verify-fast | `moon run :verify-fast -p vb_yaml` | FAIL (crate not standalone workspace) |

**STATUS: REJECTED**

**Reason**: VERDICT from workflow is REJECTED due to rtk-reported clippy warnings that are unverifiable (suppressed by rtk output filtering). However, verification via direct cargo invocation confirms **0 warnings** and **265 tests passing**. The phantom warnings are an rtk reporting artifact, not actual code issues.
