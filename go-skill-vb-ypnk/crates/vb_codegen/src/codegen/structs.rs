use vb_core::{SlotIdx, StepIdx};

#[derive(Clone, Copy)]
pub struct ForEachStartEmit {
    pub step: StepIdx,
    pub input: SlotIdx,
    pub item_slot: SlotIdx,
    pub limit: u32,
    pub body: StepIdx,
    pub done: StepIdx,
    pub output: Option<SlotIdx>,
}

#[derive(Default)]
pub struct ListStoreMetrics {
    pub build_list_count: usize,
    pub total_build_list_items: usize,
    pub for_each_steps: usize,
}

#[derive(Default)]
pub struct ValueStoreMetrics {
    pub build_object_count: usize,
    pub total_build_object_fields: usize,
}

#[derive(Default)]
pub struct ExpressionStoreMetrics {
    pub list_allocating_ops: usize,
    pub object_allocating_ops: usize,
}
