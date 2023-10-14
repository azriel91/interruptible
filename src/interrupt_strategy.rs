//! How to poll an underlying stream when an interruption is received.
use std::fmt::Debug;

/// Marker trait for interrupt strategy variants.
pub trait InterruptStrategyT: Clone + Copy + Debug + PartialEq + Eq {
    /// Data stored by `InterruptibleStream` when polled.
    type PollState: Debug;

    /// Initializes the [`PollState`] for the `InterruptibleStream` to track
    /// state across poll invocations.
    fn poll_state_new(&self) -> Self::PollState;
}

/// On interrupt, wait for the current future's to complete and yield its
/// output, but do not poll the underlying stream for any more futures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinishCurrent;

impl InterruptStrategyT for FinishCurrent {
    type PollState = ();

    fn poll_state_new(&self) -> Self::PollState {}
}

/// On interrupt, continue polling the stream for the next `n` futures.
///
/// `n` is an upper bound, so fewer than `n` futures may be yielded if the
/// underlying stream ends early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollNextN(pub u32);

/// Whether the stream has been interrupted, and how many futures to continue
/// polling for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollNextNState {
    /// The stream has not been interrupted.
    NotInterrupted,
    /// The stream has been interrupted.
    Interrupted {
        /// Number of futures remaining to poll form the underlying stream.
        n_remaining: u32,
    },
}

impl InterruptStrategyT for PollNextN {
    type PollState = PollNextNState;

    fn poll_state_new(&self) -> Self::PollState {
        PollNextNState::NotInterrupted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug() {
        assert_eq!("FinishCurrent", format!("{:?}", FinishCurrent));
        assert_eq!("PollNextN(3)", format!("{:?}", PollNextN(3)));
        assert_eq!(
            "Interrupted { n_remaining: 3 }",
            format!("{:?}", PollNextNState::Interrupted { n_remaining: 3 })
        );
    }

    #[test]
    fn clone() {
        assert_eq!(FinishCurrent, Clone::clone(&FinishCurrent));
        assert_eq!(PollNextN(3), Clone::clone(&PollNextN(3)));
        assert_eq!(
            PollNextNState::Interrupted { n_remaining: 3 },
            Clone::clone(&PollNextNState::Interrupted { n_remaining: 3 })
        );
    }
}
