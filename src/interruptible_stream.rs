use std::{
    fmt::{self, Debug},
    pin::Pin,
};

use futures::{
    stream::Stream,
    task::{Context, Poll},
};
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{
    interrupt_strategy::{FinishCurrent, FinishCurrentState, PollNextN, PollNextNState},
    InterruptSignal, InterruptStrategyT, OwnedOrMutRef, StreamOutcome, StreamOutcomeNRemaining,
};

/// Wrapper around a `Stream` that adds interruptible behaviour.
pub struct InterruptibleStream<'rx, S, IS>
where
    IS: InterruptStrategyT,
{
    /// Underlying stream that produces values.
    stream: Pin<Box<S>>,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    /// If an interruption is received, has this stream returned a
    /// `ControlFlow::Break` in `poll_next`.
    interruption_notified: bool,
    /// Whether the underlying stream has an item pending.
    ///
    /// Because `Stream`s are polled in order to poll the underlying future that
    /// produces the item, we need to track whether the underlying stream has
    /// been polled and returned `Poll::Pending`, so we should be intentional
    /// whether or not to interrupt a stream that we have work in progress.
    has_pending: bool,
    /// How to poll the underlying stream when an interruption is received.
    strategy: IS,
    /// Tracks state across poll invocations.
    strategy_poll_state: IS::PollState,
}

impl<'rx, S, IS> fmt::Debug for InterruptibleStream<'rx, S, IS>
where
    IS: InterruptStrategyT,
    <IS as InterruptStrategyT>::PollState: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleStream")
            .field("stream", &"..")
            .field("interrupt_rx", &self.interrupt_rx)
            .field("interruption_notified", &self.interruption_notified)
            .field("has_pending", &self.has_pending)
            .field("strategy", &self.strategy)
            .field("strategy_poll_state", &self.strategy_poll_state)
            .finish()
    }
}

impl<'rx, S, IS> InterruptibleStream<'rx, S, IS>
where
    S: Stream,
    IS: InterruptStrategyT,
{
    /// Returns a new `InterruptibleStream`, wrapping the provided stream.
    pub(crate) fn new(
        stream: S,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
        strategy: IS,
    ) -> Self {
        let strategy_poll_state = strategy.poll_state_new();

        Self {
            stream: Box::pin(stream),
            interrupt_rx,
            interruption_notified: false,
            has_pending: false,
            strategy,
            strategy_poll_state,
        }
    }
}

impl<'rx, S> Stream for InterruptibleStream<'rx, S, FinishCurrent>
where
    S: Stream,
{
    type Item = StreamOutcome<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.interrupt_rx.try_recv() {
            Ok(InterruptSignal) => self.strategy_poll_state = FinishCurrentState::Interrupted,
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }

        match self.strategy_poll_state {
            FinishCurrentState::NotInterrupted => {}
            FinishCurrentState::Interrupted => {
                if !self.has_pending {
                    if self.interruption_notified {
                        return Poll::Ready(None);
                    } else {
                        self.interruption_notified = true;
                        return Poll::Ready(Some(StreamOutcome::InterruptBeforePoll));
                    }
                }
            }
        }

        let poll = self.stream.as_mut().poll_next(cx);
        self.has_pending = poll.is_pending();

        poll.map(|item_opt| {
            item_opt.map(|item| match self.strategy_poll_state {
                FinishCurrentState::NotInterrupted => StreamOutcome::NoInterrupt(item),
                FinishCurrentState::Interrupted => StreamOutcome::InterruptDuringPoll(item),
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

impl<'rx, S> Stream for InterruptibleStream<'rx, S, PollNextN>
where
    S: Stream,
{
    type Item = StreamOutcomeNRemaining<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.strategy_poll_state {
            PollNextNState::NotInterrupted => match self.interrupt_rx.try_recv() {
                Ok(InterruptSignal) => {
                    let n_remaining = if self.has_pending {
                        self.strategy.0 + 1
                    } else {
                        self.strategy.0
                    };

                    self.strategy_poll_state = PollNextNState::Interrupted { n_remaining }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
            },
            PollNextNState::Interrupted { n_remaining } if n_remaining > 0 => {}
            PollNextNState::Interrupted { n_remaining: _ } => {
                if !self.has_pending {
                    return Poll::Ready(None);
                }
            }
        }

        let poll = self.stream.as_mut().poll_next(cx);
        self.has_pending = poll.is_pending();

        poll.map(|item_opt| {
            item_opt.map(|item| match self.strategy_poll_state {
                PollNextNState::NotInterrupted => StreamOutcomeNRemaining::NoInterrupt(item),
                PollNextNState::Interrupted { n_remaining } => {
                    let n_remaining = n_remaining - 1;
                    self.strategy_poll_state = PollNextNState::Interrupted { n_remaining };
                    StreamOutcomeNRemaining::InterruptDuringPoll {
                        value: item,
                        n_remaining,
                    }
                }
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use futures::{future, stream, Stream};
    use tokio::sync::mpsc;

    use crate::{interrupt_strategy::PollNextN, InterruptSignal, InterruptibleStreamExt};

    #[test]
    fn debug() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interruptible_stream = stream::unfold(
            0u32,
            #[cfg_attr(coverage_nightly, coverage(off))]
            |_| future::ready(None::<(u32, u32)>),
        )
        .interruptible(&mut interrupt_rx);

        assert!(format!("{interruptible_stream:?}").starts_with("InterruptibleStream"));
    }

    #[test]
    fn size_hint() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let interruptible_stream = stream::iter([1, 2, 3]).interruptible(&mut interrupt_rx);
        assert_eq!((3, Some(3)), interruptible_stream.size_hint());

        let interruptible_stream =
            stream::iter([1, 2, 3]).interruptible_with(&mut interrupt_rx, PollNextN(2));
        assert_eq!((3, Some(3)), interruptible_stream.size_hint());
    }
}
