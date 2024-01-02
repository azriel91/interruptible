use std::{fmt, pin::Pin};

use futures::{
    future::Future,
    stream::Stream,
    task::{Context, Poll},
    FutureExt,
};

use crate::{InterruptSignal, InterruptibilityState, PollOutcome};

/// Wrapper around a `Stream` that adds interruptible behaviour.
pub struct InterruptibleStream<'rx, 'intx, S, Fut> {
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
    future_pending: Option<Fut>,
    /// Whether an interrupt signal has previously been received from the
    /// `InterruptibilityState`.
    ///
    /// This needs to be held separately from the `interruptibility_state`,
    /// because we don't want to re-poll the underlying `interrupt_rx` channel.
    interrupt_signal: Option<InterruptSignal>,
    /// Whether the pending item is counted in the existing interrupt signal.
    item_polled_is_counted: bool,
}

impl<'rx, 'intx, S, Fut> fmt::Debug for InterruptibleStream<'rx, 'intx, S, Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleStream")
            .field("stream", &"..")
            .field("interruptibility_state", &self.interruptibility_state)
            .field("interrupted_and_notified", &self.interrupted_and_notified)
            .field(
                "future_pending",
                if self.future_pending.is_some() {
                    &Some(())
                } else {
                    &None::<()>
                },
            )
            .field("interrupt_signal", &self.interrupt_signal)
            .field("item_polled_is_counted", &self.item_polled_is_counted)
            .finish()
    }
}

impl<'rx, 'intx, S, Fut> InterruptibleStream<'rx, 'intx, S, Fut>
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
            future_pending: None,
            interrupt_signal: None,
            item_polled_is_counted: false,
        }
    }

    fn interrupt_check(&mut self) {
        // Check for interrupt within an item.
        if self.interrupt_signal.is_none() {
            let item_needs_counting = !self.item_polled_is_counted;
            let poll_count_before = self.interruptibility_state.poll_since_interrupt_count();
            self.interrupt_signal = self
                .interruptibility_state
                .item_interrupt_poll(item_needs_counting);
            let poll_count_after = self.interruptibility_state.poll_since_interrupt_count();

            self.item_polled_is_counted = poll_count_before != poll_count_after;
        }
    }

    fn poll_future_item(
        &mut self,
        mut future_pending: Fut,
        cx: &mut Context<'_>,
    ) -> Poll<Option<PollOutcome<<Fut as Future>::Output>>>
    where
        Fut: Future + Unpin,
    {
        let poll_future = Pin::new(&mut future_pending).poll(cx);
        match poll_future {
            Poll::Ready(item) => match self.interrupt_signal {
                Some(_interrupt_signal) => {
                    self.future_pending = None;
                    self.interrupted_and_notified = true;
                    self.fn_interrupt_poll_run();
                    Poll::Ready(Some(PollOutcome::Interrupted(Some(item))))
                }
                None => {
                    self.future_pending = None;
                    Poll::Ready(Some(PollOutcome::NoInterrupt(item)))
                }
            },
            Poll::Pending => {
                self.future_pending = Some(future_pending);
                Poll::Pending
            }
        }
    }

    /// Runs `fn_interrupt_poll` if it exists.
    fn fn_interrupt_poll_run(&mut self) {
        if let Some(fn_interrupt_poll) = self.interruptibility_state.fn_interrupt_poll_item() {
            (*fn_interrupt_poll)();
        }
    }
}

impl<'rx, 'intx, S, Fut> Stream for InterruptibleStream<'rx, 'intx, S, Fut>
where
    S: Stream<Item = Fut>,
    Fut: Future + Unpin,
{
    // Accept a `Stream<Item = Future<Output = T>>` rather than `Stream<Item = T>`.
    //
    // So that `InterruptibleStream` knows that it's producing a future, and thus
    // can poll that future when it is polled, rather than `T` that may be a future,
    // whose delegate stream polls the `T: Future`.
    type Item = PollOutcome<Fut::Output>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.interrupted_and_notified {
            return Poll::Ready(None);
        }

        self.interrupt_check();
        if let Some(future_pending) = self.future_pending.take() {
            self.poll_future_item(future_pending, cx)
        } else {
            match self.interrupt_signal {
                Some(_interrupt_signal) => {
                    self.interrupted_and_notified = true;
                    self.fn_interrupt_poll_run();
                    Poll::Ready(Some(PollOutcome::Interrupted(None)))
                }
                None => {
                    let poll_stream = self.stream.as_mut().poll_next(cx);
                    match poll_stream {
                        Poll::Ready(item_opt) => {
                            self.future_pending = item_opt;
                        }
                        Poll::Pending => {
                            self.future_pending = None;
                        }
                    }

                    Poll::Pending
                }
            }
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
