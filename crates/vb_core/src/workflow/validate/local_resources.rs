use super::super::resource_limits::check_actual_within_declared as within;
use super::super::{CompiledNodeKind as K, ContractResource as R, ResourceContract, WorkflowError};

pub(super) fn validate(kind: &K, contract: &ResourceContract) -> Result<(), WorkflowError> {
    match kind {
        K::ForEachStart { limit, .. } | K::ForEachConcurrentStart { limit, .. } => {
            within(R::CollectItems, limit.get(), contract.max_collect_items)
        }
        K::CollectStart {
            limit, page_size, ..
        } => {
            within(R::CollectItems, limit.get(), contract.max_collect_items)?;
            within(R::CollectItems, *page_size, contract.max_collect_items)
        }
        K::TogetherStart { branches, .. } => {
            within(R::Fanout, branches.len(), usize::from(contract.max_fanout))
        }
        K::TogetherJoin { branch_count, .. } => {
            within(R::Fanout, branch_count.get(), contract.max_fanout)
        }
        _ => Ok(()),
    }
}
