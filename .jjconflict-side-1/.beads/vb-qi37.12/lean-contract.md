# Theorem Kernel Projection: vb-qi37.12

## Boundary
- TLA+-owned temporal model: persistence-before-ack, recovery fail-closed lifecycle, and diagnostic preservation through terminal runtime failure.
- Verus-owned Rust core: discard classification lattice, diagnostic envelope preservation, and recovery decode classification.
- Theorem-owned kernel: none at State 3.
- Rust/runtime shell: Fjall I/O, filesystem locks, postcard decoding implementation, runtime shard mutation, compiler parsing/validation, CLI/API presentation.
- External systems excluded from theorem proof: database, filesystem, scheduler, process lifecycle, wall-clock time.

## Theorem-Owned Clauses
- None. No tiny algebraic kernel currently exceeds Verus scope.

## Theorem Obligations
- No Lean/Aeneas/Hax obligation is required unless State 4 discovers a small algebraic kernel that Verus cannot express without unreasonable trusted code.

## Verus Sufficiency Statement
- INV-004 can be modeled as a finite classification lattice in Verus.
- INV-002 can be modeled as a diagnostic-envelope transformation property in Verus.
- INV-003 can be modeled as a decode classification property where corrupt bytes never refine to successful absence.

## Waivers
- Lean waiver for State 3: owner `rust-contract`; reason `Verus can express the identified Rust-local pure obligations and no theorem-kernel-only claim is present`; expiry `before State 5 proof writing`; compensating evidence `State 4 proof plan must either keep this waiver or introduce exact Lean/Aeneas/Hax targets`.
