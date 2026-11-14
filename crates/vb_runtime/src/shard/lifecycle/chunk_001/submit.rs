/// Submission header options shared by every submit wrapper.
#[derive(Debug, Clone, Copy)]
struct SubmitOptions {
    tenant: Option<vb_core::ids::TenantId>,
    lifetime_cap: Option<std::num::NonZeroU64>,
    persist_header: bool,
}

impl SubmitOptions {
    const fn persist(tenant: Option<vb_core::ids::TenantId>) -> Self {
        Self {
            tenant,
            lifetime_cap: None,
            persist_header: true,
        }
    }

    const fn pre_persisted() -> Self {
        Self {
            tenant: None,
            lifetime_cap: None,
            persist_header: false,
        }
    }
}

mod submit_phases {
    include!("submit_phases.rs");
}

impl Shard {
    /// Returns true when the journal is explicitly Volatile with no storage backend.
    /// Only that profile may skip retained-input admission for RunId-zero compatibility.
    fn is_volatile_admission(&self) -> bool {
        self.journal.durability_profile() == Some(vb_storage::DurabilityProfile::Volatile)
            && self.journal.storage_journal().is_none()
    }
}
