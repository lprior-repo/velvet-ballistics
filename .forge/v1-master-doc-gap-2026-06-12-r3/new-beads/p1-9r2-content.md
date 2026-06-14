P1-9r2 verify-15-gates: Expand verify to enumerate master §63's 15 named verification gates (exact names from master doc, no fabricated hex codes)

# Verification excerpts (read-before-write)

## Master doc §63 (lines 3053-3082) — VERBATIM
The 15 named gates from the verification gate pipeline:
1. `profile` — gate 1: profile (YAML/Rust workflow definition)
2. `shape` — gate 2: shape (schema validator)
3. `names` — gate 3: names (name/scope validator)
4. `references` — gate 4: references (reference validator)
5. `expressions` — gate 5: expressions (expression compiler)
6. `CFG` — gate 6: CFG (control-flow validator)
7. `bounded` — gate 7: bounded (boundedness analyzer, section 64)
8. `budgets` — gate 8: budgets (resource budget checker)
9. `contracts` — gate 9: contracts (action contract verifier)
10. `taint` — gate 10: taint (taint/secret checker)
11. `idempotency` — gate 11: idempotency (section 65)
12. `durability` — gate 12: durability (durability checker)
13. `capabilities` — gate 13: capabilities (capability checker)
14. `results` — gate 14: results (result/output validator)
15. `evidence` — gate 15: evidence (observability checker)

## Master doc §63 (line 3148-3166) — VERBATIM gate status table
The "Verification Gate Status" table at lines 3148-3166 enumerates gates 1-15 in order: profile, shape, names, references, expressions, CFG, boundedness, resource budget, action contract, secret/taint, idempotency, durability, capability, output/result, observability.

## crates/vb_cli/src/commands_verify.rs (214 lines)
- Line 70-71: `let mut checks: Vec<&'static str> = Vec::new();`
- Line 73-122: Currently produces 5-6 names: `yaml_parse`, `compilation`, `ir_validation`, `budget_computation`, `boundedness_policy`, `boundedness_policy_check`.
- The `VerifyOk` struct at line 8-17 has `pub checks: Vec<&'static str>`.

# Scope (verified, no fabrication)

Replace the `checks.push(...)` calls in `commands_verify.rs:73-122` so the 15-gate enumeration matches master §63 exactly. Each gate is appended in order; if a gate is not yet enforced, push it as `"<gate_name>:deferred"` (or use a structured enum, but the bead requires minimal change).

The 9 fabricated gate names from the rejected P1-9r (`digest_stability`, `resource_contract_validation`, `error_handler_completeness`, `taint_boundary`, `input_purity`, `expression_complexity`, `cycle_detection`, `determinism_seed`, `replay_round_trip`) are REMOVED entirely.

# Acceptance test

In `crates/vb_cli/src/commands_verify.rs` test module:
```rust
#[test]
fn verify_produces_15_gates_for_full_profile() {
    // Build a valid workflow YAML; call run_verification with VerifyProfile::Full.
    // Assert checks.len() == 15.
    // Assert the 15 names are exactly: profile, shape, names, references,
    //   expressions, CFG, bounded, budgets, contracts, taint, idempotency,
    //   durability, capabilities, results, evidence.
}
```

# Anti-hallucination guards

- DO NOT invent any new gate names. The 15 are FIXED by master §63.
- DO NOT invent hex codes (`0x0E01..0x0F0F`). The master doc does not specify gate codes.
- DO NOT include any of the 9 fabricated names from the rejected bead.

# Kani harness (skipped — verify is cold-path; no arithmetic or hot-path contracts)

The 15-gate enumeration is a static list. No Kani needed. Coverage comes from the unit test that asserts the exact 15 names.

# Dependency

This bead depends on the P0-2r / P0-3r / S-19r / S-20r cleanup (independent verification gates) — but the actual implementation is self-contained.
