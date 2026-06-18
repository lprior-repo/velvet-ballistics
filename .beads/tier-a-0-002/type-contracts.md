# Type Contracts — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.000000+00:00

## 1. Type-Contract Strategy

The rust-contract agent does not write the scanner; it writes the
*pseudocode types* the State 11 holzman-rust implementation must
satisfy. The pseudocode is intentionally close to Rust syntax so the
implementation is a near-mechanical translation, with two exceptions:

- String-based policy fields are *enums* (not strings) in the
  pseudocode. The Rust implementation must use enums and reject
  `from_str` parsers. This is the legal-state-unrepresentable boundary.
- The aggregate root's `decide()` method is the only producer of
  `GateDecision`. A `match` over `GateDecision` is total; there is no
  `_ => ...` arm.

The pseudocode is a *contract*, not a *sketch*. Any divergence between
the pseudocode and the State 11 implementation is a contract violation
that the State 13 black-hat-reviewer must reject.

## 2. Aggregate Pseudocode

```
// scripts/forbid-runtime-fmt.rs (contract surface; full implementation in State 11)

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
        clippy::todo, clippy::unimplemented, clippy::dbg_macro)]

pub struct ResidueQuarantine {
    policy: ResiduePolicy,
    source_root: SourceRoot,
    allowlist: AllowlistRef,
    report: ScanReport,
    state: ResidueQuarantineState,
}

impl ResidueQuarantine {
    pub fn run(policy: ResiduePolicy,
               source_root: SourceRoot,
               allowlist_path: &Path) -> Result<GateDecision, GateError> {
        let mut q = Self::init(policy, source_root, allowlist_path)?;
        q.walk()?;
        q.match_lines()?;
        q.diff_against_allowlist()?;
        Ok(q.decide())
    }

    fn init(policy: ResiduePolicy,
            source_root: SourceRoot,
            allowlist_path: &Path) -> Result<Self, GateError> { ... }
    fn walk(&mut self) -> Result<(), GateError> { ... }
    fn match_lines(&mut self) -> Result<(), GateError> { ... }
    fn diff_against_allowlist(&mut self) -> Result<(), GateError> { ... }
    fn decide(self) -> GateDecision { ... }
}
```

`ResidueQuarantine::run` is the only public entry point. There is no
`pub fn new`; the aggregate is constructible only through the parser
at the policy / source / allowlist boundary.

## 3. `ResiduePolicy` Pseudocode

```
pub struct ResiduePolicy {
    pub forbidden: Vec<ForbiddenImport>,
    pub quarantined_crates: BTreeMap<HotCrateName, QuarantinedCrate>,
    pub cold_markers: BTreeSet<ColdMarker>,
    pub master_ref: MasterRef,
}
```

`ResiduePolicy` is a value object. It is constructible only through
`ResiduePolicy::from_master(master_path: &Path) -> Result<ResiduePolicy, GateError>`.
The parser walks the master document and constructs the closed sets.
It is the *only* way to add a new forbidden import: a master
amendment plus a code change to the parser. The closed-set invariant
is preserved by the type system.

### 3.1 Mapping to `scripts/forbid-runtime-fmt.sh` and sibling gates

`ResiduePolicy::forbidden` is the runtime data structure that the
scanner iterates over to find line matches. The hard-coded list inside
`scripts/forbid-runtime-fmt.rs` is a *parsed mirror* of the master
document; drift between the mirror and the master is caught by the
State 11 holzman-rust implementation's test that asserts
`ResiduePolicy::from_master` produces a policy with the expected
closed set.

| Sibling gate | Closed forbidden set | Coverage of new gate's policy |
|--------------|----------------------|--------------------------------|
| `check-removed-crate-residue` | `vb_codegen`, `vb_ui_model`, `vb_ui_makepad`, `makepad-widgets`, `makepad-draw`, bare `makepad` | Out of scope. The new gate covers runtime deps; the sibling covers removed UI/codegen crates. |
| `check-removed-feature-residue` | `target-cpu=native`, `pgo` (in active contexts), `maxperf` (feature), `generated` (feature) | Out of scope. The new gate covers runtime deps; the sibling covers removed release features. |
| `check-hot-cold-forbidden-apis` | `serde_json`, `serde_yaml`, `HashMap<String, _>`, `unbounded_channel`, etc. | **Overlapping.** The new gate's pattern set is a strict subset of the sibling's `FORMAT-JSON-001` / `FORMAT-YAML-001` / `MAP-STRING-001` / `CHANNEL-UNBOUNDED-001` class IDs, restricted to the four hot crates. The new gate is the *narrowed* re-statement with explicit forbidden crate names. |

The narrowing is intentional: the sibling `check-hot-cold-forbidden-apis`
covers a wider set of forbidden APIs across a wider set of crates
(via cold-marker logic) and uses class IDs as the failure language.
The new gate uses forbidden *import names* as the failure language,
which is the form the master document (§2 line 99-102, §12 line 419)
actually uses.

## 4. `ScanDecision` and `GateDecision` Pseudocode

```
pub enum GateDecision {
    Pass,
    Fail(Vec<ResidueMatch>),     // active residue (non-empty)
    InvalidInvocation(String),   // contract violation
}

pub type ScanDecision = GateDecision;  // legacy alias, used by tests
```

The `ScanDecision` alias is kept because the test fixture convention
in this repository (see `scripts/test-check-removed-feature-residue.sh`
assertion style) uses the term "scan decision" when reading the gate's
exit code. The contract binds both names to the same type.

The two error variants are *not* part of `GateDecision`'s normal
success path; they are the *contract violation* path and the gate
script translates them to exit code 2 with the attached `String`
emitted on stderr.

## 5. `ResidueMatch` Pseudocode

```
pub struct ResidueMatch {
    pub file: SourcePath,                  // "crates/<hot_crate>/src/<...>/<file>.rs"
    pub line_no: u32,                      // 1-indexed; matches editor line numbers
    pub forbidden: ForbiddenImport,
    pub snippet: TextSnippet,              // trimmed to 120 chars
    pub channel_kind: Option<ChannelKind>,  // Some(_) for channel patterns
}

pub enum ChannelKind { Bounded, Unbounded }
```

### 5.1 Smart constructor

```
impl ResidueMatch {
    pub fn new(file: SourcePath,
               line_no: u32,
               forbidden: ForbiddenImport,
               snippet: TextSnippet,
               channel_kind: Option<ChannelKind>) -> Self {
        Self { file, line_no, forbidden, snippet, channel_kind }
    }
}
```

The smart constructor is `pub` because the scanner and the test
fixtures both need it. The fields are `pub` to allow `derive(Debug,
Clone, PartialEq, Eq, Hash)]` for test assertions. There is no
`ResidueMatch::forbidden_setter`; the field is set only at construction.

### 5.2 Display impl

```
impl fmt::Display for ResidueMatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: RUNTIME-FMT: {}: {}",
               self.file, self.line_no, self.forbidden.name.as_str(), self.snippet)
    }
}
```

The `Display` impl is the canonical stderr line. Tests assert against
this format. The format is a *contract*: changes to the format are
breaking changes to `test-forbid-runtime-fmt.sh`.

## 6. `ForbiddenImport` Pseudocode

```
pub struct ForbiddenImport {
    pub name: ForbiddenImportName,
    pub kind: ForbiddenImportKind,
    pub master_ref: MasterRef,
}

pub enum ForbiddenImportName {
    SerdeJson,
    SerdeYaml,
    Hyper,
    Reqwest,
    Axum,
    HashMapStringGeneric,
    TokioSyncMpscUnbounded,
}

impl ForbiddenImportName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SerdeJson              => "serde_json",
            Self::SerdeYaml              => "serde_yaml",
            Self::Hyper                  => "hyper",
            Self::Reqwest                => "reqwest",
            Self::Axum                   => "axum",
            Self::HashMapStringGeneric   => "HashMap<String,_>",
            Self::TokioSyncMpscUnbounded => "tokio::sync::mpsc::unbounded",
        }
    }

    pub fn as_pattern(&self) -> &'static str {
        // Substring used by the line classifier. Some entries have a
        // shorter pattern than the as_str() form (e.g. serde_json vs
        // serde_json::Value) to catch variant imports and qualified paths.
        match self {
            Self::SerdeJson              => "serde_json",
            Self::SerdeYaml              => "serde_yaml",
            Self::Hyper                  => "hyper",
            Self::Reqwest                => "reqwest",
            Self::Axum                   => "axum",
            Self::HashMapStringGeneric   => "HashMap<String,",
            Self::TokioSyncMpscUnbounded => "tokio::sync::mpsc::unbounded",
        }
    }
}

pub enum ForbiddenImportKind {
    CrateName,        // serde_json, serde_yaml, hyper, reqwest, axum, tokio
    PathToken,        // tokio::sync::mpsc::unbounded
    TypeExpression,   // HashMap<String, _>
}

pub struct MasterRef {
    pub section: u32,  // 2, 12, 43, ...
    pub line: u32,     // 99, 419, 2038, ...
}
```

### 6.1 Variant-import coverage

The pattern table is chosen so that the following forbidden
constructions all fail:

| Construction | Matched by |
|--------------|------------|
| `use serde_json;` | `SerdeJson` |
| `serde_json::Value` | `SerdeJson` |
| `serde_json::from_str(...)` | `SerdeJson` |
| `serde_json::to_string(...)` | `SerdeJson` |
| `use serde_yaml;` | `SerdeYaml` |
| `use hyper;` | `Hyper` |
| `use hyper::server::...;` | `Hyper` |
| `use reqwest;` | `Reqwest` |
| `use axum;` | `Axum` |
| `let m: HashMap<String, _> = HashMap::new();` | `HashMapStringGeneric` |
| `tokio::sync::mpsc::unbounded_channel()` | `TokioSyncMpscUnbounded` |
| `tokio::sync::mpsc::unbounded()` | `TokioSyncMpscUnbounded` |
| `use tokio::sync::mpsc::unbounded;` | `TokioSyncMpscUnbounded` |

The pattern table is enumerated in `hazard-analysis.md` §5.1.

## 7. `QuarantinedCrate` and `HotCrateName` Pseudocode

```
pub struct QuarantinedCrate {
    pub crate_name: HotCrateName,
    pub path_glob: PathGlob,                  // "crates/<name>/src/**/*.rs"
    pub cold_markers: BTreeSet<ColdMarker>,
}

pub enum HotCrateName { VbCore, VbRuntime, VbStorage, VbIpc }

impl HotCrateName {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VbCore    => "vb_core",
            Self::VbRuntime => "vb_runtime",
            Self::VbStorage => "vb_storage",
            Self::VbIpC     => "vb_ipc",
        }
    }
}
```

The set of `QuarantinedCrate` values is a closed `BTreeMap<HotCrateName,
QuarantinedCrate>`. The map is exhaustively matched in
`ResidueQuarantine::walk`; adding a new crate requires both a master
amendment and a code change.

## 8. `ColdMarker` Pseudocode

```
pub enum ColdMarker {
    Diagnostic,
    Diagnostics,
    Fixture,
    Fixtures,
    Harness,
    Kani,
    Loom,
    Proof,
    Property,
    Proptest,
    Proptests,
    Support,
    TestUtil,
    Tests,
    Verification,
}

impl ColdMarker {
    pub fn as_str(&self) -> &'static str {
        match self { ... }
    }
}

pub fn is_cold_path(path: &Path) -> bool {
    let mut components = path.components().peekable();
    while let Some(c) = components.next() {
        if let std::path::Component::Normal(s) = c {
            let s = s.to_string_lossy();
            for marker in COLD_MARKERS {
                if s.contains(marker) {
                    return true;
                }
            }
        }
    }
    false
}
```

The set of `ColdMarker` variants is closed. The string forms are
borrowed from the sibling `scripts/check-hot-cold-forbidden-apis.rs`
(lines 7-23). Any change requires both a master amendment and a code
change in both scanners.

## 9. `AllowlistRef` Pseudocode

```
pub struct AllowlistRef {
    pub path: AllowlistPath,                          // scripts/forbid-runtime-fmt.allow
    pub entries: BTreeMap<AllowlistKey, AllowlistEntry>,
}

pub struct AllowlistKey {
    pub file: SourcePath,
    pub line_no: u32,
    pub forbidden: ForbiddenImportName,
}

pub struct AllowlistEntry {
    pub owner: String,
    pub reviewed_by: String,
    pub test: String,
    pub reason: String,
}

impl AllowlistRef {
    pub fn load(path: &Path) -> Result<AllowlistRef, GateError> {
        // read the file; on read failure -> GateError::GlobUnreadable
        // for each line: parse the pipe-separated format
        // on parse failure -> GateError::AllowlistParseFailure
        // deduplicate by key; on duplicate -> GateError::AllowlistParseFailure
    }

    pub fn lookup(&self, key: &AllowlistKey) -> Option<&AllowlistEntry> {
        self.entries.get(key)
    }
}
```

### 9.1 Allowlist file format

```
# scripts/forbid-runtime-fmt.allow
# Format: <file_path>|<line_no>|<forbidden_name>|<owner>|<reviewed_by>|<test>|<reason>
# Lines starting with '#' are comments.
crates/vb_core/src/action/tests.rs|42|serde_json|alice|bob|proptest_serde_roundtrip|dev-dep test only
crates/vb_core/tests/proptest_serde_roundtrip.rs|17|serde_json|alice|bob|proptest_serde_roundtrip|dev-dep test only
```

The file may be empty (header comments only). An empty allowlist is
the default and means "no exceptions".

## 10. `ScanReport` and `MasterRef` Pseudocode

```
pub struct ScanReport {
    pub files_scanned: u32,
    pub hot_paths_count: u32,
    pub cold_paths_count: u32,
    pub total_matches: u32,
    pub allowlisted_matches: u32,
    pub active_matches: u32,
    pub active: Vec<ResidueMatch>,
    pub allowlisted: Vec<(ResidueMatch, AllowlistEntry)>,
}

pub struct MasterRef {
    pub section: u32,
    pub line: u32,
}

pub type SourcePath = String;       // relative to repo root
pub type SourceRoot = PathBuf;      // absolute path to repo root
pub type PathGlob = String;         // glob pattern
pub type TextSnippet = String;      // trimmed line text
pub type AllowlistPath = PathBuf;   // absolute path to .allow file
```

## 11. `GateError` Pseudocode

```
pub enum GateError {
    PatternFileMissing(String),       // ForbiddenImport name -> master line
    GlobUnreadable { path: String, os_error: String },
    AllowlistParseFailure { line: u32, reason: String },
    ScriptInvocationFailure(String),  // catch-all for uncaught panics
    NewResidueDetected,               // sentinel for the gate's happy-path failure
}

impl GateError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::NewResidueDetected => 1,           // active residue found
            Self::PatternFileMissing(_) => 2,        // contract violation
            Self::GlobUnreadable { .. } => 2,        // contract violation
            Self::AllowlistParseFailure { .. } => 2, // contract violation
            Self::ScriptInvocationFailure(_) => 2,   // contract violation
        }
    }

    pub fn stderr_template(&self) -> String {
        match self {
            Self::NewResidueDetected =>
                "summary: active={N} allowlisted={M} files_scanned={K} hot_paths={H} cold_paths={C}".to_owned(),
            Self::PatternFileMissing(name) =>
                format!("GateError:PatternFileMissing: {name}"),
            Self::GlobUnreadable { path, os_error } =>
                format!("GateError:GlobUnreadable: {path}: {os_error}"),
            Self::AllowlistParseFailure { line, reason } =>
                format!("GateError:AllowlistParseFailure: line {line}: {reason}"),
            Self::ScriptInvocationFailure(reason) =>
                format!("GateError:ScriptInvocationFailure: {reason}"),
        }
    }
}
```

The error contract is exhaustive: every error variant has a defined
exit code and a defined stderr template. See `error-taxonomy.md` for
the full mapping and the stderr/stdout policy.

## 12. Mapping to Sibling Gate Patterns

The pseudocode above is a near-translation of the sibling gates' shape:

| Concept | Sibling gate | New gate |
|---------|--------------|----------|
| Bash wrapper | `scripts/check-removed-feature-residue.sh` (45 lines) | `scripts/forbid-runtime-fmt.sh` (modeled on sibling) |
| Scanner compiled with | `rustc --edition=2024` (45 lines) | `rustc --edition=2024` (preferred) or `clippy-driver` |
| `findings` accumulator | `Vec<Finding>` | `Vec<ResidueMatch>` (this contract) |
| Allowlist format | `# allow-removed-feature:` / `// allow-removed-feature:` per-line markers | `scripts/forbid-runtime-fmt.allow` file (modeled on `scripts/hot-cold-forbidden-apis.allow`) |
| Stderr line format | `<path>:<lineno>: REMOVED-FEATURE: <token>: <line>` | `<file>:<line_no>: RUNTIME-FMT: <forbidden_name>: <snippet>` (this contract) |
| Final summary | `summary: active=N allowlisted=M files_scanned=K` | `summary: active=N allowlisted=M files_scanned=K hot_paths=H cold_paths=C` |
| Cold-marker set | (not used) | `COLD_MARKERS` (borrowed from `check-hot-cold-forbidden-apis.rs::COLD_MARKERS`) |
| Exit codes | 0 = pass, 1 = active residue, 64 = invalid invocation | 0 = pass, 1 = active residue, 2 = contract violation |

The differences are intentional and document why the new gate is
narrower and more spec-bound than the existing siblings.

## 13. Contract Tests (State 9)

The three test names from the bead description map to the following
contract-level tests, which the State 9 test-writer will implement
in `scripts/test-forbid-runtime-fmt.sh`:

1. `test_quarantine_gate_blocks_json_import` — runs the gate against
   a fixture containing `use serde_json;` and asserts (a) exit code 1,
   (b) a stderr line of the form
   `<path>:<line_no>: RUNTIME-FMT: serde_json: use serde_json;`,
   (c) a `summary:` line with `active>=1`.

2. `test_quarantine_gate_blocks_unbounded_channel` — runs the gate
   against a fixture containing `tokio::sync::mpsc::unbounded_channel()`
   and asserts (a) exit code 1, (b) a stderr line of the form
   `<path>:<line_no>: RUNTIME-FMT: tokio::sync::mpsc::unbounded:
   let _c = tokio::sync::mpsc::unbounded_channel();`,
   (c) a `summary:` line with `active>=1`.

3. `test_moon_ci_quarantine_dependency_correctly_ordered` — runs the
   gate against the real moon task graph and asserts (a) the
   `forbid-runtime-fmt` task exists, (b) the `check` task's `deps:`
   array contains the gate, (c) the gate appears before the heavier
   `cargo check` invocations in the dep array.

Each test maps to a `match` arm in the State 11 implementation's
`test-forbid-runtime-fmt.sh` exit-on-failure logic. The tests are
failing-first: the implementation must satisfy them before the
State 11 holzman-rust agent can mark State 11 complete.
