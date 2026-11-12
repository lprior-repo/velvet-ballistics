#![forbid(unsafe_code)]
//! Remaining-budget counters for one admitted run.
//!
//! `ReleaseAboveLimit` carries its resource as `&'static str` by decision
//! (policy-driven dimension keys; see the open-registry rationale in
//! `crate::errors`).

use super::AggregateResourceBudget;
use thiserror::Error;

/// Outcome of a remaining-budget exhaustion, for the caller's policy to act
/// on. Releasable dimensions (open gates, signal subscriptions) suspend:
/// a slot frees when a gate closes or a subscription drops, so the run can
/// resume. Monotonic dimensions (transitions, journal bytes, value-store
/// entries, incidents, cost) never replenish within a run, so exhaustion
/// fails. Callers may override this default mapping; the error stays typed
/// either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetExhaustionOutcome {
    Suspend,
    Fail,
}

/// Per-dimension remaining-budget exhaustion for one admitted run.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemainingBudgetError {
    #[error("open gates exhausted")]
    OpenGatesExhausted,
    #[error("signal subscriptions exhausted")]
    SignalSubscriptionsExhausted,
    #[error("incident emissions exhausted")]
    IncidentEmissionsExhausted,
    #[error("transitions exhausted")]
    TransitionsExhausted,
    #[error("journal bytes exhausted: requested {requested}, remaining {remaining}")]
    JournalBytesExhausted { requested: u64, remaining: u64 },
    #[error("value-store entries exhausted: requested {requested}, remaining {remaining}")]
    ValueStoreEntriesExhausted { requested: u64, remaining: u64 },
    #[error("cost units exhausted: requested {requested}, remaining {remaining}")]
    CostUnitsExhausted { requested: u64, remaining: u64 },
    #[error("release above admitted limit: {resource}")]
    ReleaseAboveLimit { resource: &'static str },
}

impl RemainingBudgetError {
    /// Default suspension/failure mapping for this exhaustion.
    #[must_use]
    pub const fn outcome(&self) -> BudgetExhaustionOutcome {
        match self {
            Self::OpenGatesExhausted | Self::SignalSubscriptionsExhausted => {
                BudgetExhaustionOutcome::Suspend
            }
            _ => BudgetExhaustionOutcome::Fail,
        }
    }
}

/// Remaining-budget counters for one admitted run, covering the §77.18
/// dimensions added by this batch. Created from the admitted aggregate
/// budget; every consume is checked, never saturating or truncating.
/// Gates and signal subscriptions are releasable (close/unsubscribe);
/// transitions, journal bytes, value-store entries, incidents, and cost
/// are monotonic within a run. Whole-run release is shard-level
/// accounting via `AggregateResourceUsage::try_subtract_budget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemainingBudget {
    limits: AggregateResourceBudget,
    remaining_open_gates: u32,
    remaining_signal_subscriptions: u32,
    remaining_incident_emissions: u64,
    remaining_transitions: u64,
    remaining_journal_bytes_total: u64,
    remaining_value_store_entries: u64,
    remaining_cost_units: u64,
}

impl RemainingBudget {
    /// Creates counters with every dimension fully remaining.
    #[must_use]
    pub const fn from_budget(budget: &AggregateResourceBudget) -> Self {
        Self {
            limits: *budget,
            remaining_open_gates: budget.max_open_gates,
            remaining_signal_subscriptions: budget.max_signal_subscriptions,
            remaining_incident_emissions: budget.max_incident_emissions,
            remaining_transitions: budget.max_transitions,
            remaining_journal_bytes_total: budget.max_journal_bytes_total,
            remaining_value_store_entries: budget.max_value_store_entries,
            remaining_cost_units: budget.max_cost_units,
        }
    }

    /// The admitted budget these counters enforce.
    #[must_use]
    pub const fn limits(&self) -> AggregateResourceBudget {
        self.limits
    }

    /// Remaining open-gate slots.
    #[must_use]
    pub const fn remaining_open_gates(&self) -> u32 {
        self.remaining_open_gates
    }

    /// Remaining signal-subscription slots.
    #[must_use]
    pub const fn remaining_signal_subscriptions(&self) -> u32 {
        self.remaining_signal_subscriptions
    }

    /// Remaining incident emissions.
    #[must_use]
    pub const fn remaining_incident_emissions(&self) -> u64 {
        self.remaining_incident_emissions
    }

    /// Remaining total transitions.
    #[must_use]
    pub const fn remaining_transitions(&self) -> u64 {
        self.remaining_transitions
    }

    /// Remaining cumulative journal bytes.
    #[must_use]
    pub const fn remaining_journal_bytes_total(&self) -> u64 {
        self.remaining_journal_bytes_total
    }

    /// Remaining value-store entries.
    #[must_use]
    pub const fn remaining_value_store_entries(&self) -> u64 {
        self.remaining_value_store_entries
    }

    /// Remaining cost units.
    #[must_use]
    pub const fn remaining_cost_units(&self) -> u64 {
        self.remaining_cost_units
    }

    /// Opens one gate, consuming one open-gate slot.
    pub fn try_open_gate(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_open_gates.checked_sub(1) {
            Some(next) => {
                self.remaining_open_gates = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::OpenGatesExhausted),
        }
    }

    /// Closes one gate, releasing its slot. Releasing past the admitted
    /// limit is a caller bug, not silent replenishment.
    pub fn release_gate(&mut self) -> Result<(), RemainingBudgetError> {
        if self.remaining_open_gates >= self.limits.max_open_gates {
            return Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_open_gates",
            });
        }
        match self.remaining_open_gates.checked_add(1) {
            Some(next) => {
                self.remaining_open_gates = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_open_gates",
            }),
        }
    }

    /// Subscribes to one signal, consuming one subscription slot.
    pub fn try_subscribe_signal(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_signal_subscriptions.checked_sub(1) {
            Some(next) => {
                self.remaining_signal_subscriptions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::SignalSubscriptionsExhausted),
        }
    }

    /// Drops one signal subscription, releasing its slot.
    pub fn release_signal(&mut self) -> Result<(), RemainingBudgetError> {
        if self.remaining_signal_subscriptions >= self.limits.max_signal_subscriptions {
            return Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_signal_subscriptions",
            });
        }
        match self.remaining_signal_subscriptions.checked_add(1) {
            Some(next) => {
                self.remaining_signal_subscriptions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_signal_subscriptions",
            }),
        }
    }

    /// Emits one incident, consuming one emission.
    pub fn try_emit_incident(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_incident_emissions.checked_sub(1) {
            Some(next) => {
                self.remaining_incident_emissions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::IncidentEmissionsExhausted),
        }
    }

    /// Consumes one total transition.
    pub fn try_consume_transition(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_transitions.checked_sub(1) {
            Some(next) => {
                self.remaining_transitions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::TransitionsExhausted),
        }
    }

    /// Appends `bytes` journal bytes. No partial append: an overrun
    /// consumes nothing and returns the typed exhaustion.
    pub fn try_append_journal_bytes(&mut self, bytes: u64) -> Result<(), RemainingBudgetError> {
        match self.remaining_journal_bytes_total.checked_sub(bytes) {
            Some(next) => {
                self.remaining_journal_bytes_total = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::JournalBytesExhausted {
                requested: bytes,
                remaining: self.remaining_journal_bytes_total,
            }),
        }
    }

    /// Creates `count` value-store entries.
    pub fn try_create_value_store_entries(
        &mut self,
        count: u64,
    ) -> Result<(), RemainingBudgetError> {
        match self.remaining_value_store_entries.checked_sub(count) {
            Some(next) => {
                self.remaining_value_store_entries = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::ValueStoreEntriesExhausted {
                requested: count,
                remaining: self.remaining_value_store_entries,
            }),
        }
    }

    /// Spends `units` cost units.
    pub fn try_spend_cost_units(&mut self, units: u64) -> Result<(), RemainingBudgetError> {
        match self.remaining_cost_units.checked_sub(units) {
            Some(next) => {
                self.remaining_cost_units = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::CostUnitsExhausted {
                requested: units,
                remaining: self.remaining_cost_units,
            }),
        }
    }
}
