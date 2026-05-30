# ARCHITECTURAL DRIFT REPORT: naming_scan/types.rs

## FILE OVERVIEW
- **Path**: `crates/vb_cli/src/naming_scan/types.rs`
- **Line Count**: 429 lines
- **Status**: ❌ VIOLATION — Exceeds 300-line hard limit by 129 lines (43% over)

---

## VIOLATION SUMMARY

| Rule | Status | Details |
|------|--------|---------|
| Line Count | ❌ FAIL | 429 > 300 (129 line overage) |
| Primitive Obsession | ❌ FAIL | 12 instances of raw `String`/`Vec<String>` |
| DDD Cohesion | ⚠️ PARTIAL | NewTypes present but inconsistent |
| Parse, Don't Validate | ⚠️ MISSING | No validation in constructors |

---

## 1. LINE COUNT VIOLATION (CRITICAL)

**File is 429 lines — must be split into minimum 2 files.**

Suggested split:
```
naming_scan/
├── types/
│   ├── mod.rs           (~30 lines — re-exports)
│   ├── constants.rs     (~25 lines — CANONICAL_* constants only)
│   ├── newtypes.rs      (~80 lines — RepoPath, RepoRoot, LineNumber, ColumnNumber, RenderedReport)
│   ├── enums.rs         (~100 lines — CanonicalNameKind, SpellingClass, LegacyException, OccurrenceClass, ScanInput, NamingScanError)
│   ├── config.rs        (~80 lines — RawScanConfig, ScanConfig, AllowlistPolicy, LegacyAllowRule)
│   ├── table.rs         (~40 lines — CanonicalSpellingTable, CanonicalEntry)
│   └── finding.rs       (~50 lines — NamingFinding, ScanReport)
└── tests/
    └── types_tests.rs   (~100 lines — all tests)
```

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 `CanonicalSpellingTable` — Raw Strings for Domain Concepts
**Lines 10-19**

```rust
pub struct CanonicalSpellingTable {
    pub product: String,           // ← Should be CanonicalProductName
    pub binary: String,             // ← Should be CanonicalBinaryName
    pub package: String,           // ← Should be CanonicalPackageName
    pub bead_rig: String,          // ← Should be CanonicalBeadRigName
    pub crate_module: String,      // ← Should be CanonicalCrateModuleName
    pub bead_database: String,     // ← Should be CanonicalBeadDatabaseName
    pub language_version: String,  // ← Should be CanonicalLanguageVersionName
}
```

**Required NewTypes:**
```rust
pub struct CanonicalProductName(String);
pub struct CanonicalBinaryName(String);
// ... etc
```

**Why:** Each field is a distinct domain concept with different validation rules. A `product` name is NOT the same type as a `language_version`. Using raw `String` allows mixing them at compile time.

---

### 2.2 `LegacyAllowRule` — Raw Strings in Enum Variants
**Lines 58-81**

```rust
pub enum LegacyAllowRule {
    RepositoryPath { path: String },           // ← NewType: LegacyRepoPath
    MasterFilename { filename: String },       // ← NewType: LegacyFilename
    MigrationReference { label: String, artifact: String, legacy_text: String },
    Wildcard { pattern: String },              // ← NewType: GlobPattern
    PrefixOnly { prefix: String },              // ← NewType: NamePrefix
    Substring { needle: String },               // ← NewType: SearchNeedle
}
```

**Required NewTypes:**
```rust
pub struct LegacyRepoPath(String);
pub struct LegacyFilename(String);
pub struct MigrationLabel(String);
pub struct MigrationArtifact(String);
pub struct LegacyText(String);
pub struct GlobPattern(String);
pub struct NamePrefix(String);
pub struct SearchNeedle(String);
```

---

### 2.3 `RawScanConfig` — Raw String Vectors
**Lines 89-97**

```rust
pub struct RawScanConfig {
    pub canonical_entries: Vec<CanonicalEntry>,
    pub legacy_allowlist: Vec<LegacyAllowRule>,
    pub scan_patterns: Vec<String>,           // ← NewType: ScanPattern
    pub excluded_path_rules: Vec<String>,     // ← NewType: ExclusionRule
    pub workspace_root: PathBuf,
    pub report_destination: Option<PathBuf>,
}
```

**Required NewTypes:**
```rust
pub struct ScanPattern(String);
pub struct ExclusionRule(String);
```

---

### 2.4 `ScanConfig` — Raw String Fingerprint
**Lines 113-121**

```rust
pub struct ScanConfig {
    pub canonical_table: CanonicalSpellingTable,
    pub allowlist_policy: AllowlistPolicy,
    pub scan_patterns: Vec<String>,           // ← Should be Vec<ScanPattern>
    pub excluded_path_rules: Vec<String>,     // ← Should be Vec<ExclusionRule>
    pub config_fingerprint: String,           // ← NewType: ConfigFingerprint
    pub report_destination: Option<PathBuf>,
}
```

**Required NewType:**
```rust
pub struct ConfigFingerprint(String);
```

---

### 2.5 `NamingFinding` — Raw String Remediation
**Lines 177-184**

```rust
pub struct NamingFinding {
    pub path: RepoPath,
    pub line: LineNumber,
    pub column: ColumnNumber,
    pub spelling_class: SpellingClass,
    pub remediation: String,  // ← NewType: RemediationText
}
```

**Required NewType:**
```rust
pub struct RemediationText(String);
```

---

### 2.6 `LegacyException` — Raw Strings in Enum
**Lines 186-200**

```rust
pub enum LegacyException {
    RepositoryPath { path: String },
    MasterFilename { filename: String },
    MigrationReference { artifact: String, label: String, legacy_text: String },
}
```

**Required NewTypes:** Same as `LegacyAllowRule` variants.

---

### 2.7 `OccurrenceClass` — Raw Strings in Enum Variants
**Lines 202-225**

```rust
pub enum OccurrenceClass {
    NoOccurrence,
    CanonicalProduct { canonical: String, kind: CanonicalNameKind },
    CanonicalCrateModule { canonical: String, kind: CanonicalNameKind },
    CanonicalLanguageVersion { canonical: String, kind: CanonicalNameKind },
    AllowedLegacy { exception: LegacyException },
    InvalidLegacy { spelling_class: SpellingClass, remediation: String },
}
```

**Required NewTypes:**
```rust
pub struct CanonicalToken(String);  // Used in multiple variants
pub struct InvalidRemediation(String);
```

---

### 2.8 `ScanInput` — Raw Strings
**Lines 227-242**

```rust
pub enum ScanInput {
    Text { path: RepoPath, contents: String },   // ← NewType: FileContents
    Bytes { path: RepoPath, bytes: Vec<u8> },    // ← NewType: RawBytes
    File { path: RepoPath, absolute_path: PathBuf },
}
```

**Required NewTypes:**
```rust
pub struct FileContents(String);
pub struct RawBytes(Vec<u8>);
```

---

### 2.9 `ScanReport` — Raw String Fingerprint
**Lines 244-252**

```rust
pub struct ScanReport {
    pub root: RepoRoot,
    pub config_fingerprint: String,  // ← Should be ConfigFingerprint (reuse)
    pub selected_input_count: usize,
    pub scanned_text_input_count: usize,
    pub findings: Vec<NamingFinding>,
    pub report_destination: Option<PathBuf>,
}
```

**Fix:** Use `ConfigFingerprint` type (defined in 2.4).

---

### 2.10 `NamingScanError` — Raw Strings in Error Variants
**Lines 259-269**

```rust
pub enum NamingScanError {
    InvalidRoot { root: RepoRoot },
    InvalidConfiguration { reason: String },           // ← NewType: ConfigReason
    FileDiscoveryFailed { path: RepoPath, source: String },  // ← NewType: IoErrorMessage
    InputReadFailed { path: RepoPath, source: String },      // ← NewType: IoErrorMessage
    PatternCompilationFailed { pattern: String, source: String }, // ← NewType: RegexPattern
    InvalidCanonicalSpelling { findings: Vec<NamingFinding> },
    ReportWriteFailed { path: PathBuf, source: String },     // ← NewType: IoErrorMessage
}
```

**Required NewTypes:**
```rust
pub struct ConfigReason(String);
pub struct IoErrorMessage(String);
pub struct RegexPattern(String);
```

---

## 3. GOOD NEWTYPE PATTERNS (PRESERVE)

The following ARE correct NewType patterns — do not refactor:

| Type | Lines | Validation |
|------|-------|------------|
| `RepoPath(String)` | 123-137 | None (should add non-empty validation) |
| `RepoRoot(PathBuf)` | 139-147 | None (should add dir exists check) |
| `LineNumber(u64)` | 149-157 | None (should add > 0 validation) |
| `ColumnNumber(u64)` | 159-167 | None (should add > 0 validation) |
| `RenderedReport(String)` | 254-257 | None |

---

## 4. MISSING VALIDATION ("Parse, Don't Validate")

**Issue:** No constructor performs validation. Example:

```rust
impl RepoPath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
        // ← Should validate: non-empty, valid UTF-8, correct path separators
    }
}
```

**Required Validation Rules:**
- `RepoPath::new`: Non-empty, valid UTF-8
- `LineNumber::new`: Must be > 0
- `ColumnNumber::new`: Must be > 0
- `RepoRoot::new`: Path must exist and be directory (deferred to runtime)
- All `*Name` NewTypes: Pattern validation (e.g., kebab-case for product names)

---

## 5. TEST PROBLEMS

**Test file location:** Tests are inline (lines 271-429) — 159 lines of tests.

**Issue:** Tests verify implementation details (`path.0`, `ln.0`) rather than behavior.

**Required Fix:** Move to `tests/types_tests.rs` and test via public API only.

---

## 6. CONSTANTS DRIFT

**Lines 3-8:** Constants have TYPOS:
```rust
pub(crate) const CANONICAL_HYPHEN: &str = "velvet-ballastics";  // ← TYPO: "ballastics" not "ballistics"
```

**This is a live bug.** The constant `CANONICAL_HYPHEN` is misspelled. This will cause incorrect scan results.

---

## 7. RECOMMENDED REFACTORING ORDER

1. **Create `constants.rs`** — Fix typo first
2. **Create `newtypes.rs`** — Extract all NewType definitions
3. **Create `enums.rs`** — Extract enums with proper NewType fields
4. **Create `config.rs`** — Extract config structs
5. **Create `table.rs`** — Extract CanonicalSpellingTable, CanonicalEntry
6. **Create `finding.rs`** — Extract NamingFinding, ScanReport
7. **Create `error.rs`** — Extract NamingScanError
8. **Create `mod.rs`** — Re-exports with deprecation notices
9. **Move tests** to `tests/types_tests.rs`

---

## SUMMARY

| Metric | Count |
|--------|-------|
| Total Lines | 429 |
| Over Limit | 129 (43%) |
| Raw String Fields | 35+ |
| Required NewTypes | 25+ |
| Required Files | 8 |
| Typo Bugs | 1 (CANONICAL_HYPHEN) |
| Missing Validations | 5+ constructors |

**VERDICT: REFACTOR REQUIRED**
