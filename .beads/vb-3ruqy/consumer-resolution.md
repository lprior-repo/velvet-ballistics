# Consumer resolution and allocation proposal

Analysis snapshot and implementation hold are those in contract.md. This is a domain design proposal; no compiler behavior has changed.

## Slot layout

Use two passes. First reserve the complete existing internal layout: step-index outputs, every nested body's outputs, explicit numeric operands that currently participate in slot sizing, and all compound scratch slots (current foreach and aggregate use slot1). Compute the checked high-water mark. Then append one input port per declaration in source order, checking profile and u16 limits, and freeze the input-name environment before lowering any consumer.

The first pass must inspect structure and operand shapes, not perform successful input-reference resolution before bindings exist. The second pass emits numeric IR against the frozen plan. This preserves existing numeric-slot compatibility while avoiding an input/output overlap caused by simply allocating inputs at slot0. Any future intentional overwrite needs an explicit language contract; input name binding alone does not authorize one.

The direct WorkflowParts.semantic_source_identity field and part_01 digest hunk belong to compiler CI. The allocation pass and integration point must be coordinated with that owner before source edits; this document does not assign ownership of their hunk.

## Consumer matrix

| Source consumer | Snapshot source / behavior | Required semantic resolution |
| --- | --- | --- |
| Finish result | finish_inputs.rs and part_05_utility.rs; named final-input binding already exists | Resolve direct input to its frozen SlotIdx; retain existing explicit numeric and step-output behavior. |
| Set value | part_02.rs; integer literal becomes SetConst, output uses step-index slot | Preserve literal SetConst. A direct reference requires value-copy semantics; an expression requires existing expression bytecode semantics. Do not treat an input name as a constant or invent a new expression grammar. |
| Do input | AST types/step.rs declares action/input; part_02 dispatch currently rejects Do | Master Do consumes a SlotIdx and authorized action contract. Supporting this reference requires actual Do lowering and registry/capability integration, not merely changing slot_from_text. Keep unsupported diagnostic until implemented. |
| ForEach input | part_02.rs; numeric-only, scratch slot1 | Resolve collection slot; production consumption requires List kind and backed data. Include body/scratch layout in first pass. |
| Collect source | part_03.rs; numeric-only | Resolve source slot under the existing collection node semantics; preserve collection/body ordering. |
| Aggregate input | part_04.rs; numeric-only, scratch slot1 | Resolve list source; preserve current numeric initialization grammar. Do not infer that an initial value string accepts references without a normative field contract. |
| Wait event and timeout | part_04.rs; numeric-only | Resolve each slot operand separately; preserve node-specific event/deadline numeric kind checks and optional timeout absence. |
| Ask prompt and timeout | part_04.rs; numeric-only | Resolve prompt and optional timeout slots; string payload backing and numeric timeout validation remain separate. |
| Choose branch when | part_14.rs; numeric-only | Preserve the distinction between a precomputed condition slot and a source expression. Compile supported expressions through the expression resolver; no prefix substitution masquerading as expression support. |
| Nested bodies/branches/repeat | recursive dispatch in lowering | Reuse the frozen environment at every depth, account for each body's slots, and preserve current branch/loop result semantics. |
| Accessor-bearing references | master section8; expression_bytecode.rs and vb_expr resolver surfaces | Only emit a supported numeric AccessorIdx after field/path validation and symbol-table ownership. Unsupported accessors reject precisely; no runtime string lookup. |

This matrix separates master-declared behavior from snapshot gaps. It does not authorize implementing unrelated Save behavior or inventing additional schema features. WaitUntil's numeric deadline consumption exists in core; expose it through source only according to the actual source primitive contract.

## Resolution boundary

A cold resolver distinguishes legacy numeric operand, declared input reference, named step output and supported expression/accessor. A recognized but invalid input reference never falls back to integer parsing. Unknown declaration, malformed reference, unsupported accessor, wrong operand kind and exhausted slot budget remain separate diagnostic categories.

Existing expression_bytecode.rs supports an ExpressionReferenceResolver producing ExprOp, while vb_expr exposes a ReferenceResolver producing SlotIdx. They are separate existing surfaces. Share the binding environment or adapt explicitly; do not assume either interface already handles every accessor or compiler-owned string symbol.

Longform input schema currently recognizes only is. Reject every unsupported key, including default, required and items, rather than silently accepting ignored metadata or inventing semantics. The precise new diagnostic API is coordinated separately; it must identify input declaration, unsupported key and source location without copying payload contents.
