impl crate::shard::Shard {
    pub(crate) fn drive_duplicate_retry_fixture(
        &mut self,
        run: vb_core::RunId,
    ) -> crate::RuntimeResult<()> {
        self.drive_run(run)
    }
}
