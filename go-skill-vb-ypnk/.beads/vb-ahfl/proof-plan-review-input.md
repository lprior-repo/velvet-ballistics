# Proof Plan Review Input: vb-ahfl State 4 Repair After State 3 Scope Repair

## Reviewer Decision Requested

Review the refreshed State 4 proof plan after repaired State 3 resolved `SCOPE-001` for the UI artifact schema parity stack. This is a planning artifact only.

## Approval Boundary

- `SCOPE-001` is resolved for this artifact stack by `.beads/vb-ahfl/delivery-scope.jsonl` and repaired `contract.md` PRE-001.
- Engine YAML-to-IR semantic evidence is excluded. If selected later, regenerate State 2/3/4/5 instead of extending this plan silently.
- Required production-bound obligations remain planned obligations, not proof evidence, until their owner states execute the named commands and record raw output.

## Rejection Context Addressed

- Prior `MANUAL-SCOPE-001`/`BLOCKER-SCOPE-001` language is replaced by `SCOPE-001` with explicit scope-resolution and regeneration semantics.
- Required production-bound Verus/Kani/proptest/API/mutation/fuzz rows name exact downstream targets and commands rather than waiver placeholders.
- `KANI-CANON-001` no longer uses unsupported cargo-kani 0.67.0 flags. The planned command is `cargo kani -p vb_ui_model --tests --harness vb_ahfl_canonicalization_no_false_parity --default-unwind 8`; default Kani safety checks remain enabled unless a later owner explicitly disables them, which this plan does not do.
- Production canonicalization APIs, Kani harness discoverability, and the missing pre-existing include blocker remain routed to State 10 before State 5 can produce raw Kani `SUCCESS` evidence.
- `STATIC-BOUNDARY-001` uses the repaired dependency/import scan and ignores documentation comments that caused the prior State 6 false positive.
- TLA+, Lean/Aeneas/Hax, Loom, Miri, Flux, and dependency-audit lanes are explicit `not_applicable` rows with expiry triggers.
- Prior abstract Verus evidence is not claimed as production-bound proof.

## Discovery Evidence

```bash
pwd -P
test -s ".beads/vb-ahfl/contract.md" && test -s ".beads/vb-ahfl/traceability-matrix.jsonl" && test -s ".beads/vb-ahfl/delivery-scope.jsonl" && test -s ".beads/vb-ahfl/proof-obligations.jsonl" && test -s ".beads/vb-ahfl/verification-layers.md"
rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_ui_model crates/vb_ui_makepad crates/velvet_ballastics velvet-ballistics-MASTER.md
rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_ui_model crates/vb_ui_makepad crates/velvet_ballastics verification velvet-ballistics-MASTER.md
bash -lc 'cargo metadata --format-version 1 --no-deps >/tmp/vb-ahfl-cargo-metadata.json && ! rg -n "^(\\s*(makepad|tokio|async-std|reqwest|hyper|serde_yaml|yaml-rust)\\s*=|\\s*use\\s+(makepad|tokio|async_std|reqwest|hyper|serde_yaml|yaml_rust)\\b|\\s*extern\\s+crate\\s+(makepad|tokio|async_std|reqwest|hyper|serde_yaml|yaml_rust)\\b)" crates/vb_ui_model/Cargo.toml crates/vb_ui_model/src'
```

No discovery command was blocked.

Kani command discovery for this repair:

```bash
TMPDIR="$(pwd -P)/target/tmp" cargo kani --version
TMPDIR="$(pwd -P)/target/tmp" cargo kani --help
```

Observed support: `cargo-kani 0.67.0` supports `--tests`, `--harness`, and `--default-unwind`; it does not support `--bounds-checks` or positive `--overflow-checks`. The State 4 repair is therefore command-contract repair only, not proof execution.

## Planned Obligation Summary

- Required planned proof/test/release obligations: `SCOPE-001`, `VERUS-META-001`, `VERUS-BOUNDS-001`, `VERUS-REDACT-001`, `VERUS-GRAPH-001`, `KANI-CANON-001`, `PROP-PARITY-001`, `STATIC-BOUNDARY-001`, `API-COMPAT-001`, `MUT-ERR-001`, `FUZZ-REDACT-001`, `GATE-CI-001`.
- Not applicable under current repaired static UI schema scope: `WAIVED-TLA-001`, `WAIVED-LEAN-001`, `LOOM-NA-001`, `MIRI-NA-001`, `FLUX-NA-001`, `DEPS-NA-001`.

## Reviewer Checks

```bash
jq -c . .beads/vb-ahfl/proof-obligations.planned.jsonl >/tmp/vb-ahfl-proof-obligations-planned.valid
jq -e -s 'all(.[]; . as $row | ["id","requirement_id","contract_clause","risk","verifier","artifact","command","expected_evidence","assumptions","required","mode","owner_state","rerun_from","status","waiver"] | all(.[]; . as $k | ($row | has($k))))' .beads/vb-ahfl/proof-obligations.planned.jsonl
jq -r 'select(.required == true and ((.command == "not_applicable") or (.command | test("^WAIVER:|^waived$")))) | .id' .beads/vb-ahfl/proof-obligations.planned.jsonl
jq -r 'select(.id == "KANI-CANON-001") | .command' .beads/vb-ahfl/proof-obligations.planned.jsonl
```

The final reviewer check must print exactly `cargo kani -p vb_ui_model --tests --harness vb_ahfl_canonicalization_no_false_parity --default-unwind 8` for `KANI-CANON-001`.
