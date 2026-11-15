use vb_core::{CompiledNodeKind as K, ResourceContract};

pub(super) fn include(contract: &mut ResourceContract, kind: &K) {
    let items = match kind {
        K::ForEachStart { limit, .. } | K::ForEachConcurrentStart { limit, .. } => limit.get(),
        K::CollectStart {
            limit, page_size, ..
        } => limit.get().max(*page_size),
        _ => 0,
    };
    contract.max_collect_items = contract.max_collect_items.max(items);
}
