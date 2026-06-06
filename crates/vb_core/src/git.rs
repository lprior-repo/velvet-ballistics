// vb-63st6.1: Git state value objects for proof lane helpers.
#![forbid(unsafe_code)]
//! Git state value objects for proof lane verification helpers.
//!
//! This module provides types for representing git branch, stash, and protection
//! state that can be used by proof lanes to verify repository state.

/// Represents a git branch and its state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchState<'a> {
    /// Branch name.
    pub name: &'a str,
    /// Whether the branch is remote-tracked.
    pub is_remote: bool,
    /// Whether the branch has unpushed commits.
    pub has_unpushed: bool,
    /// Whether the branch is protected.
    pub is_protected: bool,
}

/// Represents a git stash entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry<'a> {
    /// Stash index.
    pub index: usize,
    /// Branch the stash was created from.
    pub branch: &'a str,
    /// Commit message of the stash.
    pub message: &'a str,
}

/// Represents the overall repository git state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitState<'a> {
    /// Current branch name.
    pub current_branch: &'a str,
    /// All local branches.
    pub branches: &'a [&'a str],
    /// All stash entries.
    pub stashes: &'a [StashEntry<'a>],
    /// Whether there are unpushed commits.
    pub has_unpushed: bool,
}

impl<'a> GitState<'a> {
    /// Returns true if there are any unpushed commits.
    #[must_use]
    pub const fn has_unpushed_commits(&self) -> bool {
        self.has_unpushed
    }

    /// Returns true if there are no stash entries.
    #[must_use]
    pub const fn has_stashes(&self) -> bool {
        !self.stashes.is_empty()
    }

    /// Returns the number of stash entries.
    #[must_use]
    pub const fn stash_count(&self) -> usize {
        self.stashes.len()
    }
}

impl<'a> BranchState<'a> {
    /// Returns true if the branch has unpushed commits.
    #[must_use]
    pub const fn has_unpushed_commits(&self) -> bool {
        self.has_unpushed
    }
}
