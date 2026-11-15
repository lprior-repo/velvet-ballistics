#![forbid(unsafe_code)]

use std::io::Write;
use std::num::NonZeroUsize;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use vb_ipc::server::{IpcResponse, IpcServer, IpcServerError};
use vb_ipc::{IpcCommand, IpcFrameHeader, IpcPayload};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn fixture_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("ipc60-{}-{label}", std::process::id()))
}

fn write_new(path: &std::path::Path, contents: &[u8]) -> TestResult {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(contents)?;
    Ok(())
}

fn owner_health(server: &mut IpcServer, path: &std::path::Path) -> TestResult {
    let mut client = UnixStream::connect(path)?;
    client.set_read_timeout(Some(Duration::from_secs(2)))?;
    let payload = postcard::to_allocvec(&IpcPayload::Health)?;
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 71, u32::try_from(payload.len())?);
    client.write_all(&header.encode()?)?;
    client.write_all(&payload)?;
    let mut runtime = vb_runtime::runtime::Runtime::new_for_tests_and_benchmarks_only(
        NonZeroUsize::MIN,
        vb_runtime::shard::ShardConfig::default(),
    )?;
    (0..4).try_for_each(|_| {
        assert!(server.poll_once(&mut runtime, Some(Duration::from_millis(10)))?);
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    let response_header = vb_ipc::read_frame_header(&mut client)?;
    assert_eq!(response_header.command, IpcCommand::Health);
    assert_eq!(response_header.correlation, 71);
    let response = vb_ipc::read_frame_payload(&mut client, &response_header)?;
    assert_eq!(
        postcard::from_bytes::<IpcResponse>(&response)?,
        IpcResponse::Healthy
    );
    Ok(())
}

#[test]
fn bind_is_owner_only_under_permissive_umask() -> TestResult {
    if std::env::var_os("VB_IPC_OWNER_MODE_CHILD").is_none() {
        let output = Command::new("sh")
            .args(["-c", "umask 000; exec \"$@\"", "ipc-mode-child"])
            .arg(std::env::current_exe()?)
            .args([
                "--exact",
                "bind_is_owner_only_under_permissive_umask",
                "--nocapture",
            ])
            .env("VB_IPC_OWNER_MODE_CHILD", "1")
            .output()?;
        assert!(output.status.success(), "child failed: {output:?}");
        return Ok(());
    }
    let socket = fixture_path("socket");
    let owner = fixture_path("owner");
    write_new(&owner, b"owner control")?;
    let mut server = IpcServer::bind(&socket)?;
    let metadata = std::fs::metadata(&socket)?;
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    assert_eq!(metadata.uid(), std::fs::metadata(&owner)?.uid());
    owner_health(&mut server, &socket)?;
    assert!(matches!(
        IpcServer::bind(&socket),
        Err(IpcServerError::BindFailed { .. })
    ));
    let after = std::fs::metadata(&socket)?;
    assert_eq!(
        (after.ino(), after.uid(), after.mode()),
        (metadata.ino(), metadata.uid(), metadata.mode())
    );
    owner_health(&mut server, &socket)
}

#[test]
fn bind_failure_preserves_existing_file_permissions_and_bytes() -> TestResult {
    let path = fixture_path("file");
    write_new(&path, b"existing owned contents")?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))?;
    let before = std::fs::metadata(&path)?;
    assert!(matches!(
        IpcServer::bind(&path),
        Err(IpcServerError::BindFailed { .. })
    ));
    let after = std::fs::metadata(&path)?;
    assert_eq!(std::fs::read(&path)?, b"existing owned contents");
    assert_eq!(
        (after.ino(), after.uid(), after.mode()),
        (before.ino(), before.uid(), before.mode())
    );
    Ok(())
}

#[test]
fn bind_failure_does_not_create_missing_parent() -> TestResult {
    let parent = fixture_path("missing");
    let socket = parent.join("socket");
    assert!(!parent.exists());
    assert!(matches!(
        IpcServer::bind(&socket),
        Err(IpcServerError::BindFailed { .. })
    ));
    assert!(!parent.exists());
    assert!(!socket.exists());
    Ok(())
}
