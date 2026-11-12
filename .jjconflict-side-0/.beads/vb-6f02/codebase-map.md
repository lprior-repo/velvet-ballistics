# Codebase Map - vb-6f02: Add contracts-as-data suite

## Workspace Topology

```
velvet-ballistics/
├── Cargo.toml                    # Workspace root (19 crates)
├── .moon/tasks/all.yml           # 32 moon CI tasks (see below)
├── .beads/schemas/               # 79 CUE validation schemas (bead pipeline output)
├── contracts/
│   ├── invariants.yaml           # 12.3K - Machine-readable mechanical invariants
│   ├── proof_obligations.yaml    # 27.0K - Proof obligation metadata
│   └── verus/
│       └── vb_qi37_16_5_lifecycle_journal_storage.rs  # Verus contract sketch
├── crates/
│   ├── vb_core/                  # Hot in-memory execution core
│   ├── vb_yaml/                  # Cold-path YAML parsing (saphyr wrapper)
│   ├── vb_validate/              # Cold-path workflow validation (schema, gates, type/taint)
│   ├── vb_cli/                   # CLI binary (argparse, commands, envelopes)
│   ├── vb_ui_model/              # Typed view models (re-exports vb_core primitives)
│   ├── vb_ui_snapshot/           # UI snapshot capture + layout checks
│   ├── vb_proof_kernels/         # Proof kernel types
│   ├── vb_storage/               # Storage layer
│   ├── vb_runtime/               # Runtime
│   ├── vb_expr/                  # Expression parsing
│   ├── vb_compile/               # Compilation layer
│   ├── vb_codegen/               # Code generation
│   ├── vb_ipc/                   # IPC
│   ├── vb_ui_makepad/            # Makepad GUI
│   ├── vb_ui/                    # UI layer
│   ├── vb_doc/                   # Documentation
│   ├── vb_benchmark/             # Benchmarks
│   └── workspace_tests/          # Cross-crate integration tests
├── xtask/
│   ├── src/lib.rs                # Public API: evidence, evidence_gate, command_family,
│   │                               # dependency_boundary, parser, registry, routing, status
│   ├── src/main.rs               # CLI entry: clap commands → handlers
│   └── src/evidence/             # ★ 16 modules: contracts-as-data infrastructure
│       ├── release_contract.rs   # GateEvidence, WhyFailed, GateStatus, run_gate/run_profile
│       ├── release_validation.rs # Validation logic
│       ├── release_validators.rs # Validator implementations
│       ├── release_rendering.rs  # Evidence rendering/output
│       ├── release_model.rs      # Release model types
│       ├── tooling_and_gate_types.rs  # AI_FAST_GATES, AI_RELEASE_GATES, etc.
│       ├── fixture_parsers.rs    # Fixture parsing
│       ├── negative_fixtures.rs  # Negative test fixtures
│       ├── artifact_facts.rs     # Artifact fact tracking
│       ├── error_profile_domain.rs # Error profile types
│       ├── persistence.rs        # Evidence persistence
│       ├── parsed_documents.rs   # Document parsing
│       ├── raw_documents.rs      # Raw document handling
│       ├── tests.rs              # Evidence module tests
│       └── evidence_gate.rs      # BenchmarkEvidence, AuditResult, EvidenceBundle
├── fuzz/                         # Fuzz targets (currently has build error)
└── scripts/
    ├── check-agent-cli-contract.sh  # Contract literal presence checker
    ├── check-beads-server-mode.sh
    ├── check-nightly-features.sh
    ├── check-source-length.sh
    ├── check-workspace-assertions.sh
    └── check-workspace-assertions.py
```

## Key Files and Roles

### 1. contracts/as-data Infrastructure (Existing)

| File | Size | Role |
|------|------|------|
| `contracts/invariants.yaml` | 12.3K | Machine-readable mechanical invariants. Defines rules with `id`, `applies_to` (glob paths), `forbidden` (symbol lists), `description`. Checked by xtask forbidden-scan. |
| `contracts/proof_obligations.yaml` | 27.0K | Proof obligation metadata loaded by `xtask proof-plan` |
| `contracts/verus/*.rs` | ~1K | Verus verification contract sketches |

**Pattern:** YAML files define rules → xtask reads them → reports pass/fail. This is the existing contracts-as-data model.

### 2. xtask Evidence Modules (Existing - Foundation Layer)

| File | Size | Role | Risk |
|------|------|------|------|
| `xtask/src/evidence/release_contract.rs` | 7.3K | GateEvidence, WhyFailed, GateStatus types + orchestration (run_gate, run_profile). Defines gate categories, screens, layout checks, redaction classes. | **LOW** - Pure data types, well-tested |
| `xtask/src/evidence/release_validation.rs` | 8.1K | Validation logic for evidence | LOW |
| `xtask/src/evidence/release_validators.rs` | 9.2K | Validator implementations | LOW |
| `xtask/src/evidence/release_rendering.rs` | 10.0K | Evidence rendering/output | LOW |
| `xtask/src/evidence/release_model.rs` | 6.9K | Release model types | LOW |
| `xtask/src/evidence/tooling_and_gate_types.rs` | 9.5K | Gate type definitions: AI_FAST_GATES (fmt, check, clippy, nextest, forbidden-scan, hotpath-scan), AI_RELEASE_GATES (check, test, supply-chain, miri, fuzz-smoke, coverage, mutants-smoke, bench-build, feature-powerset, source-length, maxperf), REQUIRED_UI_SUBGATES, REQUIRED_LAYOUT_CHECKS | **MEDIUM** - Central definition of all gate categories |
| `xtask/src/evidence/fixture_parsers.rs` | 10.6K | Fixture parsing | LOW |
| `xtask/src/evidence/negative_fixtures.rs` | 11.0K | Negative test fixtures | LOW |
| `xtask/src/evidence/artifact_facts.rs` | 8.6K | Artifact fact tracking | LOW |
| `xtask/src/evidence/error_profile_domain.rs` | 7.1K | Error profile types | LOW |
| `xtask/src/evidence/persistence.rs` | 4.9K | Evidence persistence (file I/O) | LOW |
| `xtask/src/evidence/parsed_documents.rs` | 7.5K | Document parsing | LOW |
| `xtask/src/evidence/raw_documents.rs` | 8.0K | Raw document handling | LOW |
| `xtask/src/evidence/tests.rs` | 1.7K | Evidence module tests | LOW |
| `xtask/src/evidence_gate.rs` | 17.9K | BenchmarkEvidence, AuditResult, EvidenceBundle types | LOW |
| `xtask/src/evidence.rs` | 640B | Module entry point (include! of all 16 files) | LOW |

### 3. xtask Enforcement Infrastructure (Existing)

| File | Role |
|------|------|
| `xtask/src/forbidden_scan.rs` | Scans Rust crates for forbidden patterns (unwrap, panic, unsafe, todo, unimplemented). Uses `FORBIDDEN_PATTERNS` constant. |
| `xtask/src/gates.rs` | Gate orchestration (8.8K) |
| `xtask/src/proof.rs` | Proof obligation loading from `contracts/proof_obligations.yaml`. Defines ProofObligation, RequiredProof, FuzzField structs. |
| `xtask/src/evidence_gate.rs` | Evidence bundle types for supply-chain, API, semver, bloat, performance |

### 4. vb_validate Crate (Core Validation Engine)

| File | Size | Role |
|------|------|------|
| `crates/vb_validate/src/schema.rs` | 71.1K | Schema validation - primary validation logic |
| `crates/vb_validate/src/gates.rs` | 101.7K | Gate implementations |
| `crates/vb_validate/src/control_flow.rs` | 24.7K | Control flow validation |
| `crates/vb_validate/src/references.rs` | 24.7K | Reference validation (RefTables) |
| `crates/vb_validate/src/type_taint.rs` | 17.4K | Type/taint analysis |
| `crates/vb_validate/src/diagnostic.rs` | 30.6K | Diagnostic rendering |
| `crates/vb_validate/src/idempotency_contract.rs` | 8.9K | Idempotency contract verification |
| `crates/vb_validate/src/lib.rs` | 9.3K | Crate entry - exposes `references`, `schema`, `gates`, `type_taint` as public API |
| `crates/vb_validate/src/schema_tests.rs` | 40.3K | Schema tests |
| `crates/vb_validate/src/gate_tests.rs` | 27.1K | Gate tests |

**Public API:** `references` (RefTables, validate_single_reference), `schema` (schema validation), `gates` (all gate implementations), `type_taint` (type/taint analysis), `diagnostic` (diagnostic types/rendering).

### 5. vb_cli Crate (CLI - Needs Contract Envelopes)

| File | Size | Role |
|------|------|------|
| `crates/vb_cli/src/cli_envelope.rs` | 10.1K | CLI envelope types |
| `crates/vb_cli/src/agent_context.rs` | 9.7K | Agent context management |
| `crates/vb_cli/src/args.rs` | 57.5K | Argument parsing (clap derive) |
| `crates/vb_cli/src/commands_ai_context.rs` | 24.1K | AI context command |
| `crates/vb_cli/src/commands_diff.rs` | 12.2K | Diff command |
| `crates/vb_cli/src/commands_journal.rs` | 10.3K | Journal command |
| `crates/vb_cli/src/commands_verify.rs` | 4.9K | Verify command |
| `crates/vb_cli/src/commands_workflow.rs` | 13.5K | Workflow command |
| `crates/vb_cli/src/args/` | - | Subcommand argument definitions |

### 6. Moon CI Tasks (32 tasks total)

| Task | Type | Dependencies | Notes |
|------|------|-------------|-------|
| `beads-server-mode` | script | - | Checks .beads/server-mode config |
| `workspace-assertions` | script | - | Workspace structure checks |
| `agent-cli-contract` | script | - | Checks CLI for required literals |
| `lint-src` | script | - | Clippy with -Dwarnings |
| `nightly-feature-gate` | command | - | Rust nightly feature gate check |
| `fmt` | command | - | rustfmt --check |
| `source-length` | command | workspace-assertions | Source length checks |
| `check` | script | nightly-feature-gate, agent-cli-contract, beads-server-mode | cargo check --all-targets |
| `test` | script | check | cargo nextest run |
| `miri` | script | - | Miri UB checks (3 targeted tests) |
| `fuzz-smoke` | script | - | **FAILING** - build error |
| `coverage` | script | check | llvm-cov smoke |
| `hardened-build` | script | test, lint-src, source-length | Hardened profile build |
| `supply-chain` | script | - | cargo audit + deny + vet + geiger + machete |
| `feature-powerset` | script | check | cargo hack check --feature-powerset |
| `mutants-smoke` | script | check | cargo-mutants smoke on vb_core |
| `bench-build` | script | - | Benchmark build |
| `doc-test` | script | test | cargo test --doc |
| `doc` | script | test | cargo doc |
| `maxperf` | script | - | Max performance build |
| `maxperf-native` | script | - | Max perf native build |
| `nightly-feature-cargo-probe` | script | check | No-op (check does the probe) |

**NOTABLE GAP:** No moon task for scanning `contracts/` directory, validating contract files, or checking schema_version/kind invariants.

## Dependency Graph

```
vb_yaml ──→ (saphyr-parser)
    ↓
vb_core ──→ vb_proof_kernels
    ↓
vb_ui_model ──→ vb_core (re-exports: ActionContract, Capability, ActionId, etc.)
    ↓
vb_ui_snapshot ──→ vb_ui_model (via layout_kernel types)
    ↓
vb_validate ──→ (standalone - cold path)
    ↓
vb_cli ──→ (args, commands, envelope)
    ↓
xtask ──→ vb_ui_snapshot (for chip_is_readable, overlap_area_px, etc.)
           vb_yaml (for proof obligation loading via serde_yaml)
```

## Public API Surfaces That Need Contracts

### Must Have Contracts (per bead acceptance tests):

1. **CLI Envelopes** - `crates/vb_cli/src/cli_envelope.rs` and `args/` directory
   - Contract file: `contracts/cli_envelope.cue` (or .yaml)
   - Validates: schema_version, kind, required fields, exit codes

2. **UI Tokens** - `crates/vb_ui_model/src/envelope/`, `crates/vb_ui_model/src/ai.rs`
   - Contract file: `contracts/ui_tokens.cue`
   - Validates: token structure, redaction, required fields

3. **Accepted Artifacts** - `crates/vb_core/src/action/`, `crates/vb_core/src/frame/`
   - Contract file: `contracts/accepted_artifacts.cue`
   - Validates: action structure, idempotency, side-effect classification

4. **Evidence Bundles** - `xtask/src/evidence/release_contract.rs`
   - Contract file: `contracts/evidence_bundle.cue`
   - Validates: GateEvidence structure, required fields, schema_version, kind

5. **Diagnostics** - `crates/vb_validate/src/diagnostic.rs`
   - Contract file: `contracts/diagnostics.cue`
   - Validates: diagnostic codes, rendering structure

6. **Gate Outputs** - `xtask/src/evidence/release_validation.rs`, `xtask/src/gates.rs`
   - Contract file: `contracts/gate_outputs.cue`
   - Validates: gate result structure, pass/fail semantics

### Key Risk Areas

| Area | Risk | Reason |
|------|------|--------|
| `xtask/src/evidence/tooling_and_gate_types.rs` | MEDIUM | Central definition of all gate categories - changing affects moon task definitions |
| `contracts/invariants.yaml` | MEDIUM | Existing contracts-as-data model - new contracts must be compatible |
| `crates/vb_validate/src/lib.rs` | LOW | Existing validation engine - new contracts may need to integrate |
| `crates/vb_cli/src/args.rs` (57.5K) | LOW | Large file but not modified by this bead |
| `moon/tasks/all.yml` | LOW | New moon task for contract-discovery |
| `xtask/src/main.rs` | LOW | New xtask subcommand for contract validation |

### CUE Schema Format (Reference)

From `.beads/schemas/*.cue`:
- Package: `validation`
- Imports: `list`
- Root type: `#BeadImplementation`
- Required fields: `schema_version`, `kind` (enforced by invariant)
- Boolean truth types: `bool & true`
- List constraints: `[...string] & list.MinItems(N)`
- String typed fields: `string` (not untyped)
