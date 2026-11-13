//! Active group guard that exposes GroupControl through a readonly Deref.
//! The guard prevents std::mem::replace on the canonical owner while still
//! allowing inspection and cloning of the control block.

use super::GroupControl;
use crate::execution_lanes::{
    AbortReason, GroupError, LaneCompletion, LaneToken, PreparedGroupEvent,
};

/// Readonly view over an active group's control block.
///
/// Ownership contract: `ActiveGroup` is constructed by the parent storage and
/// provides access to the canonical `GroupControl`. It intentionally lacks
/// `DerefMut`, `AsMut`, mutable borrow getters, raw owner returns, and owned
/// construction so callers cannot replace or move the control block.
///
/// ```compile_fail,E0596
/// use vb_core::execution_lanes::{ActiveGroup, GroupControl};
/// fn replace(mut active: ActiveGroup<'_>, replacement: GroupControl) {
///     let _old = std::mem::replace(&mut *active, replacement);
/// }
/// ```
pub struct ActiveGroup<'a> {
    control: &'a mut GroupControl,
}

impl<'a> ActiveGroup<'a> {
    /// Construct from the owning storage (parent only).
    pub(super) fn new(control: &'a mut GroupControl) -> Self {
        Self { control }
    }

    /// Prepare admission of a new lane into the group.
    pub fn prepare_admit(&mut self) -> Result<PreparedGroupEvent<'_>, GroupError> {
        self.control.prepare_admit()
    }

    /// Prepare completion of a lane with its outcome.
    pub fn prepare_completion(
        &mut self,
        token: LaneToken,
        outcome: LaneCompletion,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        self.control.prepare_completion(token, outcome)
    }

    /// Prepare draining of a lane from the group.
    pub fn prepare_drain(
        &mut self,
        token: LaneToken,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        self.control.prepare_drain(token)
    }

    /// Prepare aborting the entire group with a reason.
    pub fn prepare_abort(
        &mut self,
        reason: AbortReason,
    ) -> Result<PreparedGroupEvent<'_>, GroupError> {
        self.control.prepare_abort(reason)
    }
}

// Readonly deref for inspection and cloning; no DerefMut.
impl core::ops::Deref for ActiveGroup<'_> {
    type Target = GroupControl;

    fn deref(&self) -> &Self::Target {
        self.control
    }
}
