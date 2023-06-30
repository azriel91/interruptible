use stream_cancel::Trigger;

/// Waits for an interruption signal.
pub(crate) struct InterruptGuard {
    /// Trigger to turn off the stream.
    stream_trigger: Trigger,
}

impl InterruptGuard {
    /// Returns a new `InterruptGuard`.
    pub(crate) fn new(stream_trigger: Trigger) -> Self {
        Self { stream_trigger }
    }

    /// Waits for an interrupt signal, sent by ctrl c.
    ///
    /// On Unix this is the `SIGINT` signal.
    pub async fn wait_for_signal(self) {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to initialize signal handler for SIGINT");

        drop(self.stream_trigger);
    }
}
