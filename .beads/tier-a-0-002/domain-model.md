# Domain Model — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.000000+00:00

## 1. Ubiquitous Language

The vocabulary the rust-contract agent commits to the rest of the pipeline.
All subsequent artifacts in this bead, the State 9 test-writer, the State 11
holzman-rust implementation, and the State 14 evidence-packaging must use
these terms exactly. Drift is a contract violation.

| Term | Definition |
|------|------------|
| **forbidden import** | A crate-name or path token whose presence in a non-test `.rs` source file inside a quarantined crate is an unconditional active residue. The closed set is `serde_json`, `serde_yaml`, `hyper`, `reqwest`, `axum`, `HashMap<String, _>`, and `tokio::sync::mpsc::unbounded`. The set is closed because master §2 (lines 99-102) and §12 (lines 405-439) enumerate it canonically. |
| **hot crate** | A first-party crate whose `src/` directory is enumerated by the gate's input glob. The closed set is `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`. The set is closed because master §44.6 (line 2078) names these four crates as the runtime core where JSON and HTTP are absent. |
| **residue match** | A single line of source text, identified by `(file_path, line_no, snippet)`, that the scanner classified as containing a forbidden import. Each residue match is a candidate failure unless covered by an allowlist entry. |
| **active residue** | A residue match that is NOT covered by an allowlist entry. Active residue is a hard failure. |
| **allowlisted residue** | A residue match that IS covered by an allowlist entry that names the exact `(file_path, line_no, token, reason)` tuple. Allowlisted residue is informational and never fails the gate. |
| **cold marker** | A substring appearing in a `crates/<hot_crate>/` path that exempts the file from the gate. The closed set is `diagnostic`, `diagnostics`, `fixture`, `fixtures`, `harness`, `kani`, `loom`, `proof`, `property`, `proptest`, `proptests`, `support`, `test_util`, `tests`, `verification`. The set is copied from the sibling `check-hot-cold-forbidden-apis.rs` (lines 7-23) and is closed for this bead. |
| **scan evidence** | The full set of residue matches produced by one execution of the gate over one fixed source tree, plus the per-file classification metadata. |
| **gate decision** | The single boolean outcome of one gate execution: `Pass` iff active residue == 0. Encoded as the shell exit code (0 = Pass, 1 = Fail, 2 = InvalidInvocation). |
| **dependency correctness** | The property that `.moon/tasks/all.yml::check` (or `.moon.yml::pipeline`) declares the new gate as a transitive dependency that runs *before* the heavier compile gates. Witnessed by `test_moon_ci_quarantine_dependency_correctly_ordered`. |
| **cold path** | A source file path that contains at least one cold marker. The gate skips cold paths. |
| **hot path** | A source file path that contains zero cold markers. The gate scans every line of every hot path. |
| **policy entity** | An item in the closed forbidden-import set. Each policy entity is a small value object whose name and kind are derived from the master document and never change without a master amendment. |

## 2. Bounded Context

The Residue Quarantine bounded context is the gate's only world. It is
deliberately small and does not share state with any other gate or with
the runtime core's domain models.

**Name:** `residue-quarantine`

**Owns:**

- The closed set of forbidden imports (policy entities).
- The closed set of hot crates.
- The closed set of cold markers.
- The allowlist file format and its parsing.
- The scan-and-decide workflow.
- The stderr evidence format and exit-code mapping.

**Does NOT own:**

- Cargo.toml dependency tables (out of scope per OQ-004; covered by
  `cargo-deny check advisories` and `cargo-vet` advisory paths instead).
- The runtime core's domain types (ActionId, SymbolId, WorkflowId, etc.).
- The Fjall persistence model or the Action ABI IPC model.
- Cargo.lock (transitive entries are out of scope per OQ-006).
- `Cargo.lock` line entries for `serde_json` / `serde_yaml` that appear
  only because of cold-crate dev-deps.

**Upstream contract:** `velvet-ballistics-MASTER.md` §2 (lines 82-104),
§12 (lines 405-439), §43 (lines 2027-2065). The master document is the
single source of truth for the closed sets; the gate's pattern table
must be regenerated from the master document when the master changes.

**Downstream consumers:**

- `moon run :forbid-runtime-fmt` (and `moon ci` transitively) calls the
  gate as a fail-closed moon task.
- `scripts/test-forbid-runtime-fmt.sh` (State 9 test-writer) consumes
  the gate's stdout/stderr to assert pass/fail.
- `evidence-packaging` State 14 cites the gate's exit code as raw
  evidence for master §2/§12/§43/§44.6/§78 acceptance criteria.

## 3. Aggregate: `ResidueQuarantine`

A single aggregate root governs one gate execution. The aggregate is
constructed once per `bash scripts/forbid-runtime-fmt.sh` invocation,
mutated in-memory as the scanner walks the file tree, and reduced to
a `GateDecision` at the end.

### 3.1 Aggregate identity

The aggregate is identified by:

- `policy_id` (immutable, derived from `velvet-ballistics-MASTER.md`
  revision hash at scan start; re-derivation is a contract violation
  if the master changes mid-scan).
- `source_tree_root` (immutable; `pwd -P` at scan start).
- `quarantined_crates` (immutable, derived from policy_id).
- `allowlist` (immutable for the duration of one scan; mutating the
  allowlist mid-scan is invalid).

### 3.2 Aggregate invariants

- The forbidden-import set, hot-crate set, and cold-marker set are
  closed and immutable. They are derived from the master document and
  re-derivable by inspection of the source.
- The allowlist is append-only within one scan. An allowlist entry
  is matched to exactly one `(file_path, line_no, token)` tuple.
- The aggregate is in exactly one of the following states at every
  observable instant: `Initialized`, `Walking`, `Matching`,
  `Differencing`, `Decided`. See `workflow-model.md` for transitions.
- `GateDecision` is total: every gate execution produces exactly one
  decision. There is no "indeterminate" decision. A scanner crash
  produces a `Fail` decision with the crash message attached to the
  stderr.

### 3.3 Aggregate commands

| Command | Pre-state | Post-state | Effect |
|---------|-----------|------------|--------|
| `init(policy, source_root, allowlist)` | (none) | `Initialized` | Build the immutable policy view, validate the allowlist file is readable, exit 2 on failure. |
| `walk()` | `Initialized` | `Walking` | Enumerate every `.rs` file under each `quarantined_crates` entry, partition into `hot_paths` and `cold_paths`. |
| `match_lines()` | `Walking` | `Matching` | For each `hot_paths` file, read text, classify each non-blank line, emit zero or more `ResidueMatch` values. Cold paths are skipped. |
| `diff_against_allowlist()` | `Matching` | `Differencing` | For each `ResidueMatch`, look up an allowlist entry by `(file_path, line_no, token)`; mark matched entries as `Allowlisted`, unmatched as `Active`. |
| `decide()` | `Differencing` | `Decided` | Reduce to `GateDecision::Pass` iff `active_count == 0`, else `GateDecision::Fail` with the active matches attached. |

### 3.4 Aggregate events

- `WalkingCompleted { files_scanned, hot_paths_count, cold_paths_count }`
- `MatchingCompleted { total_matches, allowlisted_matches, active_matches }`
- `Decided { decision, evidence }`

The events are in-memory only; the aggregate does not persist events
to disk. The stderr evidence is the only durable artifact.

## 4. Value Objects

Value objects are immutable, equality-by-value, and constructed only
through parsers or smart constructors. They have no identity.

### 4.1 `ForbiddenImport`

The closed set of policy entities derived from master §2 and §12.
Backed by an enum (not a string) so the type system rejects drift.

```
struct ForbiddenImport {
    name: ForbiddenImportName,  // enum, see §4.1.1
    kind: ForbiddenImportKind,  // enum, see §4.1.2
    master_ref: MasterRef,      // { section: u32, line: u32 }
}

enum ForbiddenImportName {
    SerdeJson,
    SerdeYaml,
    Hyper,
    Reqwest,
    Axum,
    HashMapStringGeneric,        // "HashMap<String, _>"
    TokioSyncMpscUnbounded,      // "tokio::sync::mpsc::unbounded"
}

enum ForbiddenImportKind {
    CrateName,           // serde_json, serde_yaml, hyper, reqwest, axum, tokio
    PathToken,           // tokio::sync::mpsc::unbounded
    TypeExpression,      // HashMap<String, _>
}
```

A `ForbiddenImport` is constructible only from a `ForbiddenImportName`
enumeration. There is no `ForbiddenImport::from_str` parser; adding
a new forbidden import requires a master amendment and a code change
to the enumeration. This is the legal-state-unrepresentable boundary
for the gate's policy.

### 4.2 `QuarantinedCrate`

A glob pattern for one of the four hot crates. Immutable.

```
struct QuarantinedCrate {
    crate_name: HotCrateName,         // enum
    path_glob: PathGlob,              // "crates/<name>/src/**/*.rs"
    cold_markers: Vec<ColdMarker>,    // see §4.3
}

enum HotCrateName { VbCore, VbRuntime, VbStorage, VbIpc }
```

The four `QuarantinedCrate` values are constructed once at process
start and live in a `BTreeMap<HotCrateName, QuarantinedCrate>` indexed
by the `HotCrateName` enum (so a duplicate path is unrepresentable).

### 4.3 `ColdMarker`

A substring exempting a path from the gate. The closed set is borrowed
from `scripts/check-hot-cold-forbidden-apis.rs::COLD_MARKERS` (lines
7-23) and is immutable.

```
enum ColdMarker {
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
```

A path is a `cold_path` iff at least one of its `/`-separated
components contains (as a substring) the string form of a
`ColdMarker`. The classifier is `is_cold_path(path) -> bool`.

### 4.4 `ChannelKind`

Encodes whether a found channel-construction pattern is bounded or
unbounded. Used by the type signature of the scanner's match report
to make the *unbounded* nature a first-class concept.

```
enum ChannelKind {
    Bounded,
    Unbounded,
}
```

### 4.5 `ResidueMatch`

A single detected forbidden-import occurrence. The aggregate owns a
`Vec<ResidueMatch>` and reduces it to a decision.

```
struct ResidueMatch {
    file: SourcePath,                 // crates/<hot_crate>/src/.../file.rs
    line_no: u32,                     // 1-indexed
    forbidden: ForbiddenImport,       // which policy entity matched
    snippet: TextSnippet,             // the matching line, trimmed
    channel_kind: Option<ChannelKind>, // Some(.) for channel patterns
}
```

A `ResidueMatch` is constructed only by the scanner's `classify_line`
function, which guarantees that `forbidden.name` and the substring
that fired the match are consistent. The constructor is a smart
constructor; the field initializers are `pub(crate)`.

### 4.6 `AllowlistRef`

A reference to the allowlist file plus its parsed entries. The file
is `scripts/forbid-runtime-fmt.allow` and the format is one entry
per line:

```
<file_path>|<line_no>|<forbidden_name>|<owner>|<reviewed_by>|<test>|<reason>
```

Fields are pipe-separated; the `forbidden_name` field is one of the
strings in `ForbiddenImportName`'s `as_str()` form. A malformed line
fails the gate with `GateError::AllowlistParseFailure` (exit 2).

```
struct AllowlistRef {
    path: AllowlistPath,                 // scripts/forbid-runtime-fmt.allow
    entries: BTreeMap<AllowlistKey, AllowlistEntry>,
}

struct AllowlistKey {
    file: SourcePath,
    line_no: u32,
    forbidden: ForbiddenImportName,
}

struct AllowlistEntry {
    owner: String,
    reviewed_by: String,
    test: String,
    reason: String,
}
```

`AllowlistRef::load(path) -> Result<AllowlistRef, GateError>` is the
only constructor. It is the boundary parser; it cannot construct an
allowlist from an unparseable source.

### 4.7 `ScanReport`

The aggregate's final report. Produced by the `Decided` transition
and emitted to stderr.

```
struct ScanReport {
    files_scanned: u32,
    hot_paths_count: u32,
    cold_paths_count: u32,
    total_matches: u32,
    allowlisted_matches: u32,
    active_matches: u32,
    active: Vec<ResidueMatch>,
    allowlisted: Vec<(ResidueMatch, AllowlistEntry)>,
}
```

### 4.8 `GateDecision`

The aggregate's reduced outcome. Total: every gate execution produces
exactly one.

```
enum GateDecision {
    Pass,
    Fail(Vec<ResidueMatch>),     // active residue
    InvalidInvocation(String),   // contract violation: pattern file missing,
                                 // glob unreadable, allowlist unparseable, etc.
}
```

`GateDecision` is the return type of `ResidueQuarantine::decide()` and
the single source of truth for the script's exit code.

## 5. Repositories (Interface Only)

`rust-contract` does not implement repositories. The interfaces below
are the seams the State 11 holzman-rust implementation must satisfy.

### 5.1 `PolicyRepository`

- `load(master_ref: MasterRef) -> ForbiddenImportSet`
- `load_quarantined_crates() -> BTreeMap<HotCrateName, QuarantinedCrate>`
- `load_cold_markers() -> BTreeSet<ColdMarker>`

These three methods are pure and deterministic. They read from the
master document (or a checked-in mirror) and the source tree.

### 5.2 `SourceTreeWalker`

- `walk(roots: &[QuarantinedCrate]) -> Result<(Vec<HotPath>, Vec<ColdPath>), GateError>`

Deterministic walk of `.rs` files. The walker is total: every
existing file under each root is classified; missing roots produce
`Ok(([], []))` rather than an error.

### 5.3 `LineMatcher`

- `classify_line(rel_path: &str, line_no: u32, raw_line: &str) -> Vec<ResidueMatch>`

Pure function over a single line. Side-effect-free. Idempotent.

### 5.4 `AllowlistRepository`

- `load(path: &Path) -> Result<AllowlistRef, GateError>`
- `lookup(allowlist: &AllowlistRef, key: &AllowlistKey) -> Option<AllowlistEntry>`

Pure read-only. The lookup is O(log N) via the BTreeMap.

## 6. Domain Events (Stderr Evidence Contract)

Each event has a single-line stderr format that `test-forbid-runtime-fmt.sh`
asserts against. The format is a domain contract, not an implementation
detail, and is committed by this document.

### 6.1 Active residue (failure)

```
<file>:<line_no>: RUNTIME-FMT: <forbidden_name>: <snippet>
```

### 6.2 Allowlisted residue (informational)

```
<file>:<line_no>: allowlisted: <reason>: <snippet>
```

### 6.3 Final summary

```
summary: active=<N> allowlisted=<M> files_scanned=<K> hot_paths=<H> cold_paths=<C>
```

The summary line is the only stdout line the gate emits on a successful
run. On a failed run, summary is on stderr with all residue-match lines
above it.

### 6.4 Contract violations (exit 2)

```
GateError:PatternFileMissing: <path>
GateError:GlobUnreadable: <path>: <os_error>
GateError:AllowlistParseFailure: <line>: <reason>
GateError:ScriptInvocationFailure: <reason>
```

## 7. Mapping to the Master Document

| Master section | Lines | Domain element |
|----------------|-------|----------------|
| §2 "No JSON in the runtime core" | 99 | `ForbiddenImportName::SerdeJson` |
| §2 "No HTTP in the runtime core" | 100 | `ForbiddenImportName::{Hyper, Reqwest, Axum}` |
| §2 "No `HashMap<String, Value>` runtime state" | 102 | `ForbiddenImportName::HashMapStringGeneric` |
| §2 "No unbounded queues, ... task spawning" | 97 | `ForbiddenImportName::TokioSyncMpscUnbounded` |
| §12 `serde_json` | 419 | `ForbiddenImportName::SerdeJson` (cross-reference) |
| §12 `HashMap<String, _>` | 411 | `ForbiddenImportName::HashMapStringGeneric` (cross-reference) |
| §12 `unbounded channel creation` | 427 | `ForbiddenImportName::TokioSyncMpscUnbounded` (cross-reference) |
| §12 `YAML parser calls` | 421 | `ForbiddenImportName::SerdeYaml` (mapped to serde_yaml crate) |
| §12 `HTTP server/client calls` | 423 | `ForbiddenImportName::{Hyper, Reqwest, Axum}` (cross-reference) |
| §43 trigger 7 (Allocation behavior) | 2038 | protected by `ForbiddenImportName::{HashMapStringGeneric, TokioSyncMpscUnbounded}` |
| §43 trigger 8 (Hot-path behavior) | 2039 | protected by all 7 `ForbiddenImportName` variants |
| §43 trigger 9 (Fjall persistence if touched) | 2040 | protected by `ForbiddenImportName::{SerdeJson, SerdeYaml}` |
| §43 trigger 10 (IPC behavior if touched) | 2041 | protected by `ForbiddenImportName::{Hyper, Reqwest, Axum}` |
| §44.6 "JSON and HTTP are absent from `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`" | 2078 | `QuarantinedCrate` set (closed) |
| §78 "scripts/forbid-runtime-fmt.sh exit 0" | 6147 | the gate's contract: exit 0 on a clean tree |

## 8. Open Domain Questions Deferred

The following decisions are NOT domain decisions; they are
implementation decisions owned by other states. The rust-contract
agent has made the domain model invariant under any choice the
implementation agent makes.

- OQ-001 (separate yml vs in-line entry in all.yml): the contract
  binds the moon task *existence* and *ordering* (deps: before cargo
  check) but does not bind the file layout.
- OQ-002 (rustc vs clippy-driver): the contract binds Holzmann
  compliance of the scanner source; the choice of compiler is free.
- OQ-003 (four-crate scope): the contract binds the closed set
  `vb_core`, `vb_runtime`, `vb_storage`, `vb_ipc`. Other crates
  are not in the gate's domain.
- OQ-004 (source-only): the contract binds the input glob
  `crates/<name>/**/*.rs`; Cargo.toml dep tables are out of scope.

## 9. Out-of-Scope Vocabulary (Explicitly Excluded)

The following terms are not in the ubiquitous language and must not
appear in subsequent artifacts:

- "test marker" (the cold-marker set is the only path-classifier; the
  cold markers are not split into "test" and "fixture" sub-concepts).
- "dep table check" (Cargo.toml dependency check is a sibling gate's
  concern, not this gate's).
- "transitive lock entry" (Cargo.lock is excluded).
- "full repo scan" (the gate scans the four hot crates only).
- "yaml::Value" (YAML interpretation is banned by §2 line 98; the
  forbidden import for YAML coverage is the crate name `serde_yaml`).
