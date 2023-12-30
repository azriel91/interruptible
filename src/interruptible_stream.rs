use std::{fmt, pin::Pin};

use futures::{
    stream::Stream,
    task::{Context, Poll},
};

use crate::{InterruptSignal, InterruptibilityState, PollOutcome};

/// Wrapper around a `Stream` that adds interruptible behaviour.
pub struct InterruptibleStream<'rx, 'intx, S> {
    /// Underlying stream that produces values.
    stream: Pin<Box<S>>,
    /// Receiver for interrupt signal.
    interruptibility_state: InterruptibilityState<'rx, 'intx>,
    /// If an interruption is received, has this stream returned a
    /// `ControlFlow::Break` in `poll_next`.
    interrupted_and_notified: bool,
    /// Whether the underlying stream has an item pending.
    ///
    /// Because `Stream`s are polled in order to poll the underlying future that
    /// produces the item, we need to track whether the underlying stream has
    /// been polled and returned `Poll::Pending`, so we should be intentional
    /// whether or not to interrupt a stream that we have work in progress.
    has_pending: bool,
    /// Whether an interrupt signal has previously been received from the
    /// `InterruptibilityState`.
    ///
    /// This needs to be held separately from the `interruptibility_state`,
    /// because we don't want to re-poll the underlying `interrupt_rx` channel.
    interrupt_signal: Option<InterruptSignal>,
    /// Whether the pending item is counted in the existing interrupt signal.
    item_polled_is_counted: bool,
}

impl<'rx, 'intx, S> fmt::Debug for InterruptibleStream<'rx, 'intx, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleStream")
            .field("stream", &"..")
            .field("interruptibility_state", &self.interruptibility_state)
            .field("interrupted_and_notified", &self.interrupted_and_notified)
            .field("has_pending", &self.has_pending)
            .field("interrupt_signal", &self.interrupt_signal)
            .field("item_polled_is_counted", &self.item_polled_is_counted)
            .finish()
    }
}

impl<'rx, 'intx, S> InterruptibleStream<'rx, 'intx, S>
where
    S: Stream,
{
    /// Returns a new `InterruptibleStream`, wrapping the provided stream.
    pub(crate) fn new(
        stream: S,
        interruptibility_state: InterruptibilityState<'rx, 'intx>,
    ) -> Self {
        Self {
            stream: Box::pin(stream),
            interruptibility_state,
            interrupted_and_notified: false,
            has_pending: false,
            interrupt_signal: None,
            item_polled_is_counted: false,
        }
    }
}

impl<'rx, 'intx, S> Stream for InterruptibleStream<'rx, 'intx, S>
where
    S: Stream,
{
    type Item = PollOutcome<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.interrupted_and_notified {
            return Poll::Ready(None);
        }

        if self.has_pending {
            let poll = self.stream.as_mut().poll_next(cx);
            self.has_pending = poll.is_pending();

            // Poll for interrupt within an item.
            if self.interrupt_signal.is_none() {
                let item_needs_counting = !self.item_polled_is_counted;
                let poll_count_before = self.interruptibility_state.poll_since_interrupt_count();
                self.interrupt_signal = self
                    .interruptibility_state
                    .item_interrupt_poll(item_needs_counting);
                let poll_count_after = self.interruptibility_state.poll_since_interrupt_count();

                self.item_polled_is_counted = poll_count_before != poll_count_after;
            }

            poll.map(|item_opt| {
                item_opt.map(|item| match self.interrupt_signal {
                    Some(_interrupt_signal) => {
                        self.interrupted_and_notified = true;
                        PollOutcome::Interrupted(Some(item))
                    }
                    None => PollOutcome::NoInterrupt(item),
                })
            })
        } else {
            // Poll for interrupt signals between items.
            let poll_count_before = self.interruptibility_state.poll_since_interrupt_count();
            self.interrupt_signal = match self.interruptibility_state.item_interrupt_poll(true) {
                Some(_interrupt_signal) => {
                    self.interrupted_and_notified = true;

                    return Poll::Ready(Some(PollOutcome::Interrupted(None)));
                }
                None => None,
            };
            let poll_count_after = self.interruptibility_state.poll_since_interrupt_count();

            let poll = self.stream.as_mut().poll_next(cx);
            self.has_pending = poll.is_pending();

            self.item_polled_is_counted = poll_count_before != poll_count_after;

            poll.map(|item_opt| {
                item_opt.map(|item| match self.interrupt_signal {
                    Some(_interrupt_signal) => {
                        self.interrupted_and_notified = true;
                        PollOutcome::Interrupted(Some(item))
                    }
                    None => PollOutcome::NoInterrupt(item),
                })
            })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use futures::{future, stream, Stream};
    use tokio::sync::mpsc;

    use crate::{InterruptSignal, InterruptibleStreamExt};

    #[test]
    fn debug() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interruptible_stream = stream::unfold(
            0u32,
            #[cfg_attr(coverage_nightly, coverage(off))]
            |_| future::ready(None::<(u32, u32)>),
        )
        .interruptible(interrupt_rx.into());

        assert!(format!("{interruptible_stream:?}").starts_with("InterruptibleStream"));
    }

    #[test]
    fn size_hint() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let interruptible_stream = stream::iter([1, 2, 3]).interruptible(interrupt_rx.into());
        assert_eq!((3, Some(3)), interruptible_stream.size_hint());
    }
}
