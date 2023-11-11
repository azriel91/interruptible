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
///     interrupt_strategy::{FinishCurrent, PollNextN},
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
///     match interrupt_strategy {
///         InterruptStrategy::IgnoreInterruptions => {
///             // use stream as is
///         }
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
