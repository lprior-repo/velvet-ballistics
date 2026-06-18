pub fn runtime_channel_fixture() {
    let _channel_pair = tokio::sync::mpsc::unbounded_channel();
}
