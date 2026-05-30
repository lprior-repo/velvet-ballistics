# Architectural Drift Report: vb_yaml/profile.rs

**File:** `crates/vb_yaml/src/profile.rs`  
**Date:** 2026-05-29  
**Status:** DRIFT DETECTED

---

## 1. Line Count Analysis

| Module | Lines | Limit (300) | Status |
|--------|-------|-------------|--------|
| profile.rs | 24 | ✓ | OK |
| profile_dupkeys.rs | 101 | ✓ | OK |
| profile_validation.rs | 374 | ✗ | **VIOLATION** |
| **Total** | **499** | — | — |

**Line Count Violation:** `profile_validation.rs` exceeds the 300-line limit at **374 lines** (+74 lines over limit).

---

## 2. DDD Cohesion Analysis

### Single Responsibility Principle Violations

`profile_validation.rs` violates SRP by conflating **8 distinct responsibilities**:

1. **Profile orchestration** — `validate_yaml_profile()`, `validate_yaml_profile_with_limits()`
2. **Source size validation** — `check_source_size()`, `check_null_bytes_in_source()`
3. **Event collection** — `collect_and_validate_events()`
4. **Scalar validation** — `check_scalar_length()`, `check_null_bytes()`
5. **Feature rejection** — `reject_forbidden_features()`, `is_allowed_tag()`, `reject_binary_scalar()`
6. **Anchor/alias/merge rejection** — `reject_anchors_aliases_merges()`, `is_merge_key_tag()`
7. **Multi-document rejection** — `reject_multiple_documents()`
8. **YAML 1.1 ambiguity rejection** — `reject_yaml_1_1_ambiguous_scalars()`, `check_scalar_ambiguity()`

### Primitive Obsession

| Type | Usage | Should Be |
|------|-------|-----------|
| `u16` | `depth` | `Depth(u16)` or `NonZeroU16` |
| `u32` | `node_count` | `NodeCount(u32)` |
| `usize` | `max_bytes`, `max_nodes`, counters | NewType wrappers |
| `usize` | `document_count` | `DocumentCount(usize)` |

### Leaky Abstractions

Functions exposed as `pub` that should be `pub(crate)`:
- `is_allowed_tag()`
- `reject_binary_scalar()`
- `is_merge_key_tag()`
- `check_scalar_ambiguity()`
- `reject_forbidden_features()`
- `reject_anchors_aliases_merges()`
- `reject_multiple_documents()`
- `reject_yaml_1_1_ambiguous_scalars()`

These are internal validation concerns that should not be part of the public API.

---

## 3. Identified Violations

| # | Violation | Location | Severity |
|---|-----------|----------|----------|
| 1 | File exceeds 300 lines | profile_validation.rs:374 | **CRITICAL** |
| 2 | SRP violation — 8 responsibilities in one module | profile_validation.rs | **HIGH** |
| 3 | Primitive obsession — raw integer types | profile_validation.rs:63-65 | **MEDIUM** |
| 4 | Leaky abstractions — 8 internal fns marked `pub` | profile_validation.rs | **MEDIUM** |
| 5 | Domain/infrastructure mixing — saphyr_parser direct use | profile_validation.rs:61 | **LOW** |

---

## 4. Recommended Refactoring

Split `profile_validation.rs` into:

```
src/
  profile.rs              # orchestrator (24 lines)
  profile_dupkeys.rs      # duplicate key detection (101 lines)
  profile_validation.rs   # orchestration only (keep ~50 lines)
  profile_limits.rs       # size/depth limits checking (~40 lines)
  profile_features.rs     # forbidden feature rejection (~60 lines)
  profile_ambiguity.rs    # YAML 1.1 ambiguous scalars (~30 lines)
```

---

## 5. Summary

| Metric | Value |
|--------|-------|
| Total Lines | 499 |
| Violation Count | 5 |
| DDD Smell | **SRP violation, Primitive obsession, Leaky abstractions** |
| Priority | **MEDIUM** — file must be split before further drift |

---

**STATUS: REFACTORED** — requires splitting `profile_validation.rs`
