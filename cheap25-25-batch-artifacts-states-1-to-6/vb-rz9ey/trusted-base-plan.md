# Trusted Base Plan — vb-rz9ey

- bead_id: vb-rz9ey
- state: 4 (proof-planner)
- authored_by: proof-planner
- contract_sha256: e0cafa48f30fc1484731d66b5a300964146d3a1154a85acc3b9bf0d681b6cb66
- proof_obligations_count: 2 (PO-001, PO-002)
- verifier_lane_decisions_count: 14

## 1. Trusted Base Inventory

The trusted base is the union of (a) every `assume`, `axiom`, `admit`,
`external_body`, `#[trusted]`, `#[ignore]`, `opaque`, or `extern_spec` marker
introduced by an obligation, plus (b) every trusted abstraction the obligation
depends on, plus (c) every assumption listed in the obligation's `assumptions`
array. The `trusted_base_refs` field in each `proof-obligation/v1` row must
match an ID in this document.

**Result for vb-rz9ey: ZERO trusted-base entries.**

## 2. Why the Trusted Base Is Empty

This bead has no formal-verification obligations. The two obligations are
build-time and doc-surface verification:

- **PO-001** verifies that `cargo build -p vb_compile --tests` exits 0. There
  is no executable proof code; there are no `assume`, `axiom`, `admit`, or
  `external_body` markers in this obligation. The three entries in the
  obligation's `assumptions` array are *preconditions of the Cargo manifest
  edit being correct* (the test-util feature declaration is unchanged, the
  cfg arms are field-identical, the self-reference syntax matches the
  contract), not trust markers for a proof system. They are discharged by
  `proof-plan-reviewer` at State 4b and by `black-hat-reviewer` at State 8
  via static source review, not by a trust marker in proof code.
- **PO-002** verifies that `cargo build -p vb_cli` and
  `cargo build -p workspace_tests` exit 0 without `test-util` propagating, and
  that `cargo doc -p vb_compile --no-deps` does not surface
  `WorkflowSourceParts`. There is no executable proof code. The four entries
  in the obligation's `assumptions` array are preconditions about the
  downstream Cargo manifests (vb_cli line 8 and workspace_tests line 39 do
  not activate any feature; default = [] is preserved; cargo doc --no-deps
  uses default-features build), again discharged by static review.

Neither obligation requires any trust marker, so neither obligation carries a
`trusted_base_refs` entry, and `proof-obligations.planned.jsonl` row inspection
confirms this:

```
$ jq -r '.id + " trusted_base_refs=" + (.trusted_base_refs | tojson)' .beads/vb-rz9ey/proof-obligations.planned.jsonl
PO-001 trusted_base_refs=[]
PO-002 trusted_base_refs=[]
```

## 3. Cross-Check Against the Anti-Laundering Rules

Per `/home/lewis/.opencode/skill/proof-planner/SKILL.md` "Anti-laundering"
section:

- No `assume` in executable proof code: **N/A** — no executable proof code.
- No `axiom` in executable proof code: **N/A** — no executable proof code.
- No `admit` in executable proof code: **N/A** — no executable proof code.
- No `external_body` in executable proof code: **N/A** — no executable proof
  code.
- No `cover!`-as-proof: **N/A** — no proof harnesses.
- No copied harness models without bridge row: **N/A** — no harness models.
- No generic waivers: **N/A** — `waiver-candidates.jsonl` is empty (zero
  rows).
- No VACUUM Verus proofs: **PASS** — zero Verus obligations are emitted. The
  proof-planner SKILL.md Production Binding Plan mandates a `production_binding`
  field for every Verus obligation; because this bead emits no Verus
  obligations, no production-binding declarations are required and no
  backdoor mechanism can be invoked.

## 4. Production-Binding Plan

The proof-planner SKILL.md Production Binding Plan (MANDATORY for Verus
obligations) is **NOT APPLICABLE** because this bead emits zero Verus
obligations. The closest analogous concept here is the "production-binding
for cargo-build obligations":

| obligation | production symbol targeted | evidence command | bridge |
|------------|----------------------------|------------------|--------|
| PO-001 | `crates::vb_compile::yaml_ast::types::WorkflowSourceParts` (visibility flipped by the cfg gate that the cargo manifest edit activates) | `cargo build -p vb_compile --tests --message-format=human` | Direct: rustc's compile-time check on the symbol's visibility IS the proof evidence. No bridge row needed. |
| PO-002 | `crates::vb_compile::yaml_ast::types::WorkflowSourceParts` (visibility must remain pub(crate) in production builds) | `cargo build -p vb_cli && cargo build -p workspace_tests && cargo doc -p vb_compile --no-deps | grep -c WorkflowSourceParts` | Direct: cargo's build-graph resolution + rustdoc's public-API surface IS the proof evidence. No bridge row needed. |

## 5. Trusted Abstractions Consumed By This Plan

**None.** The plan does not depend on any third-party crate's spec (no
external Verus spec, no Flux spec, no Loom model, no Miri-trusted abstraction,
no cargo-fuzz corpus). It depends only on:

1. The Rust toolchain (cargo, rustc, rustdoc) — version pinned by
   `rust-toolchain.toml` in the workspace root.
2. The proptest harness infrastructure that already exists in
   `crates/vb_compile/tests/proptest_*.rs`.
3. The 9 existing integration test files in `crates/vb_compile/tests/`.

None of these require a trust marker in this plan. They are existing
artifacts whose correctness is established outside this bead (by upstream
proptest maintenance, by Rust toolchain stability, and by prior test
passes).

## 6. Lifecycle

| trusted_base_id | obligation | marker | introduced_in | removed_in | status |
|-----------------|------------|--------|---------------|------------|--------|
| (none) | (none) | (none) | (none) | (none) | n/a |

## 7. Self-Audit Checklist

- [x] Every `proof-obligation/v1` row's `trusted_base_refs` is empty.
- [x] No `assume` / `axiom` / `admit` / `external_body` / `#[trusted]` /
      `#[ignore]` / `opaque` / `extern_spec` markers introduced by this plan.
- [x] No external dependency that would require an `external_dependency_unavoidable`
      waiver.
- [x] No model reduction that would require a `superseded_by_other_lane_with_evidence`
      waiver.
- [x] No diagnostic skip that would require an `#[cfg_attr(..., ignore)]` waiver.
- [x] No third-party crate without an upstream Flux/Verus spec.

The trusted base is empty because the obligations are cargo-build / cargo-doc
invocations against existing production Rust that does not require trust
markers to verify.