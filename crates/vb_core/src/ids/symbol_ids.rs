//! Symbol identifiers used by the runtime symbol table: `SymbolId`, `ListId`,
//! and `ObjectId`.

#![forbid(unsafe_code)]

use crate::ids::macros::numeric_id;

numeric_id!(SymbolId, u32, get);
numeric_id!(ListId, u32, get);
numeric_id!(ObjectId, u32, get);
