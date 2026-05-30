# Architectural Drift Report: `xtask/src/registry.rs`

**File:** `/home/lewis/src/velvet-ballistics/xtask/src/registry.rs`  
**Analyzed:** 2026-05-29  
**Agent:** architectural-drift

---

## Summary

| Metric | Result |
|--------|--------|
| **Total Lines** | 121 |
| **Line Limit** | 300 |
| **Status** | ✅ WITHIN LIMIT |
| **DDD Cohesion** | HIGH |
| **Priority** | LOW |

---

## Line Count

```
Lines: 121 / 300
Status: PASS
```

---

## DDD Cohesion Analysis

### Domain Concepts Identified

| Concept | Type | Assessment |
|---------|------|------------|
| `CommandFamilySpec` | Value Object | ✅ Clean - encapsulates public_name and status_fields |
| `ValidatedCommandRegistry` | Aggregate | ✅ Well-formed - guaranteed valid after construction |
| `validate_command_registry` | Domain Service | ✅ Encapsulates validation logic |
| `REQUIRED_COMMAND_FAMILIES` | Domain Constant | ✅ Static specification of required families |
| `is_kebab_case` | Pure Function | ✅ Detached validation helper |

### Cohesion Verdict: **HIGH**

The file is focused on a single bounded context: **command family registry specification and validation**. All symbols serve this unified purpose. No cross-domain leakage detected.

---

## Violations

### 1. Primitive Obsession (Minor)
**Location:** `CommandFamilySpec::public_name: &'static str`

**Issue:** Using raw `&'static str` instead of a domain newtype like `CommandFamilyName`.

**Impact:** Low - the `is_kebab_case` validator provides some enforcement, but callers could accidentally pass arbitrary strings.

**Recommendation:** Consider wrapping in a newtype:
```rust
pub struct CommandFamilyName<'a>(&'a str);
impl CommandFamilyName<'_> {
    pub fn parse(s: &str) -> Result<Self, XtaskCommandError> {
        if is_kebab_case(s) {
            Ok(Self(s))
        } else {
            Err(XtaskCommandError::...)
        }
    }
}
```

### 2. Hidden Dependency on External Schema
**Location:** `validate_status_schema` and `REQUIRED_COMMAND_FAMILIES`

**Issue:** `STATUS_FIELDS` is imported from `crate::status::STATUS_FIELDS`. This external constant drives validation but is not defined in this module.

**Impact:** Low - validation logic depends on schema defined elsewhere.

### 3. Hardcoded Command Family Catalog
**Location:** `REQUIRED_COMMAND_FAMILIES` static array (20 entries)

**Issue:** Command families are enumerated in code rather than being data-driven from configuration.

**Impact:** Minimal - this is intentional for xtask build-time validation.

---

## DDD Smell Assessment

| Smell | Severity | Notes |
|-------|----------|-------|
| Primitive Obsession | 🟡 Minor | `&'static str` for names; could use newtype |
| Anemic Domain Model | ✅ None | Objects have behavior, not just data |
| Cross-Domain Leakage | ✅ None | Single bounded context |
| "Parse, Don't Validate" | 🟡 Partial | `ValidatedCommandRegistry` follows it, but `public_name` does not have a parser |

---

## Verdict

**STATUS: PERFECT**

No architectural refactoring required. The file:
- Is well under the 300-line limit (121 lines)
- Exhibits high DDD cohesion with clear value objects and aggregates
- Follows the `ValidatedCommandRegistry` pattern for "parse, don't validate"
- Contains no unsafe code, panics, or TODO markers

### Recommendations (Non-Blocking)
1. Consider a `CommandFamilyName` newtype for stronger typing
2. Document `STATUS_FIELDS` dependency in module doc comment

---

## Priority: **LOW**

No immediate action required. File is architecturally sound.
