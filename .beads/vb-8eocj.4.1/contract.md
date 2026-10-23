# vb-8eocj.4.1: immutable source metadata proposal

Status: draft for Master acceptance, not an integration or completion claim.
The standalone alias/requirement API and existing limits were acknowledged by
Master through the coordinator on 2026-10-23; hook/integration remains unapproved.
Source base: `b612001890c4b1d69ad20945899eed8d1c533e3f`.
The parent bead's existing lexical compiler work and `.beads/vb-8eocj.4`
contract remain unchanged. The coordinator owns tracker lifecycle and approval.

## Approved implementation boundary

The standalone production module is `crates/vb_core/src/workflow/source_metadata.rs`;
its 13 direct unit tests are in `source_metadata/tests.rs`. A dedicated workspace
test target includes those actual files for native qualification. This module is deliberately not
declared in `workflow/mod.rs` yet.
This draft, the adjacent acceptance plan, and migration manifest are delivery
documents, not authority to change any listed integration path.

No workflow hook, constructor migration, compiler integration, serialization,
schema, storage, runtime, credential, security, or actual secret access is
implemented. No aggregate 1 MiB limit is introduced. Reserved runtime drive
files are untouched. Initial native compilation and 13 direct behavioral tests passed at the V95
source pin recorded in `native-v95/README.md`; broader integration remains pending
with Master; ordinary crate checks would not compile this unhooked file.

## Domain and API

`WorkflowSourceMetadata` owns immutable, definition-order symbol payloads and
presence-only secret bindings. Each binding retains both the source alias and
declared host requirement name, for example `token` and `TOKEN`. A `SymbolId` identifies the zero-based
position of its UTF-8 payload in this one workflow's table. IDs are not portable
between workflows or arbitrary runtime stores.

The constructor consumes `Box<[Box<str>]>` and
`Box<[CompiledSecretRequirement]>`, returning typed validation errors.
Read-only accessors expose symbols, checked lookup, count, aggregate byte
measurement, and requirement names. An explicit count-consistency check is
provided for a later WorkflowParts hook. No mutable accessor, unchecked
metadata constructor, implicit deserializer, or runtime interner is exposed.
`CompiledSecretRequirement::new(alias, requirement)` creates table-relative
references, exposed through `alias()` and `requirement()`. Their bounds and nonempty-name invariant become established only when the owning
metadata constructor accepts it.

The compiler must use one checked interner for text literals, object keys,
accessor fields, and requirement names. Equal UTF-8 byte strings reuse an ID.
The compiler, not this validation layer, assigns first-use IDs in deterministic
canonical traversal order. The metadata constructor preserves the supplied
order and rejects duplicates rather than silently renumbering references.
Requirement aliases must be strictly increasing by numeric SymbolId. Distinct
aliases may name the same host requirement; this relationship is not discarded.
Compiler declaration canonicalization and sorting the final requirement IDs
must not reorder the payload table or object fields.

## Invariants, resources, and effects

- Symbol count cannot exceed existing `MAX_VALUES_PER_RUN` (1,000,000), because
  each symbol must eventually occupy an arena entry. This is an outer structural
  ceiling, not a promise that a run's smaller admitted arena budget fits.
- Each UTF-8 payload respects existing `MAX_SYMBOL_BYTES_PER_VALUE` (4,096).
  Empty text is legal. Empty requirement names are not.
- Conversion of every index and the count to u32 is checked. Byte accounting
  uses checked u64 arithmetic. No new aggregate-byte acceptance policy exists.
- Requirement count cannot exceed symbol count; every requirement resolves to
  two nonempty payloads; alias IDs are strictly increasing, so duplicate aliases
  are rejected. Requirement-name repetition across different aliases is legal.
- Duplicate-symbol validation allocates one fallibly reserved, bounded scratch
  vector of borrowed references and IDs. Sorting that scratch does not modify
  payload order. Work is O(n log n) comparisons with bounded string lengths;
  payload inspection and requirements validation are linear. No recursion,
  async work, IO, global state, secret provider, or hashing side effects occur.
- Ownership transfers already allocated buffers. The constructor is not a
  bounded decoder: a future wire boundary must validate envelope/count/length
  limits before allocating those buffers. Cached count/byte measurements are
  derived data, not a proposed wire representation.
- Source metadata is cold, immutable workflow-lifetime storage. There is no
  new allocation or validation in an engine transition. No benchmark or
  zero-cost claim is made.

## Secret-model decision reserved to Master

Master sections 8 and 66 support named requirements and presence-only admission;
sections 47/48 and supporting secret-model documentation need an explicit
cross-layer decision before expression value access can be supported.
This slice stores both names only, preserving `SecretField.key` and `.value`
from `vb_yaml/src/yaml_ast/types/fields.rs`; the latter is a declared environment
requirement name, never the environment value. The existing canonical source
digest already includes both fields. It does not imply a host input slot, capability
grant, provider lookup, fabricated value, authorization receipt, or opaque
handle. `secrets.*` value expressions must remain explicitly unsupported.

Recommended later decision: admission asks the authorized host only whether
each named requirement is available; any runtime use is an action-scoped,
opaque, nonserializable handle resolved only by authorized action workers.
An ordinary input slot is not an acceptable substitute: existing input seeding
marks it clean, and there is no approved secret receipt/recovery lifecycle.
Master must define accepted provider-reference syntax and interpretation,
capability/action scope, missing-secret behavior, taint,
revocation, and retry/recovery semantics before approving those paths.

## Coordinated integration contract, not implemented here

1. Add a separately approved metadata field to WorkflowParts and
   CompiledWorkflow, validate its count against symbols_count, preserve it in
   all construction/conversion routes, and migrate the attached inventory.
   Existing fixtures with nonzero symbol counts need real declared payloads or
   explicit negative-test intent; do not fabricate opaque defaults.
2. Extend the existing expression reference resolver with one fallible symbol
   interning seam, so text and accessor/object lowering share the table.
   Freeze the table only after lowering, then attach it in compiler part_01.
3. Version the artifact wire/evidence contract explicitly. Appending a serde
   default field to positional Postcard is not a compatibility strategy.
   Retain an explicit old-schema decoder or reject old artifacts with a typed
   migration-required outcome. Do not silently treat counts as empty payloads.
4. Canonical artifact bytes must include each payload in ID order with length
   framing, and each (alias, requirement) pair in strict alias order. Hash the zeroed-digest
   canonical parts at every compiler/storage verifier entry point. Include
   the new validation module in the verifier implementation fingerprint.
   Source identity and artifact identity remain separate existing concepts.
5. Seed an empty run ValueStore with the entire compiled table in ID order,
   before any externally supplied or action-produced values. Verify returned
   IDs. Reject insufficient admitted arena capacity before exposing the run.
   Existing nonempty stores require verified exact prefix identity, never an
   unchecked append or silent collision. All transferred structured values
   require bounded recursive handle remapping; source-only IDs are not enough.
6. Restored full checkpoints must validate the compiled prefix exactly and
   must not reseed duplicates. Noncheckpoint recovery must use the same fresh
   initialization path. Binding to the artifact digest prevents an unrelated
   table being substituted. Any future secret presence receipt requires its
   own approved versioned admission/recovery contract, not a raw secret value.
7. Resolve symbol equality explicitly before claiming end-to-end text support:
   the current store does not deduplicate, while equality compares handles.
   External text must either be interned/remapped canonically or compared by
   bytes through an approved checked path. Object keys have the same identity
   concern. This slice chooses neither runtime policy.

## Traceability and acceptance

Relevant architecture: master sections 11 (cold metadata and numeric runtime),
48 (arena-local handles), 51 (deterministic ordering), 66 (presence-only secrets),
and 77.17/77.18 (compatibility and resource budgets). Existing hard limits are
unchanged. Adjacent `test-plan.md` separates module acceptance from later
source-to-runtime acceptance. `migration-manifest.md` inventories exact base
locations and distinguishes textual hits from actual constructors.

No proof closure, green test, runtime integration, persistence compatibility,
performance result, or F1 completion follows from this draft alone.
