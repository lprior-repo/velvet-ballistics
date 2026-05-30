# ARCH-DRIFT HAMMER REPORT
## Target: `crates/vb_cli/src/naming_scan/config.rs`
## Line Count: 351 (VIOLATES 300-LINE HARD LIMIT)

---

## EXECUTIVE SUMMARY

| Violation | Severity | Status |
|-----------|----------|--------|
| File exceeds 300 lines (351) | CRITICAL | MUST FIX |
| Primitive obsession: `String` return type on `fingerprint_for_destination` | HIGH | SHOULD FIX |
| Primitive obsession: bare `&[String]` in validation signatures | MEDIUM | SHOULD FIX |
| Test helpers buried inline in production module (186–351) | MEDIUM | MUST FIX |

**The file is 17% over the line limit and structurally violates DDD cohesion.**

---

## VIOLATION 1 — FILE SIZE (CRITICAL)

**351 lines found. Maximum allowed: 300. Overage: 51 lines (17%).**

The file contains three distinct DDD responsibility layers all welded together:

```
Lines   Responsibility
─────   ─────────────────────────────────────────────────
1–16    CanonicalSpellingTable factory
18–33   validate_scan_config (workflow entry point)
35–39   invalid_config error constructor
41–51   validate_patterns (validation sub-routine)
53–71   validate_allowlist (validation sub-routine)
73–90   table_from_entries (transformation)
92–102  validate_entry (validation sub-routine)
105–119 duplicate_error / missing_error (error constructors)
121–131 required_kinds (pure helper)
133–143 kind_name (pure helper)
145–154 expected_token (pure helper)
156–162 fingerprint_for_destination (pure helper)
165–351 ALL TESTS
```

**DDD Principle violated:** One file = one domain concept. This file contains **four** concepts: factory, validation workflow, error domain, and tests. Each must be a separate module.

**Required split:**

| Module | Lines | Responsibility |
|--------|-------|----------------|
| `config.rs` (new) | ~15 | Public API: `canonical_spelling_table`, `validate_scan_config` |
| `validation.rs` | ~80 | All `validate_*`, `required_kinds`, `kind_name`, `expected_token`, `table_from_entries` |
| `config_errors.rs` | ~20 | `invalid_config`, `duplicate_error`, `missing_error` constructors |
| `config/tests.rs` | ~186 | All test helpers + tests moved from config.rs |

---

## VIOLATION 2 — PRIMITIVE OBSESSION: `String` FINGERPRINT (HIGH)

**Location:** Line 156–162

```rust
fn fingerprint_for_destination(destination: Option<&PathBuf>) -> String {
    if destination.is_some() {
        "vb-37lc-maximum-bounded-config".to_owned()
    } else {
        "vb-37lc-minimum-config".to_owned()
    }
}
```

**Problem:** The return type is bare `String`. This is a semantic value object — a **config fingerprint** — that carries meaning (`"vb-37lc-maximum-bounded-config"` vs `"vb-37lc-minimum-config"`). Using `String` means any caller can accidentally concatenate, slice, or compare it incorrectly.

**Scott Wlaschin:** *"Make illegal states unrepresentable."* A bare `String` can be any arbitrary text. A `ConfigFingerprint` NewType can only be the two valid variants.

**Required fix:**
```rust
pub struct ConfigFingerprint(String);

impl ConfigFingerprint {
    pub fn maximum() -> Self { Self("vb-37lc-maximum-bounded-config".to_owned()) }
    pub fn minimum() -> Self { Self("vb-37lc-minimum-config".to_owned()) }
}
```

---

## VIOLATION 3 — PRIMITIVE OBSESSION: `&[String]` IN SIGNATURES (MEDIUM)

**Location:** Lines 41, 53

```rust
fn validate_patterns(patterns: &[String]) -> Result<(), NamingScanError>
fn validate_allowlist(rules: &[LegacyAllowRule]) -> Result<(), NamingScanError>
```

`&[String]` is a primitive array. The validation domain has a semantic concept: **PatternSet** or **ScanPatterns**. This NewType also carries the opportunity for `Parse, don't validate` — patterns could be compiled once at construction time rather than checked naively character-by-character.

**Required fix:**
```rust
pub struct ScanPatterns(Vec<String>);  // in types.rs or a new patterns module
pub struct LegacyAllowlist(Vec<LegacyAllowRule>);
```

---

## VIOLATION 4 — TEST HELPERS BURIED IN PRODUCTION MODULE (MEDIUM)

Lines 180–351 (172 lines) are entirely tests and test helpers. The `valid_entries()` and `valid_config()` helpers are substantial — they belong in `config/tests.rs`, not inline in production code.

**Impact:** Bloats production module, mixes test concern with domain concern.

---

## SECONDARY FINDING: `types.rs` IS ALSO OVERSIZED

`crates/vb_cli/src/naming_scan/types.rs` is **429 lines** — also violates the 300-line limit. It contains multiple DDD concepts that should be separated:

- `CanonicalSpellingTable`, `CanonicalEntry`, `CanonicalNameKind` → `canonical.rs`
- `LegacyAllowRule`, `AllowlistPolicy` → `allowlist.rs` (already exists as `allowlist.rs` in sibling — investigate import cycle)
- `RawScanConfig`, `ScanConfig` → `config.rs`
- `RepoPath`, `RepoRoot` → `repo.rs`
- `LineNumber`, `ColumnNumber` → `position.rs`
- `NamingFinding`, `OccurrenceClass`, `ScanInput`, `ScanReport`, `RenderedReport` → `scan_result.rs`
- `NamingScanError` → `errors.rs`
- `SpellingClass` → `classify.rs`

This cascading split is consistent with how `mod.rs` already partitions siblings: `allowlist`, `classify`, `config`, `discovery`, `legacy`, `line_scan`, `ordering`, `report`, `repository`, `types`.

---

## SUMMARY OF REQUIRED REFACTORS

1. **Extract `validation.rs`** — move `validate_patterns`, `validate_allowlist`, `validate_entry`, `table_from_entries`, `required_kinds`, `kind_name`, `expected_token`
2. **Extract `config_errors.rs`** — move `invalid_config`, `duplicate_error`, `missing_error`
3. **Create `ConfigFingerprint` NewType** — replace bare `String` return in `fingerprint_for_destination`
4. **Move tests to `config/tests.rs`** — `valid_entries`, `valid_config`, all test functions
5. **Prune `config.rs` to ~15 lines** — only `canonical_spelling_table` and `validate_scan_config` remain
6. **Audit `types.rs`** — further split per DDD concept boundaries

---

## DDD COHESION SCORE

| Metric | Score |
|--------|-------|
| Single responsibility | 2/10 (4 concepts in 1 file) |
| Primitive obsession | 5/10 (String fingerprints, raw slices) |
| Type safety | 7/10 (good enums, some NewTypes, missing ConfigFingerprint) |
| Test isolation | 3/10 (tests inline in production module) |
| File size compliance | 0/10 (351 > 300) |

**Overall: 3.4 / 10 — SIGNIFICANT DRIFT**

---

*Generated by: architectural-drift agent*
*Target: naming_scan/config.rs*
*Workspace: arch-drift-hammer*
