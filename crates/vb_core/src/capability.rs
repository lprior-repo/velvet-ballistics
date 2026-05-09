#![forbid(unsafe_code)]

//! Capability model for workflow admission control.

use crate::ids::ActionId;
use serde::{Deserialize, Serialize};

/// Capability grant controlling which actions a workflow may invoke.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    pub name: Box<str>,
    pub action: ActionId,
}

impl Capability {
    pub fn new(name: Box<str>, action: ActionId) -> Self {
        Self { name, action }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn action_id(&self) -> ActionId {
        self.action
    }
}

/// Bounded set of capability grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    grants: Vec<Capability>,
}

impl CapabilitySet {
    pub fn empty() -> Self {
        Self { grants: Vec::new() }
    }

    pub fn from_grants(grants: Box<[Capability]>) -> Self {
        Self {
            grants: grants.into_vec(),
        }
    }

    pub fn grants(&self, required: &Capability) -> bool {
        let mut i = 0;
        while i < self.grants.len() {
            if let Some(grant) = self.grants.get(i) {
                if grant.name().is_empty() {
                    i = match i.checked_add(1) {
                        Some(next) => next,
                        None => break,
                    };
                    continue;
                }
                let name_match = capability_name_grants(grant.name(), required.name());
                if name_match && grant.action == required.action {
                    return true;
                }
            }
            i = match i.checked_add(1) {
                Some(next) => next,
                None => break,
            }
        }
        false
    }

    pub fn len(&self) -> usize {
        self.grants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }
}

fn capability_name_grants(grant_name: &str, required_name: &str) -> bool {
    if grant_name.is_empty() {
        return false;
    }
    if required_name == grant_name {
        return true;
    }
    match required_name.strip_prefix(grant_name) {
        Some(suffix) => suffix.starts_with('.'),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(name: &str, action: ActionId) -> Capability {
        Capability::new(name.into(), action)
    }

    #[test]
    fn capability_new_and_accessors() {
        let c = cap("network", ActionId::new(1));
        assert_eq!(c.name(), "network");
        assert_eq!(c.action_id(), ActionId::new(1));
    }

    #[test]
    fn capability_clone_is_equal() {
        let a = cap("secrets.read", ActionId::new(5));
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn capability_set_empty_grants_nothing() {
        let set = CapabilitySet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert!(!set.grants(&cap("network", ActionId::new(1))));
    }

    #[test]
    fn capability_set_from_slice() {
        let caps = Box::new([
            cap("network", ActionId::new(1)),
            cap("secrets.read", ActionId::new(2)),
        ]);
        let set = CapabilitySet::from_grants(caps);
        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());
    }

    #[test]
    fn capability_set_grants_exact_name() {
        let caps = Box::new([cap("network", ActionId::new(1))]);
        let set = CapabilitySet::from_grants(caps);
        assert!(set.grants(&cap("network", ActionId::new(1))));
        assert!(!set.grants(&cap("secrets", ActionId::new(1))));
        assert!(!set.grants(&cap("network", ActionId::new(2))));
    }

    #[test]
    fn capability_set_grants_hierarchical_prefix() {
        let caps = Box::new([cap("network", ActionId::new(1))]);
        let set = CapabilitySet::from_grants(caps);
        assert!(set.grants(&cap("network.github", ActionId::new(1))));
        assert!(set.grants(&cap("network.http", ActionId::new(1))));
        assert!(!set.grants(&cap("secrets.network", ActionId::new(1))));
    }

    #[test]
    fn capability_set_does_not_grant_short_partial_prefix() {
        // Given a grant whose name is only a lexical prefix of the required capability.
        let short = CapabilitySet::from_grants(Box::new([cap("net", ActionId::new(1))]));

        // When checking a dotted child capability under a different root.
        // Then the partial prefix must not grant access.
        assert!(!short.grants(&cap("network.github", ActionId::new(1))));
    }

    #[test]
    fn capability_set_does_not_grant_sibling_partial_prefix() {
        // Given a grant whose name is a sibling lexical prefix, not a hierarchy parent.
        let sibling = CapabilitySet::from_grants(Box::new([cap("networking", ActionId::new(1))]));

        // When checking a dotted child of the network hierarchy.
        // Then the sibling prefix must not grant access.
        assert!(!sibling.grants(&cap("network.github", ActionId::new(1))));
    }

    #[test]
    fn capability_set_grants_requires_action_match() {
        let caps = Box::new([cap("network", ActionId::new(1))]);
        let set = CapabilitySet::from_grants(caps);
        assert!(!set.grants(&cap("network", ActionId::new(2))));
        assert!(!set.grants(&cap("network.github", ActionId::new(2))));
    }

    #[test]
    fn capability_set_multiple_caps_checked() {
        let caps = Box::new([
            cap("network", ActionId::new(1)),
            cap("secrets", ActionId::new(2)),
        ]);
        let set = CapabilitySet::from_grants(caps);
        assert!(set.grants(&cap("network", ActionId::new(1))));
        assert!(set.grants(&cap("secrets", ActionId::new(2))));
        assert!(set.grants(&cap("network.github", ActionId::new(1))));
        assert!(set.grants(&cap("secrets.read", ActionId::new(2))));
    }

    #[test]
    fn capability_set_empty_name_grants_nothing() {
        let caps = Box::new([cap("", ActionId::new(1))]);
        let set = CapabilitySet::from_grants(caps);
        assert!(!set.grants(&cap("network", ActionId::new(1))));
        assert!(!set.grants(&cap("", ActionId::new(1))));
    }
}
