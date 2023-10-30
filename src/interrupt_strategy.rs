//! How to poll an underlying stream when an interruption is received.
use std::fmt::Debug;

/// How to poll an underlying stream when an interruption is received.
///
/// # Examples
///
/// Instead of storing one of the [`InterruptStrategyT`] types through a type
/// parameter, consumers of this library can hold an [`InterruptStrategy`], and
/// at runtime use a `match` to instantiate the `InterruptibleStream`.
///
/// ```rust,no_run
/// use futures::stream::Stream;
/// use interruptible::{
///     InterruptSignal, InterruptStrategy, InterruptibleStream, InterruptibleStreamExt,
/// };
/// use tokio::sync::mpsc;
///
/// pub fn interrupt_strategy_apply<'rx, S>(
///     interrupt_strategy: InterruptStrategy,
///     stream: S,
///     interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
/// ) where
///     S: Stream,
/// {
///     // Each of these return a different `InterruptibleStream<'rx, S, IS>`.
///     match self {
///         InterruptStrategy::FinishCurrent => {
///             let _interruptible_stream = stream.interruptible_with(interrupt_rx, FinishCurrent);
///         }
///         InterruptStrategy::PollNextN(n) => {
///             let _interruptible_stream = stream.interruptible_with(interrupt_rx, PollNextN(n));
///         }
///     }
/// }
/// ```

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
    PollNextN(u32),
}

/// Marker trait for interrupt strategy variants.
pub trait InterruptStrategyT: Clone + Copy + Debug + PartialEq + Eq {
    /// Data stored by `InterruptibleStream` when polled.
    type PollState: Debug;

    /// Initializes the [`PollState`] for the `InterruptibleStream` to track
    /// state across poll invocations.
    fn poll_state_new(&self) -> Self::PollState;
}

/// On interrupt, keep going (ignore interruptions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IgnoreInterruptions;

impl InterruptStrategyT for IgnoreInterruptions {
    type PollState = ();

    fn poll_state_new(&self) -> Self::PollState {}
}

/// On interrupt, wait for the current future's to complete and yield its
/// output, but do not poll the underlying stream for any more futures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinishCurrent;

/// Whether the stream has been interrupted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishCurrentState {
    /// The stream has not been interrupted.
    NotInterrupted,
    /// The stream has been interrupted.
    Interrupted,
}

impl InterruptStrategyT for FinishCurrent {
    type PollState = FinishCurrentState;

    fn poll_state_new(&self) -> Self::PollState {
        FinishCurrentState::NotInterrupted
    }
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
        assert_eq!(
            "FinishCurrent",
            format!("{:?}", InterruptStrategy::FinishCurrent)
        );
        assert_eq!("IgnoreInterruptions", format!("{:?}", IgnoreInterruptions));
        assert_eq!("FinishCurrent", format!("{:?}", FinishCurrent));
        assert_eq!(
            "NotInterrupted",
            format!("{:?}", FinishCurrentState::NotInterrupted)
        );
        assert_eq!("PollNextN(3)", format!("{:?}", PollNextN(3)));
        assert_eq!(
            "Interrupted { n_remaining: 3 }",
            format!("{:?}", PollNextNState::Interrupted { n_remaining: 3 })
        );
    }

    #[test]
    fn clone() {
        assert_eq!(
            InterruptStrategy::FinishCurrent,
            Clone::clone(&InterruptStrategy::FinishCurrent)
        );
        assert_eq!(IgnoreInterruptions, Clone::clone(&IgnoreInterruptions));
        assert_eq!(FinishCurrent, Clone::clone(&FinishCurrent));
        assert_eq!(
            FinishCurrentState::NotInterrupted,
            Clone::clone(&FinishCurrentState::NotInterrupted)
        );
        assert_eq!(PollNextN(3), Clone::clone(&PollNextN(3)));
        assert_eq!(
            PollNextNState::Interrupted { n_remaining: 3 },
            Clone::clone(&PollNextNState::Interrupted { n_remaining: 3 })
        );
    }
}
