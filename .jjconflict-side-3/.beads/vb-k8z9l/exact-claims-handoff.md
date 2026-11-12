# vb-k8z9l: Narrow two-file artifact fixture repair

## Edits

### crates/workspace_tests/tests/vb_ssei_verification_admission_acceptance.rs

Removed debug `println` block and false "production bug" comments from `test_strict_verify_emits_certificate_when_workflow_is_safe`. Added explicit assertion that `admission.idempotency_attested().is_empty()` — this is the correct precise contract because `prepare_compiled_artifact` intentionally does not infer OPERATOR ATTESTATIONS from action declarations. IdempotencyClassification is RuntimeValidated (static declaration check), not OperatorAttested. Preserved exact granted capabilities, Strict policy, and artifact identity assertions.

**Corrected assertion logic:**

| Claim | Old (weakened) | New (precise) |
|---|---|---|
| idempotency_attested | Not asserted (commented out as "production bug") | Explicitly asserted `.is_empty()` |
| Reason | None | Static declaration check, not operator attestation |

### crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs

Removed dead helper `accepted_artifact_with_required_capability` (line 119) and the now-unused `REQUIRED_GATE_COUNT` import. This closes the `-D dead-code` compile error that failed `moon ci`.

## Verification

```bash
RUSTFLAGS=-Dwarnings cargo test -p velvet-ballistics-workspace-tests \
    --test vb_ssei_verification_admission_acceptance \
    --test vb_core_yaml_e2e_chain_contract
```

**Results:**
- `vb_ssei_verification_admission_acceptance`: 4 passed, 0 failed
- `vb_core_yaml_e2e_chain_contract`: 35 passed, 0 failed
- Both compile with `-Dwarnings` (no warnings)
- Formatted with installed rustfmt

## Prior draft note

The prior v4 draft removed the `idempotency_attested` assertion entirely and left debug printlns, calling it a "production bug." That was incorrect: the empty set is intentional (declaration-only fixture). This repair restores the precise contract and removes the dead code.

## Scope

Exactly two test files. No production source changes. No new panic/unwrap/expect/unsafe/todo/dbg. Qi37 remains separate; no global acceptance claimed.