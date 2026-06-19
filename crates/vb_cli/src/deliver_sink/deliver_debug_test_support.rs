//! Debug/test hooks for the deliver sink, compiled into the release binary
//! when the `instrumented-cli` Cargo feature is enabled.
//!
//! These hooks are driven by environment variables rather than by
//! `install(HookConfig)` so that integration tests can set env vars and
//! launch the real binary without Rust-level test support.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

use crate::deliver_sink::deliver_error::DeliverSinkError;

const CLEANUP_FAILURES_ENV: &str = "VB_DELIVER_SINK_TEST_CLEANUP_FAILURES";
const POST_COMMIT_FINAL_ACTION_ENV: &str = "VB_DELIVER_SINK_TEST_POST_COMMIT_FINAL_ACTION";
const SYNC_RESULTS_ENV: &str = "VB_DELIVER_SINK_TEST_SYNC_RESULTS";
const RIVAL_REPLACEMENT_BYTES: &[u8] = b"rival replacement\n";

enum FinalPathChange {
    UnlinkFinalPath,
    ReplaceFinalPath,
}

#[derive(Default)]
struct Hooks {
    loaded: bool,
    cleanup_failures: Vec<OsString>,
    post_commit_final_path_change: Option<FinalPathChange>,
    sync_results: VecDeque<Result<(), DeliverSinkError>>,
}

thread_local! {
    static HOOKS: RefCell<Hooks> = RefCell::new(Hooks::default());
}

fn with_hooks<T>(f: impl FnOnce(&mut Hooks) -> T) -> T {
    HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        if !hooks.loaded {
            *hooks = load_hooks();
        }
        f(&mut hooks)
    })
}

fn load_hooks() -> Hooks {
    Hooks {
        loaded: true,
        cleanup_failures: parse_cleanup_failures(),
        post_commit_final_path_change: parse_final_path_change(),
        sync_results: parse_sync_results(),
    }
}

fn parse_cleanup_failures() -> Vec<OsString> {
    env::var(CLEANUP_FAILURES_ENV)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter(|entry| !entry.is_empty())
                .map(OsString::from)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_final_path_change() -> Option<FinalPathChange> {
    match env::var(POST_COMMIT_FINAL_ACTION_ENV).ok().as_deref() {
        Some("unlink-final") => Some(FinalPathChange::UnlinkFinalPath),
        Some("replace-final") => Some(FinalPathChange::ReplaceFinalPath),
        _ => None,
    }
}

fn parse_sync_results() -> VecDeque<Result<(), DeliverSinkError>> {
    env::var(SYNC_RESULTS_ENV)
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter(|entry| !entry.is_empty())
                .filter_map(parse_sync_result)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_sync_result(token: &str) -> Option<Result<(), DeliverSinkError>> {
    match token {
        "ok" => Some(Ok(())),
        "permission_denied" => Some(Err(DeliverSinkError::Io(io::ErrorKind::PermissionDenied))),
        _ => None,
    }
}

pub(crate) fn maybe_change_final_path_after_final_sync(
    path: &Path,
) -> Result<(), DeliverSinkError> {
    let final_path_change = with_hooks(|hooks| hooks.post_commit_final_path_change.take());

    match final_path_change {
        Some(FinalPathChange::UnlinkFinalPath) => remove_file_if_present(path),
        Some(FinalPathChange::ReplaceFinalPath) => replace_final_path(path),
        None => Ok(()),
    }
}

fn remove_file_if_present(path: &Path) -> Result<(), DeliverSinkError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(DeliverSinkError::Io(error.kind())),
    }
}

fn replace_final_path(path: &Path) -> Result<(), DeliverSinkError> {
    let Some(parent) = path.parent() else {
        return Err(DeliverSinkError::MissingParent);
    };

    let replacement = parent.join(".vb-rival-replacement");
    std::fs::write(&replacement, RIVAL_REPLACEMENT_BYTES)
        .map_err(|error| DeliverSinkError::Io(error.kind()))?;
    std::fs::rename(&replacement, path).map_err(|error| DeliverSinkError::Io(error.kind()))
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
