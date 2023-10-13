/// How to poll an underlying stream when an interruption is received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptStrategy {
    /// On interrupt, wait for the current future's to complete and yield its
    /// output, but do not poll the underlying stream for any more futures.
    FinishCurrent,
    /// On interrupt, continue polling the stream for the next `n` futures.
    ///
    /// `n` is an upper bound, so fewer than `n` futures may be yielded if the
    /// underlying stream ends early.
    PollNextN(u32),
}
