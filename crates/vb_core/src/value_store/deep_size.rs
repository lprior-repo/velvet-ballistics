use crate::errors::{CoreError, CoreResult};
use crate::ids::{BlobId, ListId, ObjectId, SymbolId};
use crate::value::SlotValue;

use super::ValueStore;

impl ValueStore {
    /// Deep byte size of a value: payload bytes of every scalar and
    /// every arena entry reachable from it, traversing list and object
    /// handles instead of measuring the handles themselves. Scalars
    /// count as Null 0, Bool 1, I64 8, F64 8; a symbol counts its
    /// string bytes, a blob its payload bytes, and an object field its
    /// key bytes plus its value. Each handle is counted once, so a
    /// handle shared by several parents contributes once and a handle
    /// cycle terminates; traversal work is therefore bounded by the
    /// reachable arena contents rather than by the expanded tree,
    /// which sharing could blow up exponentially. An unresolvable
    /// handle or an overflowing sum is an error, never a silent
    /// undercount.
    pub fn deep_size(&self, value: &SlotValue) -> CoreResult<u64> {
        let mut total: u64 = 0;
        let mut visited_symbols: std::collections::HashSet<SymbolId> =
            std::collections::HashSet::new();
        let mut visited_lists: std::collections::HashSet<ListId> = std::collections::HashSet::new();
        let mut visited_objects: std::collections::HashSet<ObjectId> =
            std::collections::HashSet::new();
        let mut visited_blobs: std::collections::HashSet<BlobId> = std::collections::HashSet::new();
        let mut stack: Vec<SlotValue> = vec![*value];
        while let Some(current) = stack.pop() {
            match current {
                SlotValue::Null => {}
                SlotValue::Bool(_) => {
                    total = add_size(total, 1)?;
                }
                SlotValue::I64(_) | SlotValue::F64(_) => {
                    total = add_size(total, 8)?;
                }
                SlotValue::Symbol(id) => {
                    if visited_symbols.insert(id) {
                        let len = self.symbol(id)?.len();
                        total = add_size(total, len_to_u64(len)?)?;
                    }
                }
                SlotValue::Blob(id) => {
                    if visited_blobs.insert(id) {
                        let len = self.blob(id)?.len();
                        total = add_size(total, len_to_u64(len)?)?;
                    }
                }
                SlotValue::List(id) => {
                    if visited_lists.insert(id) {
                        for item in self.list(id)? {
                            stack.push(*item);
                        }
                    }
                }
                SlotValue::Object(id) => {
                    if visited_objects.insert(id) {
                        for field in self.object(id)? {
                            stack.push(SlotValue::Symbol(field.key));
                            stack.push(field.value);
                        }
                    }
                }
            }
        }
        Ok(total)
    }
}

fn add_size(total: u64, addend: u64) -> CoreResult<u64> {
    total
        .checked_add(addend)
        .ok_or(CoreError::ResourceLimitExceeded {
            resource: "result_bytes",
        })
}

fn len_to_u64(len: usize) -> CoreResult<u64> {
    u64::try_from(len).map_err(|_| CoreError::ResourceLimitExceeded {
        resource: "result_bytes",
    })
}
