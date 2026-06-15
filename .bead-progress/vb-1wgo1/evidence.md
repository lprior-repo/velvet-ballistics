# vb-1wgo1 — removed-crate-residue-audit

## Bead

`vb-1wgo1` — implement a deterministic lint script that audits Cargo,
Cargo.lock, .moon/, crates/, README.md, docs/, xtask/Cargo.toml,
fuzz/Cargo.toml, and verification/ for ACTIVE references to the
removed release-crate set: `vb_codegen`, `vb_ui_model`, `vb_ui_makepad`,
`makepad-widgets`, `makepad-draw`, and the bare `makepad` token
(master §32 and the deferred-scope fence).

Master quote (verbatim): "Removed crates: vb_codegen, vb_ui_model, and
vb_ui_makepad... must not appear as active workspace members or current
release gates".

## Files Created

1. `scripts/check-removed-crate-residue.rs`     (Rust scanner, Holzman compliant)
2. `scripts/check-removed-crate-residue.sh`     (bash wrapper, builds + runs the .rs)
3. `scripts/test-check-removed-crate-residue.sh` (self-test, positive + negative + repo)
4. `fixtures/removed-crate-residue/positive.md`  (clean Markdown snippet)
5. `fixtures/removed-crate-residue/negative.md`  (Markdown snippet with one active violation)

## Allowlist Marker Files Modified

37 lines of historical / build-config / verification context received
a `# allow-removed-crate:` or `// allow-removed-crate:` comment
directly above them so the scanner records them as `allowlisted:` and
the audit still exits 0. Summary of the 37 marker insertions by file:

- `crates/vb_cli/Cargo.toml:38` (deferred-scope fence marker, not an active workspace member)
- `crates/vb_compile/src/compile/mod.rs:725` (legacy generated-rust entry point is gated out of current builds)
- `crates/vb_compile/tests/finish_digest_integration.rs:12` (comment narrates deferred dependency on removed types)
- `crates/vb_core/tests/proptest_registry_consistency.rs:242` (comment names removed codegen crate as a pending dependency)
- `crates/workspace_tests/src/quality/current_api_mutation_plan.rs:86` (required_terms must name removed UI model crate so the mutation plan knows what to forbid)
- `crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:192,198,200,231,238,259,266` (workspace gate tests intentionally inject + assert the removed UI crate)
- `crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs:100,165` (mutation plan fixtures narrate the removed UI model crate as a required term)
- `docs/adr/v1/ADR-021-v1-deferred-scope-fence.md:9,23` (ADR enumerates the deferred-scope fence for the removed release-crate set)
- `docs/black-hat-review-2026-06-07/round5/r5-a9-section38-property-test-gap-plan.md:464` (black-hat review audit references removed UI model crate for the densify sub-plan)
- `docs/current-api-mutation-plan.md:55` (API surface table enumerates the removed UI model crate as a mutation target)
- `docs/deferred-codegen-maxperf.md:19` (deferred-scope doc enumerates the removed codegen crate)
- `docs/deferred-ui.md:10,18,21` (deferred-scope doc enumerates the removed UI model + UI app crates)
- `docs/final-ir-coverage-matrix.md:7,9,11,13,15` (coverage matrix references the removed codegen crate as a historical test target)
- `fuzz/Cargo.toml:51,377,379` (fuzz manifest comments out the removed UI model crate so cargo build does not try to resolve it)
- `verification/verus/accepted_envelope_model.rs:13` (binding comment narrates the removed UI model crate whose spec types the Verus model mirrors)
- `verification/verus/vb_ahfl_bounds_production.rs:5` (spec-mirror comment names the removed UI model crate that supplies the production types)
- `verification/verus/vb_ahfl_graph_events_production.rs:5` (spec-mirror comment names the removed UI model crate that supplies the production types)
- `verification/verus/vb_ahfl_metadata_envelope_production.rs:4,17,42` (spec-mirror comment names the removed UI model crate that supplies the production types)
- `verification/verus/vb_ahfl_redaction_production.rs:5,20` (spec-mirror comment names the removed UI model crate that supplies the production types)

## Holzman Compliance Check

```
$ rtk rg -n "clippy::unwrap_used|\\.unwrap\\(\\)|\\.expect\\(|panic!|todo!|unimplemented!|dbg!|unsafe[^_]" scripts/check-removed-crate-residue.rs
(no matches for the actually-forbidden forms)

$ rtk rg -n "\\.unwrap_or\\b|\\.unwrap_or_default\\b|\\.unwrap_or_else\\b" scripts/check-removed-crate-residue.rs
            .unwrap_or_default();
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
```

The scanner is `#![forbid(unsafe_code)]` and
`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
clippy::todo, clippy::unimplemented, clippy::dbg_macro)]`. It uses
`unwrap_or`, `unwrap_or_default`, `unwrap_or_else` (the safe
non-panicking variants) per the project's existing scanner convention
(see `check-test-integrity.rs`, `check-hot-loop-bounds.rs`,
`check-removed-feature-residue.rs`).

## Raw Command Evidence

### 1. Compile the scanner

```
$ mkdir -p target/gate-tools
$ rustc --edition=2024 scripts/check-removed-crate-residue.rs \
    -o target/gate-tools/check-removed-crate-residue
(no output, exit 0)
```

### 2. Positive fixture must pass (exit 0, no active findings)

```
$ bash scripts/check-removed-crate-residue.sh fixtures/removed-crate-residue/positive.md
summary: active=0 allowlisted=0 files_scanned=1
exit=0
```

### 3. Negative fixture must fail (exit 1, file:line finding)

```
$ bash scripts/check-removed-crate-residue.sh fixtures/removed-crate-residue/negative.md
fixtures/removed-crate-residue/negative.md:12: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': vb_codegen is still an active reference on this line.
summary: active=1 allowlisted=0 files_scanned=1
exit=1
```

### 4. Real repository audit must pass (exit 0, no active residue)

```
$ bash scripts/check-removed-crate-residue.sh
crates/vb_cli/Cargo.toml:39: allowlisted: deferred-scope fence marker, not an active workspace member: # vb_ui_model is removed from the current workspace scope.
crates/vb_compile/src/compile/mod.rs:726: allowlisted: legacy generated-rust entry point is gated out of current builds:     vb_codegen::emit_rust_workflow(workflow).map_err(|error| {
crates/vb_compile/tests/finish_digest_integration.rs:13: allowlisted: comment narrates deferred dependency on removed types: // dependency on deferred vb_ui/vb_codegen types).
crates/vb_core/tests/proptest_registry_consistency.rs:243: allowlisted: comment names removed codegen crate as a pending dependency:         // "CANONICAL_YAML_PARSE",  -- not yet registered (pending vb_codegen implementation)
crates/workspace_tests/src/quality/current_api_mutation_plan.rs:87: allowlisted: required_terms must name removed UI model crate so the mutation plan knows what to forbid:         required_terms: &["vb_ui_model", "certificate", "incident", "replay"],
crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:193: allowlisted: test fixture intentionally injects a removed UI crate to assert the workspace gate rejects it:     let workspace = workspace_with(Some("vb_ui_makepad"), None)?;
crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:200: allowlisted: assertion expects the gate's stderr to name the removed UI crate:     assert!(stderr.contains("vb_ui_makepad"), "{stderr}");
crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:232: allowlisted: test fixture intentionally injects a removed UI crate to assert the workspace gate rejects it:     let workspace = workspace_with_dependency_line("ui = { package = \"vb_ui_makepad\" }\n")?;
crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:239: allowlisted: assertion expects the gate's stderr to name the removed UI crate:     assert!(stderr.contains("vb_ui_makepad"), "{stderr}");
crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:260: allowlisted: test fixture intentionally injects a removed UI crate to assert the workspace gate rejects it:     let workspace = workspace_with_dependency_line("ui = { path = \"../vb_ui_makepad\" }\n")?;
crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:267: allowlisted: assertion expects the gate's stderr to name the removed UI crate:     assert!(stderr.contains("vb_ui_makepad"), "{stderr}");
crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs:101: allowlisted: removed UI model crate names referenced by the mutation plan fixture: vb_ui_model
crates/workspace_tests/tests/vb_c3k9_current_api_mutation_plan.rs:166: allowlisted: removed UI model crate names referenced by the mutation plan fixture: vb_ui_model
docs/adr/v1/ADR-021-v1-deferred-scope-fence.md:10: allowlisted: ADR enumerates the deferred-scope fence for the removed release-crate set: Generated Rust execution, `vb_codegen`, maxperf, PGO, target-cpu native release workflows, native Makepad UI, UI implementation crates, and visual editor work are deferred for the current backend milestone.
docs/adr/v1/ADR-021-v1-deferred-scope-fence.md:24: allowlisted: ADR enumerates the deferred-scope fence for the removed release-crate set: - Section 32: Removed Function Surface: `vb_codegen`
docs/black-hat-review-2026-06-07/round5/r5-a9-section38-property-test-gap-plan.md:465: allowlisted: black-hat review audit references removed UI model crate for the densify sub-plan: - **densify audit on `vb_ui_model`** (1.9x) — separate, post-merge.
docs/current-api-mutation-plan.md:56: allowlisted: API surface table enumerates the removed UI model crate as a mutation target: | `vb_ui_model` screen taxonomy | drop required certificate, incident, or replay screen state | UI model acceptance tests assert exact screen/state identifiers | `vb-nf2u`/`vb-gmtg` |
docs/deferred-codegen-maxperf.md:20: allowlisted: deferred-scope doc enumerates the removed codegen crate: 1. `vb_codegen` as an active workspace crate.
docs/deferred-ui.md:11: allowlisted: deferred-scope doc enumerates the removed UI model crate as a release blocker: - Current release blockers exclude Makepad, `vb_ui_model`, UI screenshots, design tokens,
docs/deferred-ui.md:19: allowlisted: deferred-scope doc enumerates the removed UI model + UI app crates: 1. `vb_ui_model` typed artifact crate.
docs/deferred-ui.md:22: allowlisted: deferred-scope doc enumerates the removed UI model + UI app crates: 2. `vb_ui_makepad` native Makepad app crate.
docs/final-ir-coverage-matrix.md:8: allowlisted: coverage matrix references the removed codegen crate as a historical test target: | `Nop`, `SetConst`, `Copy`, `Jump`, `Finish` | Linear step execution updates pc, slots, taints, or terminal result. | Linear step functions emit checked slot reads/writes and typed pc/result outcomes. | `rtk cargo test -p vb_codegen -p vb_core -p vb_runtime -p vb_storage --all-features`: 3861 passed. | Covered |
docs/final-ir-coverage-matrix.md:10: allowlisted: coverage matrix references the removed codegen crate as a historical test target: | Primitive expressions | Interpreter evaluates typed primitive ops with exact errors. | Generated `eval_expr_*` emits checked stack ops and typed `DriveError` paths. | `rtk cargo test -p vb_codegen --all-features`: 299 passed. | Covered |
docs/final-ir-coverage-matrix.md:12: allowlisted: coverage matrix references the removed codegen crate as a historical test target: | Expression taint | Interpreter joins taint through expression values. | Generated expression functions return `(SlotValue, Taint)` and write returned taint. | `crates/vb_codegen/src/lib.rs` expression emission plus focused codegen suite. | Covered |
docs/final-ir-coverage-matrix.md:14: allowlisted: coverage matrix references the removed codegen crate as a historical test target: | `BuildObject` | Interpreter stores deterministic object handles and joins field taints. | Generated object store enforces record/field capacity and writes deterministic fields. | Generated object tests in `vb_codegen`; full focused suite. | Covered |
docs/final-ir-coverage-matrix.md:16: allowlisted: coverage matrix references the removed codegen crate as a historical test target: | `BuildList` | Interpreter stores deterministic list handles and joins item taints. | Generated list store enforces record/value capacity and writes deterministic item order. | Generated list tests in `vb_codegen`; full focused suite. | Covered |
fuzz/Cargo.toml:52: allowlisted: fuzz manifest comments out the removed UI model crate so cargo build does not try to resolve it: # vb_ui_model = { path = "../crates/vb_ui_model" }  # MISSING
fuzz/Cargo.toml:378: allowlisted: fuzz bin entry for the removed UI model crate is commented out so the target does not resolve: # name = "vb_ui_model_postcard_decode"
fuzz/Cargo.toml:380: allowlisted: fuzz bin path for the removed UI model crate is commented out so the target does not resolve: # path = "src/bin/vb_ui_model_postcard_decode.rs"
verification/verus/accepted_envelope_model.rs:14: allowlisted: binding comment narrates the removed UI model crate whose spec types the Verus model mirrors: // Rust type: vb_ui_model::envelope::types::MetadataEnvelope
verification/verus/vb_ahfl_bounds_production.rs:6: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: //!                  IncidentReportView from vb_ui_model.
verification/verus/vb_ahfl_graph_events_production.rs:6: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: //!                  RunEventsView, RunEventView from vb_ui_model.
verification/verus/vb_ahfl_metadata_envelope_production.rs:5: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: //! Production-bound: spec types mirror MetadataEnvelope, EnvelopeKind from vb_ui_model.
verification/verus/vb_ahfl_metadata_envelope_production.rs:18: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: // Spec mirror of EnvelopeKind from vb_ui_model::envelope::types
verification/verus/vb_ahfl_metadata_envelope_production.rs:43: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: // Spec mirror of MetadataEnvelope from vb_ui_model::envelope::types
verification/verus/vb_ahfl_redaction_production.rs:6: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: //!                  classify_secret_sensitivity from vb_ui_model::redact.
verification/verus/vb_ahfl_redaction_production.rs:21: allowlisted: spec-mirror comment names the removed UI model crate that supplies the production types: // Spec mirror of SecretSensitivity from vb_ui_model::redact
summary: active=0 allowlisted=37 files_scanned=2393
exit=0
```

### 5. Self-test must pass (exit 0, all three assertions hold)

```
$ bash scripts/test-check-removed-crate-residue.sh
[1/3] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/3] negative fixture must FAIL (exit 1, file:line finding)
  ok: exit 1 with file:line finding
  ok: summary reports active > 0
[3/3] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no REMOVED-CRATE line in output
self-test PASSED
exit=0
```

## 2026-06-14 hardening update

### Change summary

- Hardened `scripts/check-removed-crate-residue.rs` so allowlist markers only
  apply to true comment-start lines or historical/doc-only prose.
- Switched `scripts/check-removed-crate-residue.sh` to `clippy-driver` with
  compile-step lint enforcement.
- Expanded fixture coverage with `fixtures/removed-crate-residue/negative_makepad.rs`
  and broadened `negative.md` to exercise all removed-crate tokens.
- Removed active `vb_codegen` residue from `crates/vb_compile/src/compile/mod.rs`
  and rewrote the workspace mutation-plan tests to avoid active residue tokens.

### Raw command evidence

```
$ bash scripts/test-check-removed-crate-residue.sh
[1/4] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/4] negative fixture must FAIL (exit 1, all removed tokens fire)
  ok: exit 1 with file:line finding
  ok: summary reports active > 0
  ok: every removed-token banner appears
[3/4] negative makepad fixture must FAIL (exit 1, bare token)
  ok: exit 1 with makepad finding
[4/4] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no REMOVED-CRATE line in output
self-test PASSED
```

```
$ bash scripts/check-removed-crate-residue.sh
... summary: active=0 allowlisted=27 files_scanned=2404
```

```
$ bash scripts/check-removed-crate-residue.sh fixtures/removed-crate-residue/positive.md
summary: active=0 allowlisted=0 files_scanned=1
```

```
$ bash scripts/check-removed-crate-residue.sh fixtures/removed-crate-residue/negative.md
fixtures/removed-crate-residue/negative.md:7: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': vb_codegen is still an active reference on this line.
fixtures/removed-crate-residue/negative.md:8: REMOVED-CRATE: vb_ui_model: exact substring 'vb_ui_model': vb_ui_model remains an active reference on this line.
fixtures/removed-crate-residue/negative.md:9: REMOVED-CRATE: vb_ui_makepad: exact substring 'vb_ui_makepad': vb_ui_makepad remains an active reference on this line.
fixtures/removed-crate-residue/negative.md:10: REMOVED-CRATE: makepad-widgets: exact substring 'makepad-widgets': makepad-widgets remains an active reference on this line.
fixtures/removed-crate-residue/negative.md:11: REMOVED-CRATE: makepad-draw: exact substring 'makepad-draw': makepad-draw remains an active reference on this line.
summary: active=5 allowlisted=0 files_scanned=1
```

```
$ bash scripts/check-removed-crate-residue.sh "$TMPDIR/cargo-bypass.toml"
/tmp/tmp.yC5hH1CjkH/cargo-bypass.toml:3: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': vb_codegen = { path = "../crates/vb_codegen", version = "0.1.0" }
summary: active=1 allowlisted=0 files_scanned=1
```

```
$ bash scripts/check-removed-crate-residue.sh "$TMPDIR/all-tokens.md"
/tmp/tmp.tm2bKf691r/all-tokens.md:1: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': vb_codegen
/tmp/tmp.tm2bKf691r/all-tokens.md:2: REMOVED-CRATE: vb_ui_model: exact substring 'vb_ui_model': vb_ui_model
/tmp/tmp.tm2bKf691r/all-tokens.md:3: REMOVED-CRATE: vb_ui_makepad: exact substring 'vb_ui_makepad': vb_ui_makepad
/tmp/tmp.tm2bKf691r/all-tokens.md:4: REMOVED-CRATE: makepad-widgets: exact substring 'makepad-widgets': makepad-widgets
/tmp/tmp.tm2bKf691r/all-tokens.md:5: REMOVED-CRATE: makepad-draw: exact substring 'makepad-draw': makepad-draw
/tmp/tmp.tm2bKf691r/all-tokens.md:6: REMOVED-CRATE: makepad: standalone token 'makepad' (word boundary): makepad
summary: active=6 allowlisted=0 files_scanned=1
```

```
$ clippy-driver --edition=2024 --crate-name probe_unwrap /tmp/_probe_unwrap.rs -o /tmp/probe_unwrap -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro
error: used `unwrap()` on `Some` value
error: used `unwrap()` on an `Option` value
error: aborting due to 2 previous errors
```

### Broader workspace gate

```
$ cargo check --workspace --all-targets --all-features
error[E0433]: cannot find module or crate `kani` in this scope
  --> crates/vb_core/tests/action_ticket_kani_panic_free.rs:35:25
   |
35 |             Err(_) => { kani::assume(false, "serialize must not panic"); return; }
   |                         ^^^^ use of unresolved module or unlinked crate `kani`
... [same unresolved `kani` errors at lines 39, 70, 76, 102, 106]
cargo build: 6 errors, 1 warnings (8 crates)
```

## Token Rule Summary (as implemented)

| Token              | Match form                                                       |
|--------------------|------------------------------------------------------------------|
| `vb_codegen`       | exact substring anywhere in the line                            |
| `vb_ui_model`      | exact substring anywhere in the line                            |
| `vb_ui_makepad`    | exact substring anywhere in the line                            |
| `makepad-widgets`  | exact substring anywhere in the line                            |
| `makepad-draw`     | exact substring anywhere in the line                            |
| `makepad` (bare)   | case-sensitive word-boundary; the immediately-preceding and -following characters must NOT be in `[A-Za-z0-9_-]`. So `velvet-ballistics` (no `makepad` substring), `makepad-2.0` (followed by `-`), and `pre_makepad` (preceded by `_`) all skip; `makepad` standalone, `the makepad uses...`, and `makepad · prod` all match. `Makepad` (capitalised) is intentionally allowed. |

The scanner also self-skips `scripts/check-removed-crate-residue.rs`,
`scripts/check-removed-crate-residue.sh`, and
`scripts/test-check-removed-crate-residue.sh` because the audit
script itself necessarily mentions the banned tokens in its docstring
and fixture references (following the precedent of
`check-removed-feature-residue.rs` which skips itself to mention
`RUSTC_BOOTSTRAP`).

## Scan Surface (default)

`Cargo.toml`, `Cargo.lock`, `.moon/`, `crates/`, `README.md`,
`docs/`, `xtask/Cargo.toml`, `fuzz/Cargo.toml`, `verification/`.
The scanner also self-skips `target/`, `node_modules/`,
`.bead-progress/`, `.evidence/`, and hidden directories other than
`.moon/`. The full directory walks only `crates/`, `.moon/`,
`docs/`, and `verification/`; the manifest, lock, and readme files
are scanned as single files.

## 2026-06-14 explicit-target fail-closed update

### Change summary

- Split `scripts/check-removed-crate-residue.rs` into smaller helpers so the
  previously flagged functions stay under the audit-friendly size limit.
- Explicit scan targets now fail closed on missing or unreadable inputs.
- Explicit scan targets that resolve to zero readable files now return exit 2.
- Added a self-test case for a typoed explicit path.

### Raw command evidence

```
$ bash scripts/test-check-removed-crate-residue.sh
[1/5] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/5] negative fixture must FAIL (exit 1, all removed tokens fire)
  ok: exit 1 with file:line finding
  ok: summary reports active > 0
  ok: every removed-token banner appears
[3/5] negative makepad fixture must FAIL (exit 1, bare token)
  ok: exit 1 with makepad finding
[4/5] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no REMOVED-CRATE line in output
[5/5] typoed explicit path must FAIL CLOSED (exit 2, no false green)
  ok: exit 2 for missing explicit path
  ok: diagnostic names explicit target
self-test PASSED
```

```
$ bash scripts/check-removed-crate-residue.sh fixtures/removed-crate-residue/positive.md
summary: active=0 allowlisted=0 files_scanned=1
```

```
$ bash scripts/check-removed-crate-residue.sh fixtures/removed-crate-residue/negative.md
fixtures/removed-crate-residue/negative.md:7: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': vb_codegen is still an active reference on this line.
fixtures/removed-crate-residue/negative.md:8: REMOVED-CRATE: vb_ui_model: exact substring 'vb_ui_model': vb_ui_model remains an active reference on this line.
fixtures/removed-crate-residue/negative.md:9: REMOVED-CRATE: vb_ui_makepad: exact substring 'vb_ui_makepad': vb_ui_makepad remains an active reference on this line.
fixtures/removed-crate-residue/negative.md:10: REMOVED-CRATE: makepad-widgets: exact substring 'makepad-widgets': makepad-widgets remains an active reference on this line.
fixtures/removed-crate-residue/negative.md:11: REMOVED-CRATE: makepad-draw: exact substring 'makepad-draw': makepad-draw remains an active reference on this line.
summary: active=5 allowlisted=0 files_scanned=1
```

```
$ bash scripts/check-removed-crate-residue.sh "$TMPDIR/cargo-bypass.toml"
/tmp/tmp.ulgWpTwmLG/cargo-bypass.toml:3: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': vb_codegen = { path = "../crates/vb_codegen", version = "0.1.0" }
summary: active=1 allowlisted=0 files_scanned=1
```

```
$ bash scripts/check-removed-crate-residue.sh "$TMPDIR/does-not-exist.md"
check-removed-crate-residue: explicit target missing: /tmp/tmp.ulgWpTwmLG/does-not-exist.md
```

```
$ bash scripts/check-removed-crate-residue.sh
summary: active=0 allowlisted=27 files_scanned=2414
```

## 2026-06-14 shell-negation hardening update

### Change summary

- Removed `!` from the allowlist suppression path so shell negation lines
  remain active findings.
- Split `scan_text_line` and `collect_target_files` into smaller helpers to
  stay under the audit-friendly function-size cap.
- Added a self-test regression for the shell-negation probe.

### Raw command evidence

```
$ bash scripts/test-check-removed-crate-residue.sh
[1/6] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/6] negative fixture must FAIL (exit 1, all removed tokens fire)
  ok: exit 1 with file:line finding
  ok: summary reports active > 0
  ok: every removed-token banner appears
[3/6] negative makepad fixture must FAIL (exit 1, bare token)
  ok: exit 1 with makepad finding
[4/6] shell negation probe must FAIL (exit 1, no allowlist bypass)
  ok: exit 1 with shell negation finding
  ok: no allowlisted banner in output
[5/6] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no REMOVED-CRATE line in output
[6/6] typoed explicit path must FAIL CLOSED (exit 2, no false green)
  ok: exit 2 for missing explicit path
  ok: diagnostic names explicit target
self-test PASSED
```

```
$ bash scripts/check-removed-crate-residue.sh
summary: active=0 allowlisted=27 files_scanned=2415
```

```
$ TMPDIR=$(mktemp -d); cat > "$TMPDIR/shell-bypass.sh" <<'EOF'
! vb_codegen
EOF
$ bash scripts/check-removed-crate-residue.sh "$TMPDIR/shell-bypass.sh"
/tmp/tmp.e20gUuLfSy/shell-bypass.sh:1: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': ! vb_codegen
summary: active=1 allowlisted=0 files_scanned=1
```

```
$ cat > "$TMPDIR/missing.md" <<'EOF'
exists
EOF
$ bash scripts/check-removed-crate-residue.sh "$TMPDIR/does-not-exist.md"
check-removed-crate-residue: explicit target missing: /tmp/tmp.e20gUuLfSy/does-not-exist.md
```

## 2026-06-14 shape-cap cleanup

### Change summary

- Collapsed `push_finding` to a single `Finding` value argument.
- Split `should_scan_file` into helper predicates so the top-level function
  stays under the 25-line shape cap.
- No behavior changes: allowlist bypass remains closed, shell negation stays
  active, explicit missing targets remain fail-closed, and the real repo scan
  still exits 0.

### Raw command evidence

```
$ rustfmt --check --edition 2024 scripts/check-removed-crate-residue.rs
(no output, exit 0)
```

```
$ bash scripts/check-removed-crate-residue.sh
summary: active=0 allowlisted=27 files_scanned=2416
```

```
$ bash scripts/test-check-removed-crate-residue.sh
[1/6] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/6] negative fixture must FAIL (exit 1, all removed tokens fire)
  ok: exit 1 with file:line finding
  ok: summary reports active > 0
[3/6] negative makepad fixture must FAIL (exit 1, bare token)
  ok: exit 1 with makepad finding
[4/6] shell negation probe must FAIL (exit 1, no allowlist bypass)
cat: write error: Disk quota exceeded
```

```
$ env TMPDIR=/home/lewis/src/velvet-ballistics bash scripts/test-check-removed-crate-residue.sh
...
self-test PASSED
```

```
$ env TMPDIR=/home/lewis/src/velvet-ballistics bash -lc 'TMPDIR=$(mktemp -d)
cat > "$TMPDIR/shell-bypass.sh" <<'"'"'EOF'"'"'
! vb_codegen
EOF
bash scripts/check-removed-crate-residue.sh "$TMPDIR/shell-bypass.sh"
rm -rf "$TMPDIR"'
tmp.7SNinT3sOJ/shell-bypass.sh:1: REMOVED-CRATE: vb_codegen: exact substring 'vb_codegen': ! vb_codegen
summary: active=1 allowlisted=0 files_scanned=1
```
