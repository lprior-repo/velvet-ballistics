#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingQuotaReservation {
    pub(crate) tenant: Option<vb_core::ids::TenantId>,
    pub(crate) cost_units: u64,
}

pub(crate) struct QueuedShardCommand {
    pub(crate) command: ShardCommand,
    pub(crate) quota: Option<PendingQuotaReservation>,
}
