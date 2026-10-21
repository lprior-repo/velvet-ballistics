#[test]
fn review_reactor_preserves_two_correlations_across_socket_backpressure() -> TestResult {
    let path = path("review_correlations");
    let mut server = IpcServer::bind(&path)?;
    let mut runtime = runtime();
    let mut client = UnixStream::connect(&path)?;
    client.set_read_timeout(Some(Duration::from_millis(200)))?;
    poll(&mut server, &mut runtime, 1)?;
    let connection = server.clients.get_mut(&1).ok_or("missing client")?;
    let filler = [0; 65_536];
    let mut filled = 0usize;
    let mut blocked = false;
    for _ in 0..512 {
        match connection.stream.write(&filler) {
            Ok(count) => filled = filled.checked_add(count).ok_or("overflow")?,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => { blocked = true; break; }
            Err(error) => return Err(error.into()),
        }
    }
    assert!(blocked);
    client.write_all(&health(101, 0)?)?;
    client.write_all(&health(102, 0)?)?;
    poll(&mut server, &mut runtime, 3)?;
    let connection = server.clients.get(&1).ok_or("retained client")?;
    let queued = crate::IpcFrameHeader::decode(&super::read_buffer_header(&connection.write_buffer)?, MaxPayloadBytes::DEFAULT)?;
    assert_eq!(queued.correlation, 101);
    assert!(connection.read_buffer.is_empty());
    assert!(server.has_pending_responses());
    client.read_exact(&mut vec![0; filled])?;
    poll(&mut server, &mut runtime, 20)?;
    assert_eq!(response(&mut client)?, (IpcFrameHeader::new(IpcCommand::Health,0,101,1), IpcResponse::Healthy));
    assert_eq!(response(&mut client)?, (IpcFrameHeader::new(IpcCommand::Health,0,102,1), IpcResponse::Healthy));
    assert!(!server.has_pending_responses());
    std::fs::remove_file(path)?;
    Ok(())
}
