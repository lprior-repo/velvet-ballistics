# Type Contracts — vb-7akm0 Lint-Suppression Audit

| Field | Value |
|---|---|
| bead_id | vb-7akm0 |
| state | 3 (rust-contract) |
| skill | rust-contract |
| captured_at | 2026-07-01T16:04Z |
| upstream_artifacts | STATE.md, codebase-map.md, delivery-scope.jsonl, domain-model.md |

## 0. Scope

This file defines Rust-style type contracts (newtypes, smart constructors, enums, structs, parser boundaries, and visibility invariants) for the lint-suppression audit domain. The contracts are domain specifications; they describe what the lint-cleanup refactor MUST produce. No production implementation is emitted here.

Notation conventions:
- `struct Foo(Inner)` denotes a newtype.
- `enum Foo { ... }` denotes an algebraic enum.
- "Visibility invariant" lines are post-conditions that all valid `Suppression` records MUST preserve.
- "Forbidden" lines are values the smart constructor MUST reject.

## 1. Newtypes (Identity Primitives)

### 1.1 `RepoPath`

```rust
/// Repository-relative path. Reject absolute paths and `..` segments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RepoPath(SmolStr);

impl RepoPath {
    /// Smart constructor. Rejects:
    /// - empty string
    /// - leading `/`
    /// - any `..` segment
    /// - segments containing `\0` or `\n`
    pub fn new(s: impl AsRef<str>) -> Result<Self, RepoPathError>;

    /// Returns the inner string slice.
    #[must_use]
    pub fn as_str(&self) -> &str;
}
```

Forbidden values: `""`, `"/foo"`, `"../foo"`, `"foo\0bar"`, `"foo\nbar"`. The smart constructor returns `Err(RepoPathError)` for any of these.

### 1.2 `SmolStr`

Re-exported from the `smol_str` crate; used as the inner type for all string-valued newtypes. Length-capped at 1024 bytes.

## 2. Enums (Discriminated States)

### 2.1 `SuppressionKind`

```rust
/// Why the `#[allow(unreachable_pub)]` exists on this file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SuppressionKind {
    /// File has zero `pub` items; the allow is a no-op.
    VestigialSuppression,
    /// File has duplicate canonical export; pub fns are reachable only via crate-internal paths.
    InternalDuplicate,
    /// File mirrors a canonical export; reachable from sibling test submodules via direct paths.
    ExternalDuplicate,
    /// File declares types used across multiple #[cfg(test)] submodules; pub(crate) suffices.
    CrossTestType,
    /// File is the diag module whose constants/functions may be externally reachable via re-export.
    DiagModule,
    /// File is the diagnostic.rs re-export of diag_render items.
    DiagnosticReexport,
    /// File is in vb_cli with a dormant-test decision requirement.
    OrphanTestDecision,
}
```

### 2.2 `Category`

```rust
/// Codebase-map category letter A..G (mapped from `codebase-map.md` §21-§109).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Category {
    VestigialSuppression = 0,    // A
    GateInternalDuplicate = 1,   // B
    TaintTypeDuplicate = 2,      // C
    SchemaSupportNarrow = 3,     // D
    DiagModuleMixed = 4,         // E
    DiagnosticReexport = 5,      // F
    OrphanTestDecision = 6,      // G
}
```

### 2.3 `Treatment`

```rust
/// What the implementation owner must do to clear the suppression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Treatment {
    /// Delete the inner-attribute line; the lint does not fire on the remaining items.
    DeleteAllow,
    /// Change `pub` to `pub(crate)`; the lint does not fire on `pub(crate)` items.
    PubToPubCrate,
    /// Change `pub fn` to bare `fn`; reachability via crate-internal direct paths is preserved.
    PubFnToFn,
    /// Defer to a human/architect decision: retire the orphan test or register it.
    DecisionRequired { recommendation: DecisionRecommendation },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DecisionRecommendation {
    /// Retire the orphan test; narrow CLI items to private/pub(crate).
    RetireOrphanTest,
    /// Register the orphan test in workspace_tests/Cargo.toml.
    RegisterOrphanTest,
    /// Change `pub mod commands_diff;` to `pub(crate) mod commands_diff;`.
    NarrowModuleVisibility,
}
```

Forbidden combinations:
- `Treatment::PubFnToFn` paired with a `Suppression` whose `pub_items_at_file_scope` contains a `Const` or `Struct` (those have no `fn` to narrow). The smart constructor rejects this.
- `Treatment::PubToPubCrate` paired with a `Suppression` whose items already have `Visibility::PubCrate` (already narrowed). The smart constructor rejects this as a no-op.
- `Treatment::DecisionRequired` paired with a `Suppression` whose `category` is NOT `OrphanTestDecision`.

### 2.4 `Visibility`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Visibility {
    /// Bare `pub`.
    Pub,
    /// `pub(crate)`.
    PubCrate,
    /// `pub(super)`.
    PubSuper,
    /// No `pub` keyword; default private visibility.
    Private,
}

impl Visibility {
    /// True if the lint `unreachable_pub` is suppressed by this visibility.
    #[must_use]
    pub fn silences_unreachable_pub(&self) -> bool {
        !matches!(self, Self::Pub)
    }

    /// True if the item is reachable from outside the defining crate.
    #[must_use]
    pub fn is_externally_visible(&self) -> bool {
        matches!(self, Self::Pub)
    }
}
```

### 2.5 `ItemKind`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ItemKind {
    Fn,
    Struct,
    Enum,
    Const,
    Use,
    Mod,
}
```

### 2.6 `ImportStyle`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImportStyle {
    /// `use crate::path::name;` from a sibling module.
    CratePath,
    /// `use super::*;` from a child `#[cfg(test)] mod tests`.
    SuperPath,
    /// `use super::name;` from a child `#[cfg(test)] mod tests`.
    SuperExplicit,
}

impl ImportStyle {
    /// True if this import style resolves through crate-internal direct paths
    /// (and therefore works for `fn` items without `pub`).
    #[must_use]
    pub fn is_crate_internal(&self) -> bool {
        matches!(self, Self::CratePath | Self::SuperPath | Self::SuperExplicit)
    }
}
```

### 2.7 `RiskTag`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RiskTag {
    LintSuppressionAudit,
    TestVisibility,
    PublicApi,
    DormantArtifact,
    DecisionRequired,
    ProductionBindingVerification,
    TestSuiteReverify,
}
```

## 3. Structs (Composite Records)

### 3.1 `PubItemRef`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PubItemRef {
    /// e.g. "validate_gate_07_expression_stack_depth".
    pub item_name: SmolStr,
    pub item_kind: ItemKind,
    /// 1-indexed line of the item declaration.
    pub line: u32,
    /// Current visibility of the item BEFORE the narrowing.
    pub current_visibility: Visibility,
}

impl PubItemRef {
    /// Smart constructor. Rejects empty `item_name` and `line == 0`.
    pub fn new(
        item_name: impl AsRef<str>,
        item_kind: ItemKind,
        line: u32,
        current_visibility: Visibility,
    ) -> Result<Self, PubItemRefError>;
}
```

### 3.2 `ConsumerRef`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConsumerRef {
    pub consumer_path: RepoPath,
    /// 1-indexed line of the import/use statement.
    pub consumer_line: u32,
    pub import_style: ImportStyle,
}
```

Invariant: `consumer_line >= 1`; `import_style.is_crate_internal() == true` for ALL consumers in Categories B/C/D (this is what makes the narrowing safe).

### 3.3 `Suppression`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Suppression {
    pub file_path: RepoPath,
    pub suppression_line: u32,
    pub kind: SuppressionKind,
    pub category: Category,
    pub pub_items_at_file_scope: Vec<PubItemRef>,
    pub consumers: Vec<ConsumerRef>,
    pub externally_reachable_items: Vec<PubItemRef>,
    pub recommended_treatment: Treatment,
    pub risk_tags: Vec<RiskTag>,
    pub behavior_affecting: bool,
    pub production_bound: bool,
}

impl Suppression {
    /// Smart constructor. Enforces:
    /// - `suppression_line >= 1`
    /// - `pub_items_at_file_scope` MAY be empty only if `kind == VestigialSuppression`
    /// - `category` and `kind` MUST be consistent (see §2.2 mapping)
    /// - `treatment` MUST match the kind/category (see §2.3 forbidden combinations)
    /// - `behavior_affecting == false` (this bead is behavior-preserving)
    /// - If `category == OrphanTestDecision`, `treatment == DecisionRequired` and a recommendation is set
    pub fn new(...) -> Result<Self, SuppressionError>;

    /// Returns true if this suppression can be cleared by removing the inner attribute alone
    /// (no visibility change required).
    #[must_use]
    pub fn is_delete_allow_only(&self) -> bool {
        matches!(self.recommended_treatment, Treatment::DeleteAllow)
    }

    /// Returns true if this suppression is behavior-preserving (it MUST be for every row in this bead).
    #[must_use]
    pub fn is_behavior_preserving(&self) -> bool {
        !self.behavior_affecting
    }
}
```

### 3.4 `VisibilityInvariant`

```rust
/// Rust visibility invariant: a `pub` item is only allowed at a syntactic location
/// when there is at least one external path that reaches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityInvariant {
    /// All `pub` items remaining in the post-refactor source.
    pub pub_items: Vec<PostRefactorItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostRefactorItem {
    pub file_path: RepoPath,
    pub item_name: SmolStr,
    pub visibility: Visibility,
    pub reachable_via_external_path: bool,
}

impl VisibilityInvariant {
    /// Returns true iff every `pub` item has `reachable_via_external_path == true`.
    /// This is the post-refactor assertion the lint-src gate must verify.
    #[must_use]
    pub fn is_satisfied(&self) -> bool {
        self.pub_items
            .iter()
            .filter(|item| item.visibility == Visibility::Pub)
            .all(|item| item.reachable_via_external_path)
    }
}
```

## 4. Smart Constructor Error Variants

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RepoPathError {
    #[error("repo path is empty")]
    Empty,
    #[error("repo path must not start with `/` (got {0:?})")]
    AbsolutePath(String),
    #[error("repo path contains `..` segment (got {0:?})")]
    PathTraversal(String),
    #[error("repo path contains null byte")]
    NullByte,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PubItemRefError {
    #[error("item name is empty")]
    EmptyName,
    #[error("item line must be >= 1 (got {0})")]
    ZeroLine(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SuppressionError {
    #[error("suppression line must be >= 1 (got {0})")]
    ZeroLine(u32),
    #[error("vestigial suppression declared but pub_items is non-empty")]
    VestigialWithItems,
    #[error("non-vestigial suppression declared but pub_items is empty")]
    NonVestigialWithoutItems,
    #[error("category {category:?} does not match kind {kind:?}")]
    CategoryKindMismatch { category: Category, kind: SuppressionKind },
    #[error("treatment {treatment:?} is incompatible with category {category:?}")]
    TreatmentCategoryMismatch { treatment: Treatment, category: Category },
    #[error("this bead is behavior-preserving; behavior_affecting must be false")]
    BehaviorAffectingTrue,
    #[error("decision recommendation missing for category OrphanTestDecision")]
    MissingRecommendation,
}
```

## 5. Parser Boundary (Single Parsing Point)

External input (the `delivery-scope.jsonl` rows) MUST enter the lint-audit core only through this parse function. The parse function is the ONLY point at which smart-constructor validation runs.

```rust
/// Parses a `Suppression` from a `delivery-scope.jsonl` row.
/// Rejects rows whose `file_path` is not a `RepoPath`, whose `suppression_line == 0`,
/// whose `kind`/`category` are inconsistent, or whose `behavior_affecting == true`.
pub fn parse_suppression_from_delivery_scope(
    row: &serde_json::Value,
) -> Result<Suppression, SuppressionParseError>;
```

External callers MUST NOT construct `Suppression` directly via struct-literal syntax. Every instance MUST come from `parse_suppression_from_delivery_scope`.

## 6. Visibility Invariants (Refinement Notes for `proof-planner`)

The following visibility-level refinements are recommended for the downstream `proof-planner` to express as `verus`/`flux`/`proptest` obligations:

| Refinement | Suggested verifier | Affected rows |
|---|---|---|
| `Suppression::is_delete_allow_only()` returns true iff `recommended_treatment == Treatment::DeleteAllow` | flux-rs (refinement) | All DeleteAllow rows |
| `Visibility::silences_unreachable_pub()` returns true for everything except `Pub` | verus (postcondition) | All narrowed rows |
| For every Category B/C `Suppression`, all `ConsumerRef.import_style.is_crate_internal() == true` | verus (refinement) | Categories B/C |
| For every Category D `Suppression`, `externally_reachable_items.is_empty()` | verus (refinement) | Category D |
| For every `Suppression`, `behavior_affecting == false` | flux-rs (refinement) | All rows |
| `VisibilityInvariant::is_satisfied()` returns true after all 25 suppressions are cleared | verus (postcondition); proptest (cross-check via `moon run :lint-src`) | All rows |

## 7. Cross-Type Invariants (Bead-Wide)

| Invariant | Scope | Enforcement |
|---|---|---|
| `Suppression.file_path` MUST NOT be `crates/workspace_tests/tests/vb_test_cli_diff_incident_behavior.rs` | All rows | The orphan test is NOT a suppression site; the suppression sites are in `crates/vb_cli/src/`. |
| `Suppression.suppression_line` MUST equal the line number of the `#[allow(unreachable_pub)]` inner attribute | All rows | Verified by `grep -n` during the implementation pass |
| After parsing, the count of `Suppression` records MUST equal 25 | Bead-wide | The validator runs `jq 'length'` on the parsed JSONL |
| The category distribution MUST match: A=4, B=6 (or 8 counting taint/type as B), C=1 (secret_leak) or 3 (counting taint/type), D=4, E=3, F=1, G=3 | Bead-wide | Cross-checked against delivery-scope.jsonl row 45 (rollup) |
| Every `Suppression.behavior_affecting == false` | All rows | Behavior-preservation is the bead's defining property |

## 8. Open Type-Level Questions

1. Whether `Suppression` should expose `treatment_applied: Option<Treatment>` (defaulting to `None`) so the implementation owner can stamp the actually-applied treatment after the refactor. Recommendation: yes, but defer to State 5 (holzman-rust).
2. Whether `PubItemRef.current_visibility` should be split into a separate `PreRefactorVisibility` and `PostRefactorVisibility` field pair. Recommendation: yes for the post-refactor record, but defer.
3. Whether `ConsumerRef` should carry the verbatim `use` line text (not just the line number) for forensics. Recommendation: yes, but defer.

End of type-contracts.md.