//! Test harness for the deliver sink.
//!
//! This module provides two sub-modules:
//!
//! - `test_support` — unit-test hooks driven by `install(HookConfig)`
//! - `debug_test_support` — debug/integration hooks driven by environment
//!   variables (compiled when `cfg(all(not(test), feature = "instrumented-cli"))`)

// ---------------------------------------------------------------------------
// test_support — unit-test hooks (cfg(test))
// ---------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::ffi::{OsStr, OsString};
    use std::io;
    use std::path::{Path, PathBuf};

    use crate::deliver_sink::deliver_error::DeliverSinkError;

    #[derive(Default)]
    pub(crate) struct Hooks {
        pub(crate) cleanup_failures: Vec<OsString>,
        pub(crate) parent_change: Option<ParentChange>,
        pub(crate) before_link_parent_change: Option<PostCommitParentChange>,
        pub(crate) after_link_sync_parent_change: Option<PostCommitParentChange>,
        pub(crate) post_commit_parent_change: Option<PostCommitParentChange>,
        pub(crate) post_commit_final_path_change: Option<FinalPathChange>,
        pub(crate) sync_results: VecDeque<Result<(), DeliverSinkError>>,
    }

    pub(crate) enum ParentChange {
        ReplaceOpenedPathWithNewDirectory { moved_to: PathBuf },
    }

    pub(crate) enum PostCommitParentChange {
        #[cfg(unix)]
        ReplaceResolvedPathWithSymlink {
            moved_to: PathBuf,
            replacement: PathBuf,
        },
    }

    pub(crate) enum FinalPathChange {
        UnlinkFinalPath,
        ReplaceFinalPath,
    }

    #[derive(Default)]
    pub(crate) struct HookConfig {
        pub(crate) cleanup_failures: Vec<OsString>,
        pub(crate) parent_change: Option<ParentChange>,
        pub(crate) before_link_parent_change: Option<PostCommitParentChange>,
        pub(crate) after_link_sync_parent_change: Option<PostCommitParentChange>,
        pub(crate) post_commit_parent_change: Option<PostCommitParentChange>,
        pub(crate) post_commit_final_path_change: Option<FinalPathChange>,
        pub(crate) sync_results: VecDeque<Result<(), DeliverSinkError>>,
    }

    pub(crate) struct InstalledHooks;

    thread_local! {
        static HOOKS: RefCell<Hooks> = RefCell::new(Hooks::default());
    }

    fn with_hooks<T>(f: impl FnOnce(&mut Hooks) -> T) -> T {
        HOOKS.with(|hooks| {
            let mut hooks = hooks.borrow_mut();
            f(&mut hooks)
        })
    }

    pub(crate) fn install(config: HookConfig) -> InstalledHooks {
        with_hooks(|hooks| {
            *hooks = Hooks {
                cleanup_failures: config.cleanup_failures,
                parent_change: config.parent_change,
                before_link_parent_change: config.before_link_parent_change,
                after_link_sync_parent_change: config.after_link_sync_parent_change,
                post_commit_parent_change: config.post_commit_parent_change,
                post_commit_final_path_change: config.post_commit_final_path_change,
                sync_results: config.sync_results,
            };
        });
        InstalledHooks
    }

    impl Drop for InstalledHooks {
        fn drop(&mut self) {
            with_hooks(|hooks| {
                *hooks = Hooks::default();
            });
        }
    }

    pub(crate) fn maybe_change_parent_path(parent: &Path) -> Result<(), io::Error> {
        let parent_change = with_hooks(|hooks| hooks.parent_change.take());

        if let Some(ParentChange::ReplaceOpenedPathWithNewDirectory { moved_to }) = parent_change {
            std::fs::rename(parent, &moved_to)?;
            std::fs::create_dir(parent)?;
        }
        Ok(())
    }

    pub(crate) fn maybe_change_parent_path_before_link(parent: &Path) -> Result<(), io::Error> {
        let parent_change = with_hooks(|hooks| hooks.before_link_parent_change.take());
        apply_resolved_parent_swap(parent, parent_change)
    }

    pub(crate) fn maybe_change_parent_path_after_link_sync(parent: &Path) -> Result<(), io::Error> {
        let parent_change = with_hooks(|hooks| hooks.after_link_sync_parent_change.take());
        apply_resolved_parent_swap(parent, parent_change)
    }

    pub(crate) fn maybe_change_parent_path_after_final_sync(
        parent: &Path,
    ) -> Result<(), io::Error> {
        let post_commit_parent_change = with_hooks(|hooks| hooks.post_commit_parent_change.take());

        apply_resolved_parent_swap(parent, post_commit_parent_change)
    }

    pub(crate) fn maybe_change_final_path_after_final_sync(path: &Path) -> Result<(), io::Error> {
        let final_path_change = with_hooks(|hooks| hooks.post_commit_final_path_change.take());
        apply_final_path_change(path, final_path_change)
    }

    fn apply_resolved_parent_swap(
        parent: &Path,
        parent_change: Option<PostCommitParentChange>,
    ) -> Result<(), io::Error> {
        #[cfg(unix)]
        if let Some(PostCommitParentChange::ReplaceResolvedPathWithSymlink {
            moved_to,
            replacement,
        }) = parent_change
        {
            std::fs::rename(parent, &moved_to)?;
            std::os::unix::fs::symlink(&replacement, parent)?;
        }
        Ok(())
    }

    fn apply_final_path_change(
        path: &Path,
        final_path_change: Option<FinalPathChange>,
    ) -> Result<(), io::Error> {
        match final_path_change {
            Some(FinalPathChange::UnlinkFinalPath) => {
                std::fs::remove_file(path)?;
            }
            Some(FinalPathChange::ReplaceFinalPath) => {
                let parent = path.parent().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("missing parent for test final path {}", path.display()),
                    )
                })?;
                let replacement = parent.join(".vb-rival-replacement");
                std::fs::write(&replacement, b"rival replacement\n")?;
                std::fs::rename(&replacement, path)?;
            }
            None => {}
        }
        Ok(())
    }

    pub(crate) fn should_fail_cleanup(path: &OsStr) -> bool {
        with_hooks(|hooks| {
            if let Some(position) = hooks
                .cleanup_failures
                .iter()
                .position(|candidate| candidate == path)
            {
                let _ = hooks.cleanup_failures.remove(position);
                true
            } else {
                false
            }
        })
    }

    pub(crate) fn next_sync_result() -> Option<Result<(), DeliverSinkError>> {
        with_hooks(|hooks| hooks.sync_results.pop_front())
    }
}
