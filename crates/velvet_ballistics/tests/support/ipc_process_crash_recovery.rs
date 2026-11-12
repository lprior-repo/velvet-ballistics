use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::num::NonZeroUsize;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::os::unix::process::ExitStatusExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};
use vb_ipc::server::IpcResponse;
use vb_ipc::{IPC_HEADER_LEN, IpcCommand, IpcFrameHeader, MaxPayloadBytes};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const MAX_ENTRIES: usize = 4096;
const MAX_DEPTH: usize = 16;
const MAX_COPY_BYTES: u64 = 268_435_456;
const COPY_BUFFER: usize = 65_536;
const MAX_FRAME: usize = 65_536;
const MAX_REQUESTS: usize = 64;
const POLLS: usize = 500;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const IO_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
enum Refusal {
    Limit(&'static str),
    State(&'static str),
    Protocol(&'static str),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Limit(name) => write!(f, "crash fixture resource limit: {name}"),
            Self::State(reason) => write!(f, "crash fixture state: {reason}"),
            Self::Protocol(reason) => write!(f, "crash fixture protocol: {reason}"),
        }
    }
}

impl std::error::Error for Refusal {}

pub(crate) struct CrashFixture {
    root: Option<tempfile::TempDir>,
    db: PathBuf,
    copy: PathBuf,
    sockets: [Option<PathBuf>; 2],
    child: Option<Child>,
    client: Option<UnixStream>,
    deadline: Instant,
    starts: usize,
    copies: usize,
    requests: usize,
    first_killed: bool,
    second_success: bool,
    copy_ready: bool,
    copy_inspected: bool,
    completed: bool,
}

impl CrashFixture {
    pub(crate) fn new() -> TestResult<Self> {
        let root = tempfile::Builder::new()
            .prefix("vb-pcr-")
            .tempdir_in(std::env::temp_dir())?;
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(30))
            .ok_or(Refusal::Limit("scenario deadline"))?;
        Ok(Self {
            db: root.path().join("db"),
            copy: root.path().join("copy"),
            root: Some(root),
            sockets: [None, None],
            child: None,
            client: None,
            deadline,
            starts: 0,
            copies: 0,
            requests: 0,
            first_killed: false,
            second_success: false,
            copy_ready: false,
            copy_inspected: false,
            completed: false,
        })
    }

    pub(crate) fn db_path(&self) -> &Path {
        &self.db
    }

    pub(crate) fn copy_path(&self) -> &Path {
        &self.copy
    }

    pub(crate) fn start_server(&mut self, socket_name: &str) -> TestResult {
        remaining(self.deadline)?;
        if self.child.is_some() || self.starts >= 2 {
            return Err(Refusal::State("one live child and two starts permitted").into());
        }
        if self.starts == 1
            && !(self.first_killed && self.copies == 1 && self.copy_ready && !self.copy_inspected)
        {
            return Err(Refusal::State("restart requires an unopened crash copy").into());
        }
        let mut components = Path::new(socket_name).components();
        if socket_name.len() > 20
            || !matches!(components.next(), Some(Component::Normal(_)))
            || components.next().is_some()
        {
            return Err(Refusal::State("socket must be a short leaf name").into());
        }
        let root = self
            .root
            .as_ref()
            .ok_or(Refusal::State("fixture root absent"))?;
        let socket = root.path().join(socket_name);
        if self.sockets.iter().flatten().any(|old| old == &socket) {
            return Err(Refusal::State("restart must use a fresh socket path").into());
        }
        let child = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
            .arg("ipc-serve")
            .arg("--socket")
            .arg(&socket)
            .arg("--db")
            .arg(self.db_path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        self.child = Some(child);
        *self
            .sockets
            .get_mut(self.starts)
            .ok_or(Refusal::Limit("child starts"))? = Some(socket.clone());
        self.starts = self
            .starts
            .checked_add(1)
            .ok_or(Refusal::Limit("child starts"))?;
        for _ in 0..POLLS {
            remaining(self.deadline)?;
            if self.poll_status()?.is_some() {
                return Err(Refusal::State("server exited before readiness").into());
            }
            match UnixStream::connect(&socket) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(IO_INTERVAL))?;
                    stream.set_write_timeout(Some(IO_INTERVAL))?;
                    self.client = Some(stream);
                    return Ok(());
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                    ) => {}
                Err(error) => return Err(error.into()),
            }
            std::thread::sleep(POLL_INTERVAL.min(remaining(self.deadline)?));
        }
        Err(Refusal::Limit("readiness polls").into())
    }

    pub(crate) fn request(
        &mut self,
        command: IpcCommand,
        correlation: u64,
        payload: &[u8],
    ) -> TestResult<IpcResponse> {
        self.requests = self
            .requests
            .checked_add(1)
            .ok_or(Refusal::Limit("requests"))?;
        let max_payload = MAX_FRAME
            .checked_sub(IPC_HEADER_LEN)
            .ok_or(Refusal::Limit("frame header"))?;
        if self.requests > MAX_REQUESTS || payload.len() > max_payload {
            return Err(Refusal::Limit("request or frame size").into());
        }
        remaining(self.deadline)?;
        let io_deadline = self.deadline;
        let stream = self
            .client
            .as_mut()
            .ok_or(Refusal::State("IPC connection absent"))?;
        let header = IpcFrameHeader::new(command, 0, correlation, u32::try_from(payload.len())?);
        write_bounded(stream, &header.encode()?, io_deadline)?;
        write_bounded(stream, payload, io_deadline)?;
        let mut encoded_header = [0u8; IPC_HEADER_LEN];
        read_bounded(stream, &mut encoded_header, io_deadline)?;
        let limit = MaxPayloadBytes::new(
            NonZeroUsize::new(max_payload).ok_or(Refusal::Limit("payload bound"))?,
        );
        let response = IpcFrameHeader::decode(&encoded_header, limit)?;
        if response.correlation != correlation || response.command != command {
            return Err(Refusal::Protocol("response identity mismatch").into());
        }
        let len = usize::try_from(response.payload_len)?;
        if IPC_HEADER_LEN
            .checked_add(len)
            .ok_or(Refusal::Limit("response length"))?
            > MAX_FRAME
        {
            return Err(Refusal::Limit("response frame").into());
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(len)?;
        bytes.resize(len, 0);
        read_bounded(stream, &mut bytes, io_deadline)?;
        let (result, trailing) = postcard::take_from_bytes(&bytes)?;
        if !trailing.is_empty() {
            return Err(Refusal::Protocol("trailing response payload").into());
        }
        Ok(result)
    }

    pub(crate) fn kill_and_reap(&mut self) -> TestResult<ExitStatus> {
        if self.starts != 1 || self.first_killed {
            return Err(Refusal::State("only the first server is the crash target").into());
        }
        if self.poll_status()?.is_some() {
            return Err(Refusal::State("first server exited before SIGKILL").into());
        }
        self.child
            .as_mut()
            .ok_or(Refusal::State("child absent"))?
            .kill()?;
        let status = self.reap_until(self.deadline)?;
        if status.signal() != Some(9) {
            return Err(Refusal::State("first server was not reaped after SIGKILL").into());
        }
        self.first_killed = true;
        Ok(status)
    }

    pub(crate) fn wait_for_exit(&mut self) -> TestResult<ExitStatus> {
        if self.starts != 2 || !self.first_killed {
            return Err(Refusal::State("second server exit expected").into());
        }
        let status = self.reap_until(self.deadline)?;
        if status.code() != Some(0) {
            return Err(Refusal::State("second server did not exit successfully").into());
        }
        self.second_success = true;
        Ok(status)
    }

    pub(crate) fn copy_after_exit(&mut self) -> TestResult {
        remaining(self.deadline)?;
        if self.child.is_some()
            || !self.first_killed
            || self.copies >= 2
            || (self.starts == 2 && !self.second_success)
            || self.copy_ready
        {
            return Err(Refusal::State("copy requires reaped owner and absent prior copy").into());
        }
        copy_tree(self.db_path(), self.copy_path(), self.deadline)?;
        self.copies = self.copies.checked_add(1).ok_or(Refusal::Limit("copies"))?;
        self.copy_ready = true;
        self.copy_inspected = false;
        Ok(())
    }

    pub(crate) fn inspect_copy<T>(
        &mut self,
        read: impl FnOnce(&vb_storage::FjallJournal) -> TestResult<T>,
    ) -> TestResult<T> {
        remaining(self.deadline)?;
        if self.child.is_some()
            || self.starts != 2
            || !self.first_killed
            || !self.second_success
            || !self.copy_ready
        {
            return Err(Refusal::State("inspection requires both servers reaped").into());
        }
        let config = vb_storage::FjallConfig::try_new(1_048_576)
            .ok_or(Refusal::State("invalid inspection cache"))?;
        let result = {
            let journal = vb_storage::FjallJournal::open(self.copy_path(), Some(config))?;
            read(&journal)
        }?;
        remaining(self.deadline)?;
        self.copy_inspected = true;
        Ok(result)
    }

    pub(crate) fn delete_copy(&mut self) -> TestResult {
        if self.child.is_some() || !self.copy_ready || !self.copy_inspected {
            return Err(Refusal::State("copy must be inspected and closed before removal").into());
        }
        remove_tree(self.copy_path(), self.deadline)?;
        self.copy_ready = false;
        self.copy_inspected = false;
        Ok(())
    }

    pub(crate) fn finish(mut self) -> TestResult {
        remaining(self.deadline)?;
        if self.child.is_some()
            || !self.first_killed
            || !self.second_success
            || self.copies != 2
            || self.copy_ready
        {
            return Err(Refusal::State("scenario has not completed").into());
        }
        let cleanup_deadline = Instant::now()
            .checked_add(Duration::from_secs(5))
            .ok_or(Refusal::Limit("cleanup deadline"))?;
        remove_tree(self.db_path(), cleanup_deadline)?;
        for path in self.sockets.iter().flatten() {
            remaining(cleanup_deadline)?;
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_socket() => fs::remove_file(path)?,
                Ok(_) => return Err(Refusal::State("unexpected socket path kind").into()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let root = self
            .root
            .as_ref()
            .ok_or(Refusal::State("fixture root absent"))?;
        fs::remove_dir(root.path())?;
        if let Some(root) = self.root.take() {
            drop(root.keep());
        }
        self.completed = true;
        Ok(())
    }

    fn poll_status(&mut self) -> TestResult<Option<ExitStatus>> {
        let status = self
            .child
            .as_mut()
            .ok_or(Refusal::State("child absent"))?
            .try_wait()?;
        if status.is_some() {
            drop(self.child.take());
            drop(self.client.take());
        }
        Ok(status)
    }

    fn reap_until(&mut self, deadline: Instant) -> TestResult<ExitStatus> {
        for _ in 0..POLLS {
            remaining(deadline)?;
            if let Some(status) = self.poll_status()? {
                return Ok(status);
            }
            std::thread::sleep(POLL_INTERVAL.min(remaining(deadline)?));
        }
        Err(Refusal::Limit("owned child reap polls").into())
    }

    fn cleanup_child(&mut self) -> TestResult<ExitStatus> {
        if let Some(status) = self.poll_status()? {
            return Ok(status);
        }
        self.child
            .as_mut()
            .ok_or(Refusal::State("child absent"))?
            .kill()?;
        let deadline = Instant::now()
            .checked_add(Duration::from_secs(5))
            .ok_or(Refusal::Limit("cleanup deadline"))?;
        self.reap_until(deadline)
    }
}

impl Drop for CrashFixture {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let pid = self.child.as_ref().map(Child::id);
        drop(self.client.take());
        let outcome = if self.child.is_some() {
            Some(self.cleanup_child())
        } else {
            None
        };
        if let Some(root) = self.root.take() {
            let retained = root.keep();
            eprintln!(
                "crash fixture retained: path={} starts={} owned_pid={pid:?} reaped={} cleanup={outcome:?}",
                retained.display(),
                self.starts,
                self.child.is_none()
            );
        }
    }
}

fn remaining(deadline: Instant) -> TestResult<Duration> {
    let left = deadline
        .checked_duration_since(Instant::now())
        .filter(|value| !value.is_zero())
        .ok_or(Refusal::Limit("deadline"))?;
    Ok(left)
}

fn read_bounded(stream: &mut UnixStream, mut bytes: &mut [u8], deadline: Instant) -> TestResult {
    for _ in 0..MAX_FRAME {
        if bytes.is_empty() {
            return Ok(());
        }
        stream.set_read_timeout(Some(IO_INTERVAL.min(remaining(deadline)?)))?;
        match stream.read(bytes) {
            Ok(0) => return Err(Refusal::Protocol("response ended early").into()),
            Ok(count) => {
                bytes = bytes
                    .get_mut(count..)
                    .ok_or(Refusal::Protocol("read count"))?;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(Refusal::Limit("frame read operations").into())
}

fn write_bounded(stream: &mut UnixStream, mut bytes: &[u8], deadline: Instant) -> TestResult {
    for _ in 0..MAX_FRAME {
        if bytes.is_empty() {
            return Ok(());
        }
        stream.set_write_timeout(Some(IO_INTERVAL.min(remaining(deadline)?)))?;
        match stream.write(bytes) {
            Ok(0) => return Err(Refusal::Protocol("request write stopped").into()),
            Ok(count) => {
                bytes = bytes.get(count..).ok_or(Refusal::Protocol("write count"))?;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Err(Refusal::Limit("frame write operations").into())
}

struct TreeEntry {
    relative: PathBuf,
    metadata: Metadata,
}

fn tree_manifest(root: &Path, deadline: Instant) -> TestResult<Vec<TreeEntry>> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.file_type().is_dir() {
        return Err(Refusal::State("database root is not a directory").into());
    }
    let mut entries = Vec::new();
    let mut directories = Vec::new();
    entries.try_reserve_exact(MAX_ENTRIES)?;
    directories.try_reserve_exact(MAX_ENTRIES)?;
    entries.push(TreeEntry {
        relative: PathBuf::new(),
        metadata,
    });
    directories.push((PathBuf::new(), 0usize));
    let mut total = 0u64;
    for _ in 0..MAX_ENTRIES {
        let Some((relative, depth)) = directories.pop() else {
            return Ok(entries);
        };
        remaining(deadline)?;
        for item in fs::read_dir(root.join(&relative))?.take(MAX_ENTRIES.saturating_add(1)) {
            remaining(deadline)?;
            let item = item?;
            let child = relative.join(item.file_name());
            let child_depth = depth.checked_add(1).ok_or(Refusal::Limit("copy depth"))?;
            if child_depth > MAX_DEPTH || entries.len() >= MAX_ENTRIES {
                return Err(Refusal::Limit("copy entries or depth").into());
            }
            let metadata = fs::symlink_metadata(root.join(&child))?;
            if metadata.file_type().is_dir() {
                directories.push((child.clone(), child_depth));
            } else if metadata.file_type().is_file() && metadata.nlink() == 1 {
                total = total
                    .checked_add(metadata.len())
                    .filter(|bytes| *bytes <= MAX_COPY_BYTES)
                    .ok_or(Refusal::Limit("copy logical bytes"))?;
                if metadata.len() > MAX_COPY_BYTES {
                    return Err(Refusal::Limit("copy file bytes").into());
                }
            } else {
                return Err(Refusal::State("symlink, hardlink or special database entry").into());
            }
            entries.push(TreeEntry {
                relative: child,
                metadata,
            });
        }
    }
    if directories.is_empty() {
        Ok(entries)
    } else {
        Err(Refusal::Limit("copy directory work").into())
    }
}

fn copy_tree(source: &Path, destination: &Path, deadline: Instant) -> TestResult {
    match fs::symlink_metadata(destination) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(_) => return Err(Refusal::State("copy destination already exists").into()),
        Err(error) => return Err(error.into()),
    }
    let entries = tree_manifest(source, deadline)?;
    for entry in entries {
        remaining(deadline)?;
        let target = destination.join(&entry.relative);
        if entry.metadata.is_dir() {
            fs::create_dir(&target)?;
        } else {
            copy_file(
                &source.join(&entry.relative),
                &target,
                &entry.metadata,
                deadline,
            )?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, target: &Path, expected: &Metadata, deadline: Instant) -> TestResult {
    let mut input = File::open(source)?;
    let opened = input.metadata()?;
    if !opened.is_file()
        || opened.dev() != expected.dev()
        || opened.ino() != expected.ino()
        || opened.len() != expected.len()
    {
        return Err(Refusal::State("source changed before copy").into());
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    let mut buffer = [0u8; COPY_BUFFER];
    let mut left = expected.len();
    for _ in 0..=MAX_COPY_BYTES {
        remaining(deadline)?;
        if left == 0 {
            let mut probe = [0u8; 1];
            if input.read(&mut probe)? != 0 {
                return Err(Refusal::State("source grew during copy").into());
            }
            return Ok(());
        }
        let count = usize::try_from(left.min(u64::try_from(COPY_BUFFER)?))?;
        let area = buffer
            .get_mut(..count)
            .ok_or(Refusal::Limit("copy buffer"))?;
        let count = input.read(area)?;
        if count == 0 {
            return Err(Refusal::State("source shortened during copy").into());
        }
        let bytes = area.get(..count).ok_or(Refusal::State("copy read count"))?;
        left = left
            .checked_sub(u64::try_from(count)?)
            .ok_or(Refusal::Limit("copy bytes"))?;
        output.write_all(bytes)?;
    }
    Err(Refusal::Limit("copy read operations").into())
}

fn remove_tree(root: &Path, deadline: Instant) -> TestResult {
    for entry in tree_manifest(root, deadline)?.into_iter().rev() {
        remaining(deadline)?;
        let path = root.join(entry.relative);
        if entry.metadata.is_dir() {
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
