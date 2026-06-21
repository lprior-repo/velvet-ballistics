# CV-001: `Taint` does not derive `Ord`/`PartialOrd`, forcing fragile discriminant duplication in `join_taint`

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/value/taint.rs:7` (and `:21`)
- **Confidence**: confirmed

## Description

`Taint` is declared `#[repr(u8)]` with explicit discriminants `Clean = 0,
DerivedFromSecret = 1, Secret = 2`, but derives only `PartialEq, Eq` —
not `Ord, PartialOrd`. As a result, `join_taint` cannot use `a.max(b)`
and instead re-encodes the discriminants by hand in a `match`. If a
future change to the enum's discriminant values or variant order fails
to update `join_taint`, the lattice join silently inverts.

## Evidence

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum Taint {
    Clean = 0,
    DerivedFromSecret = 1,
    Secret = 2,
}

pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    if a_disc >= b_disc { a } else { b }
}
```

(`crates/vb_core/src/value/taint.rs:7-17` and `:21-33`)

The two `match` blocks duplicate the discriminant mapping that the
`#[repr(u8)]` already provides.

## Adversarial Check

A defender might say "the manual match avoids an `as u8` cast, which is
forbidden by the Holzman rule." But the proper fix is not to introduce
`as` — it is to derive `PartialOrd, Ord` on the enum, which compares
variants by declaration order (which *is* the lattice order, since
`Clean < DerivedFromSecret < Secret` in source order). The derived `Ord`
impl is independent of the numeric discriminants and so is robust to
discriminant changes. As written, `join_taint` and the enum's
discriminants are two sources of truth that can drift.

## Suggested Fix

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
         Serialize, Deserialize)]
#[repr(u8)]
pub enum Taint { ... }

pub fn join_taint(a: Taint, b: Taint) -> Taint {
    a.max(b)
}
```

This collapses the function to a one-liner and removes the duplicate
discriminant mapping.
