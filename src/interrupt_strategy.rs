//! How to poll an underlying stream when an interruption is received.
use std::fmt::Debug;

/// How to poll an underlying stream when an interruption is received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterruptStrategy {
    /// On interrupt, keep going.
    IgnoreInterruptions,
    /// On interrupt, wait for the current future's to complete and yield its
    /// output, but do not poll the underlying stream for any more futures.
    FinishCurrent,
    /// On interrupt, continue polling the stream for the next `n` futures.
    ///
    /// `n` is an upper bound, so fewer than `n` futures may be yielded if the
    /// underlying stream ends early.
    PollNextN(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug() {
        assert_eq!(
            "FinishCurrent",
            format!("{:?}", InterruptStrategy::FinishCurrent)
        );
    }

    #[test]
    fn clone() {
        assert_eq!(
            InterruptStrategy::FinishCurrent,
            Clone::clone(&InterruptStrategy::FinishCurrent)
        );
    }
}
