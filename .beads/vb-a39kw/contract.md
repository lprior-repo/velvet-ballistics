# vb-a39kw Rust contract

CollectNext must preserve the current collector page Taint on every newly written page, including an empty terminal page. Read taint through checked RunFrame API before allocating/inserting the new list; use the existing explicit-taint writer. Clean remains Clean; Secret remains Secret. Empty-page acceptance that performs no write keeps existing state.

Pagination IDs, source/cursor/item counts, limits, ordering validation, replay identity and completion behavior remain the same. No schema/API expansion, synthesized default, new allocation or primitive branch duplication.

Acceptance enters the actual full driver using constructor-validated IR with Secret source values. Productive and exhausted Collect cases must verify actual page payload/taint and pagination state before exact ordered SlotWritten evidence. Keep independent eight-scenario family oracles unchanged. Require focused/full library gates and independent CODE/TEST review; focused pass is not F1/main completion.
