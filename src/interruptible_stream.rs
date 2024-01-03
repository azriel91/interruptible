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

    fn interrupt_check(&mut self) {
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
        cx: &mut Context<'_>,
    ) -> Poll<Option<PollOutcome<<S as Stream>::Item>>>
    where
        S: Stream,
    {
        let poll_stream = self.stream.as_mut().poll_next(cx);
        match poll_stream {
            Poll::Ready(item_opt) => {
                self.has_pending = false;
                match self.interrupt_signal {
                    Some(_interrupt_signal) => {
                        self.interrupted_and_notified = true;
                        self.fn_interrupt_poll_run();
                        Poll::Ready(Some(PollOutcome::Interrupted(item_opt)))
                    }
                    None => match item_opt {
                        Some(item) => Poll::Ready(Some(PollOutcome::NoInterrupt(item))),
                        None => Poll::Ready(None),
                    },
                }
            }

            // Notably we cannot send any information that we are interrupted through
            // `Poll::Pending`; but only in `Poll::Ready`.
            //
            // However, consumers are able to know immediately through the `fn_interrupt_activate`
            // hook set on `InterruptibilityState`.

            // self.has_pending = true; // retained from existing state.
            Poll::Pending => Poll::Pending,
        }
    }

    /// Runs `fn_interrupt_poll` if it exists.
    fn fn_interrupt_poll_run(&mut self) {
        if let Some(fn_interrupt_poll) = self.interruptibility_state.fn_interrupt_poll_item() {
            (*fn_interrupt_poll)();
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

        self.interrupt_check();
        if self.has_pending {
            self.poll_future_item(cx)
        } else {
            match self.interrupt_signal {
                Some(_interrupt_signal) => {
                    self.interrupted_and_notified = true;
                    self.fn_interrupt_poll_run();
                    Poll::Ready(Some(PollOutcome::Interrupted(None)))
                }
                None => {
                    let poll_stream = self.stream.as_mut().poll_next(cx);
                    self.has_pending = poll_stream.is_pending();
                    poll_stream.map(|item_opt| item_opt.map(|item| PollOutcome::NoInterrupt(item)))
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
    use futures::{future, stream, Stream, StreamExt};
    use tokio::sync::mpsc::{self, error::TryRecvError};

    use crate::{InterruptSignal, InterruptibilityState, InterruptibleStreamExt, PollOutcome};

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

    #[tokio::test]
    async fn fn_interrupt_poll_item_is_run_only_when_poll_returns_ready()
    -> Result<(), Box<dyn std::error::Error>> {
        macro_rules! interruptible_stream {
            ($interruptibility_state:ident) => {
                stream::unfold(0u32, move |n| async move {
                    if n < 3 { Some((n, n + 1)) } else { None }
                })
                .interruptible_with($interruptibility_state)
            };
        }

        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interrupt_rx = &mut interrupt_rx;

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let (interrupt_poll_item_tx, mut interrupt_poll_item_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(100)
                .expect("Expected to send value.");
        }));
        interruptibility_state.set_fn_interrupt_poll_item(Some(|| {
            interrupt_poll_item_tx
                .try_send(100)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        let mut interruptible_stream = interruptible_stream!(interruptibility_state);
        let _ = interruptible_stream.next().await;
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        assert_eq!(Err(TryRecvError::Empty), interrupt_poll_item_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let (interrupt_poll_item_tx, mut interrupt_poll_item_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(101)
                .expect("Expected to send value.");
        }));
        interruptibility_state.set_fn_interrupt_poll_item(Some(|| {
            interrupt_poll_item_tx
                .try_send(101)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        let mut interruptible_stream = interruptible_stream!(interruptibility_state);
        let _ = interruptible_stream.next().await;
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        assert_eq!(Err(TryRecvError::Empty), interrupt_poll_item_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let (interrupt_poll_item_tx, mut interrupt_poll_item_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_finish_current(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(102)
                .expect("Expected to send value.");
        }));
        interruptibility_state.set_fn_interrupt_poll_item(Some(|| {
            interrupt_poll_item_tx
                .try_send(102)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        let mut interruptible_stream = interruptible_stream!(interruptibility_state);
        let _ = interruptible_stream.next().await;
        assert_eq!(Ok(102), interrupt_activate_rx.try_recv());
        assert_eq!(Ok(102), interrupt_poll_item_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let (interrupt_poll_item_tx, mut interrupt_poll_item_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(103)
                .expect("Expected to send value.");
        }));
        interruptibility_state.set_fn_interrupt_poll_item(Some(|| {
            interrupt_poll_item_tx
                .try_send(103)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        let mut interruptible_stream = interruptible_stream!(interruptibility_state);
        let _ = interruptible_stream.next().await;
        let _ = interruptible_stream.next().await;
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        assert_eq!(Err(TryRecvError::Empty), interrupt_poll_item_rx.try_recv());
        let _ = interruptible_stream.next().await;
        assert_eq!(Ok(103), interrupt_activate_rx.try_recv());
        assert_eq!(Ok(103), interrupt_poll_item_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let (interrupt_poll_item_tx, mut interrupt_poll_item_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(104)
                .expect("Expected to send value.");
        }));
        interruptibility_state.set_fn_interrupt_poll_item(Some(|| {
            interrupt_poll_item_tx
                .try_send(104)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        let mut interruptible_stream = interruptible_stream!(interruptibility_state);
        assert_eq!(
            Some(PollOutcome::NoInterrupt(0)),
            interruptible_stream.next().await
        );
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        assert_eq!(Err(TryRecvError::Empty), interrupt_poll_item_rx.try_recv());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        assert_eq!(Err(TryRecvError::Empty), interrupt_poll_item_rx.try_recv());
        assert_eq!(
            Some(PollOutcome::NoInterrupt(1)),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(PollOutcome::Interrupted(None)),
            interruptible_stream.next().await
        );
        assert_eq!(Ok(104), interrupt_activate_rx.try_recv());
        assert_eq!(Ok(104), interrupt_poll_item_rx.try_recv());

        Ok(())
    }
}
