# Staged owned input wire proposal

Status: D2 proposal for coordination; no approved public API, wire tag, limit value or implementation is claimed. The existing positional postcard Box<[SlotValue]> wire is a separate legacy format. Select the new owned format through an explicit versioned command/discriminant; never attempt one decoder and silently fall back to another.

## Bounded representation

Proposed transport aggregate: version, ordered node table, ordered supplied-root prefix. Root order follows declared input order. A shorter prefix preserves current omission behavior; it does not make arbitrary holes expressible. Omitted suffix ports remain uninitialized. Null is an explicit node and must satisfy the declared kind; an empty collection is a collection node with no children.

Each node is one of Null, Bool, I64, finite F64, UTF-8 Text, List of InputNodeIdx, ordered Object fields of text-key/index pairs, or Blob bytes. InputNodeIdx is distinct from SlotIdx and every runtime SymbolId/ListId/ObjectId/BlobId. Every edge must reference an earlier node. A bounded iterative pass validates indices, derives maximum depth with checked arithmetic, and rejects forward edges/cycles; no recursive graph traversal is required. Reject unreachable nodes to avoid unbounded hidden allocation and ambiguous unused payload content.

Check encoded bytes before decoding allocation and enforce explicit profile budgets for aggregate nodes, edges, text/key/blob bytes, depth, roots and destination-store capacity. Postcard decoding must itself respect these bounds; a post-decode size check does not protect allocation. Do not invent numeric defaults for unspecified budgets. FiniteF64 construction rejects NaN and infinities. Declared number remains I64 under the current schema contract; adding F64 to the owned representation does not widen that schema.

Materialize validated nodes in table order into a private destination ValueStore, preserving list order and object field order. value_store.rs:134-150 retains object fields in insertion order and uses first-key-wins for both value and taint lookup. Preserve that behavior for repeated keys; do not sort object fields or silently change duplicate-key semantics. Runtime handles arise only from successful insertion into this destination store. Fallible allocation/budget errors discard the prepared store and roots before admission or frame mutation.

## Taint and symbol authority

Proposed public unauthenticated payload carries no caller-selected taint; materialized roots and nested values start Clean, matching current ordinary input seeding. Any trusted secret/configuration ingress requires a separate private authenticated constructor that preserves authorized taint for nodes/fields/roots. Coordination must approve the authority split before implementing it; a public wire field cannot grant secret provenance.

Owned Text and object keys carry actual text and are interned/materialized into the destination's symbols. A raw legacy SymbolId is not text backing. Legacy valid-symbol behavior must be checked against the actual run's preloaded workflow symbols and must not be extended merely by checking symbols_count. Dangling list/object/blob handles continue to reject; migration never treats an arbitrary small handle as an owned payload.

## Durable identity and recovery

Bind an owned-input receipt to the explicit wire version, validated payload content/order, declared numeric root slots, run and accepted artifact. Payload-content hashing is essential: two stores can assign the same ListId to different lists. Canonical node numbering must be defined before claiming content-equivalent alternative graphs have identical digests; this proposal only requires an exact validated representation to rehash consistently. Object/list order and duplicate behavior remain part of that representation.

Persist validated payload bytes and root mapping atomically with the artifact and acceptance receipt so reopening can verify and rebuild the same values without original YAML/input files. Allocate store backing before driving the recovered frame. For an already executed/snapshotted run, preserve restored state and replay only the proper journal suffix; initial input materialization must not overwrite execution results.

Existing admission candidate9f0557 hashes canonical typed slot/value pairs. Extending this to owned payload content requires coordinated versioned storage/receipt semantics; do not silently reinterpret RunInputs record kind4, legacy admission tag24 or bound admission tag36. Tag36 is occupied. Original input identity, artifact/semantic-source identity and retry checkpoint/frame identity remain distinct. No new tag is reserved by this analysis.

## Staging boundaries

Compiler consumer binding and unsupported-key diagnostics can progress after CI ownership coordination without claiming owned collections work. A second coordinated change owns the new bounded transport and ValueStore preparation. A third coordinated storage/runtime integration owns retained payload receipts and real Runtime recovery. These are domain boundaries for coordinator assignment, not extra markdown task tracking; vb-3ruqy remains the live tracker until coordination creates or assigns implementation children.
