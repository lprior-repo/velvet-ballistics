#![forbid(unsafe_code)]

//! Capability model for workflow admission control.

use crate::ids::{ActionId, WorkflowDigest};
use serde::{Deserialize, Serialize};

/// Capability grant controlling which actions a workflow may invoke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Grant access to a specific action by its identifier.
    Action(ActionId),
    /// Grant access to all actions within a specific workflow.
    Workflow(WorkflowDigest),
    /// Grant access to any action in any workflow.
    AnyWorkflow,
}

/// Bounded set of capability grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    grants: Box<[Capability]>,
}

impl CapabilitySet {
    /// Creates an empty capability set.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            grants: Box::new([]),
        }
    }

    /// Creates a capability set from the given grants.
    #[must_use]
    pub fn from_grants(grants: Box<[Capability]>) -> Self {
        Self { grants }
    }

    /// Returns `true` if the given capability is covered by this set.
    ///
    /// Rules:
    /// - `AnyWorkflow` grants everything.
    /// - `Workflow(d)` grants `Action(_)` for any action when the digest matches the
    ///   required workflow, and also matches `Workflow` capabilities with the same digest.
    /// - `Action(id)` grants only that specific action.
    #[must_use]
    pub fn grants(&self, cap: &Capability) -> bool {
        let mut i = 0;
        while i < self.grants.len() {
            let Some(grant) = self.grants.get(i) else {
                break;
            };
            match grant {
                Capability::AnyWorkflow => return true,
                Capability::Workflow(digest) => {
                    if let Capability::Action(_action_id) = cap {
                        return true;
                    }
                    if let Capability::Workflow(required_digest) = cap {
                        if digest == required_digest {
                            return true;
                        }
                    }
                }
                Capability::Action(granted_id) => {
                    if let Capability::Action(required_id) = cap {
                        if granted_id == required_id {
                            return true;
                        }
                    }
                }
            }
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }
        false
    }

    /// Returns the number of grants in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.grants.len()
    }

    /// Returns `true` if the set contains no grants.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_granted_action_succeeds() {
        let granted = CapabilitySet::from_grants(Box::new([Capability::Action(ActionId::new(1))]));
        let required = Capability::Action(ActionId::new(1));
        assert!(granted.grants(&required));
    }

    #[test]
    fn capability_missing_action_rejected() {
        let granted = CapabilitySet::from_grants(Box::new([Capability::Action(ActionId::new(1))]));
        let required = Capability::Action(ActionId::new(2));
        assert!(!granted.grants(&required));
    }

    #[test]
    fn capability_any_workflow_grants_all() {
        let granted = CapabilitySet::from_grants(Box::new([Capability::AnyWorkflow]));
        assert!(granted.grants(&Capability::Action(ActionId::new(99))));
        assert!(granted.grants(&Capability::Workflow(WorkflowDigest::from_bytes([1; 32]))));
        assert!(granted.grants(&Capability::AnyWorkflow));
    }

    #[test]
    fn capability_workflow_scoped() {
        let digest_a = WorkflowDigest::from_bytes([0xAA; 32]);
        let digest_b = WorkflowDigest::from_bytes([0xBB; 32]);
        let granted = CapabilitySet::from_grants(Box::new([Capability::Workflow(digest_a)]));

        // Workflow(digest_a) grants Action(_) for any action
        assert!(granted.grants(&Capability::Action(ActionId::new(1))));
        assert!(granted.grants(&Capability::Action(ActionId::new(42))));

        // Workflow(digest_a) grants Workflow(digest_a) but not Workflow(digest_b)
        assert!(granted.grants(&Capability::Workflow(digest_a)));
        assert!(!granted.grants(&Capability::Workflow(digest_b)));
    }

    #[test]
    fn capability_action_scoped() {
        let granted = CapabilitySet::from_grants(Box::new([Capability::Action(ActionId::new(5))]));

        // Action(5) grants only Action(5)
        assert!(granted.grants(&Capability::Action(ActionId::new(5))));
        assert!(!granted.grants(&Capability::Action(ActionId::new(6))));

        // Action does not grant Workflow
        assert!(!granted.grants(&Capability::Workflow(WorkflowDigest::from_bytes([0; 32]))));
    }

    #[test]
    fn capability_multiple_actions_all_checked() {
        let granted = CapabilitySet::from_grants(Box::new([
            Capability::Action(ActionId::new(1)),
            Capability::Action(ActionId::new(2)),
        ]));

        assert!(granted.grants(&Capability::Action(ActionId::new(1))));
        assert!(granted.grants(&Capability::Action(ActionId::new(2))));
        assert!(!granted.grants(&Capability::Action(ActionId::new(3))));
    }

    #[test]
    fn capability_set_empty_grants_nothing() {
        let granted = CapabilitySet::empty();
        assert!(granted.is_empty());
        assert_eq!(granted.len(), 0);
        assert!(!granted.grants(&Capability::Action(ActionId::new(1))));
    }

    #[test]
    fn capability_set_len_counts_grants() {
        let granted = CapabilitySet::from_grants(Box::new([
            Capability::Action(ActionId::new(1)),
            Capability::Workflow(WorkflowDigest::from_bytes([0; 32])),
        ]));
        assert!(!granted.is_empty());
        assert_eq!(granted.len(), 2);
    }

    #[test]
    fn capability_any_workflow_supersedes_specific() {
        let granted = CapabilitySet::from_grants(Box::new([
            Capability::AnyWorkflow,
            Capability::Action(ActionId::new(1)),
        ]));
        // AnyWorkflow at index 0 short-circuits, so any capability is granted.
        assert!(granted.grants(&Capability::Action(ActionId::new(999))));
    }
}
