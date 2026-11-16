use super::{IpcServerError, WriteResult, write_pending_bytes};
use std::collections::VecDeque;
use std::io::{self, ErrorKind, Write};

struct ScriptedWriter {
    steps: VecDeque<io::Result<usize>>,
    offered: Vec<Vec<u8>>,
    accepted: Vec<u8>,
}

impl ScriptedWriter {
    fn new(steps: impl IntoIterator<Item = io::Result<usize>>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            offered: Vec::new(),
            accepted: Vec::new(),
        }
    }
}

impl Write for ScriptedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.offered.push(bytes.to_vec());
        let count = self
            .steps
            .pop_front()
            .ok_or_else(|| io::Error::other("unexpected write after script exhaustion"))??;
        let accepted = bytes
            .get(..count)
            .ok_or_else(|| io::Error::other("script count exceeds offered bytes"))?;
        self.accepted.extend_from_slice(accepted);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("unexpected flush"))
    }
}

#[test]
fn partial_write_characterization_preserves_suffix_order_and_readiness()
-> Result<(), IpcServerError> {
    let mut writer = ScriptedWriter::new([
        Ok(2),
        Err(io::Error::from(ErrorKind::WouldBlock)),
        Err(io::Error::from(ErrorKind::Interrupted)),
        Ok(3),
        Ok(4),
    ]);
    let mut pending = vec![0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98];
    let mut ready = true;

    assert!(matches!(
        write_pending_bytes(&mut writer, &mut pending, &mut ready)?,
        WriteResult::Wrote
    ));
    assert_eq!(writer.accepted, [0x10, 0x21]);
    assert_eq!(pending, [0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98]);
    assert!(ready);

    assert!(matches!(
        write_pending_bytes(&mut writer, &mut pending, &mut ready)?,
        WriteResult::WouldBlock
    ));
    assert_eq!(writer.accepted, [0x10, 0x21]);
    assert_eq!(pending, [0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98]);
    assert!(!ready);

    assert!(matches!(
        write_pending_bytes(&mut writer, &mut pending, &mut ready)?,
        WriteResult::Interrupted
    ));
    assert_eq!(writer.accepted, [0x10, 0x21]);
    assert_eq!(pending, [0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98]);
    assert!(!ready);

    assert!(matches!(
        write_pending_bytes(&mut writer, &mut pending, &mut ready)?,
        WriteResult::Wrote
    ));
    assert_eq!(writer.accepted, [0x10, 0x21, 0x32, 0x43, 0x54]);
    assert_eq!(pending, [0x65, 0x76, 0x87, 0x98]);
    assert!(!ready);

    assert!(matches!(
        write_pending_bytes(&mut writer, &mut pending, &mut ready)?,
        WriteResult::Wrote
    ));
    assert_eq!(
        writer.accepted,
        [0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98]
    );
    assert!(pending.is_empty());
    assert!(!ready);
    assert!(writer.steps.is_empty());
    assert_eq!(
        writer.offered,
        [
            vec![0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98],
            vec![0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98],
            vec![0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98],
            vec![0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98],
            vec![0x65, 0x76, 0x87, 0x98],
        ]
    );
    Ok(())
}

#[test]
fn no_progress_characterization_preserves_pending_and_readiness() -> Result<(), IpcServerError> {
    for initial_ready in [false, true] {
        for interrupted in [false, true] {
            let result = if interrupted {
                Err(io::Error::from(ErrorKind::Interrupted))
            } else {
                Ok(0)
            };
            let mut writer = ScriptedWriter::new([result]);
            let mut pending = vec![0x12, 0x34];
            let mut ready = initial_ready;
            let outcome = write_pending_bytes(&mut writer, &mut pending, &mut ready)?;
            assert!(matches!(
                (interrupted, outcome),
                (true, WriteResult::Interrupted) | (false, WriteResult::Closed)
            ));
            assert_eq!(pending, [0x12, 0x34]);
            assert_eq!(ready, initial_ready);
            assert!(writer.accepted.is_empty());
            assert!(writer.steps.is_empty());
            assert_eq!(writer.offered, [vec![0x12, 0x34]]);
        }
    }
    Ok(())
}

#[test]
fn fatal_write_characterization_preserves_error_bytes_and_readiness() {
    for initial_ready in [false, true] {
        let mut writer = ScriptedWriter::new([Err(io::Error::new(
            ErrorKind::BrokenPipe,
            "exact write failure",
        ))]);
        let mut pending = vec![0x12, 0x34];
        let mut ready = initial_ready;
        let outcome = write_pending_bytes(&mut writer, &mut pending, &mut ready);
        assert!(
            matches!(outcome, Err(IpcServerError::ResponseWriteFailed { source })
            if source.kind() == ErrorKind::BrokenPipe && source.to_string() == "exact write failure")
        );
        assert_eq!(pending, [0x12, 0x34]);
        assert_eq!(ready, initial_ready);
        assert!(writer.accepted.is_empty());
        assert!(writer.steps.is_empty());
        assert_eq!(writer.offered, [vec![0x12, 0x34]]);
    }
}
