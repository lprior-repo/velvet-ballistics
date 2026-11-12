# Workflow feature integration contract

Bead: vb-eirmk.13.13.3.4.16. Authority: velvet-ballistics-MASTER.md and the user-authorized main landing.

## Pinned inputs

- Core: 1e3bf19597ad0c6d3574387ba3f2ea382892c9e0, including reviewed human commit 6a32e0b090503217991978baec76e2f42d5751a2 and guarded upstream test relocations.
- YAML: 75bae4eb2ab6291630d5891b038ac3fa812aa6dd. Its later two-attribute IPC performance candidate and final remote-main successor must be incorporated before final acceptance.
- Components have separate evidence; no component pass qualifies their combined tree. The YAML landing precedes the core main push.

## Required behavior

Retain both existing feature sets: typed source declarations/literals and bound inputs, scalar and list expression semantics, authored control-flow lowering, runtime admission and scheduling, durable input recovery/checkpoint/replay, IPC boundaries, and generated execution. Preserve their existing public call boundaries, wire discriminants, persisted records, capability checks and typed error behavior. No new dependency, public API or stored format is authorized by this merge.

Compilation remains cold. Unknown primitives and invalid shapes fail closed. Checked widths, nested scopes, source/digest identity, generic and typed lowering, and 64/65 boundary behavior remain intact. Hot runtime functions retain the master25-line limit; the existing documented cold compiler justification is preserved where applicable.

Runtime outcomes, resource counters, journal-before-dispatch ordering, attempt/version fences, taint propagation and replay equivalence must agree with both retained test suites. A source compatibility fix must not replace real inputs with dummy or empty data.

## Preservation boundaries

- Resolve semantic hunks; never select one entire branch as a substitute for integration. Read both pinned versions and callers before edits.
- Preserve every executable test/property, strong assertion, fixture class and tracked regression seed. Document module/name relocation explicitly. The helper property keeps its explicit existing-seed persistence binding.
- Keep the eleven-member virtual workspace. Preserve all reference-model cases and both added lint-test files through any canonical directory decision. Master reference-location requirements take precedence unless an explicit authorized relocation is established.
- Preserve every mathematical obligation and trust boundary. Kani generators stay symbolic and exhaustive over the stated bounded domain; no hardcoded structural shapes. Verus mirrors must bind to the actual final production source, with honest drift/locator metadata. No new assume/admit/external-body/trusted escape.
- Panic/result/source checks retain both existing detection contracts and negative fixtures. Any necessary protected resolution is supplied as a reviewed proposal; no weakening, suppression, baseline change, skipped case or timeout increase.

## Isolated ownership

Each author receives an exclusive Git/JJ workspace, assigned source paths, preimage hashes and the two input pins. Unassigned conflicts remain untouched. No main-checkout changes, Git history rewriting, push or global configuration changes. Parent coordinates tracker state and final publication. Local GPU authors use Holzman Rust and Scott DDD with current rust-governance; no sweeping functional rewrite or Go/full proof-planning stage.

Partial authors report postimages and source/command evidence without claiming compilation while unrelated conflicts remain. Parent combines only hash-fenced assigned outputs, then runs actual behavior verification and independent CODE/TEST review.

## Acceptance

Required: no unresolved conflicts or generated conflict sidecars; source/test/seed preservation audit; meaningful mixed-feature public-API behavior cases; unchanged gate strength; actual full native and canonical Moon CI on the final frozen candidate; required formal and performance evidence under unchanged acceptance limits; a normal main push verified against the remote SHA. The CI environment includes actual CUE and pinned Rust/verifier tooling. Failed or incomplete runs remain preserved and unaccepted.
