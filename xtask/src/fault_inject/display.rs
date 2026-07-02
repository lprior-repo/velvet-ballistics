#![forbid(unsafe_code)]

//! Display implementations for fault injection types.

use std::fmt;

use crate::fault_inject::report::FaultReport;

impl fmt::Display for FaultReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "FaultReport {{ seed={}, events_applied={}, runtime_steps={}, recovery_required={}, schedule_hash={:#018x} }}",
            self.seed,
            self.events_applied,
            self.runtime_steps,
            self.recovery_required,
            self.schedule_hash,
        )?;
        writeln!(f, "  outcomes:")?;
        for (idx, outcome) in self.outcomes.iter().enumerate() {
            writeln!(f, "    [{idx}] {outcome:?}")?;
        }
        writeln!(f, "  journal:")?;
        for entry in &self.journal_entries {
            writeln!(f, "    {entry:?}")?;
        }
        Ok(())
    }
}
