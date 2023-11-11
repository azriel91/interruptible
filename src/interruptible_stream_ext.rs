use futures::stream::Stream;
use tokio::sync::mpsc;

#[cfg(feature = "ctrl_c")]
use tokio::sync::mpsc::error::SendError;

use crate::{
    owned_or_mut_ref::OwnedOrMutRef, InterruptSignal, InterruptStrategy, Interruptibility,
    InterruptibilityState, InterruptibleStream,
};

/// Provides the `.interruptible()` method for `Stream`s to stop producing
/// values when an interrupt signal is received.
pub trait InterruptibleStreamExt {
    /// Overrides this `Stream`'s poll value when an interrupt signal is
    /// received.
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    fn interruptible<'rx>(
        self,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> InterruptibleStream<'rx, 'static, Self>
    where
        Self: Sized;

    /// Wraps a stream to allow it to gracefully stop.
    ///
    /// The stream's items are wrapped with [`PollOutcome`].
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    /// * `interruptibility_state`: Whether interruptibility is supported.
    fn interruptible_with<'rx, 'intx>(
        self,
        interruptibility_state: InterruptibilityState<'rx, 'intx>,
    ) -> InterruptibleStream<'rx, 'intx, Self>
    where
        Self: Sized + 'rx;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_ctrl_c(self) -> InterruptibleStream<'static, 'static, Self>
    where
        Self: Sized;
}

impl<S> InterruptibleStreamExt for S
where
    S: Stream,
{
    fn interruptible<'rx>(
        self,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> InterruptibleStream<'rx, 'static, Self>
    where
        Self: Sized,
    {
        InterruptibleStream::new(
            self,
            Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy: InterruptStrategy::FinishCurrent,
            }
            .into(),
        )
    }

    fn interruptible_with<'rx, 'intx>(
        self,
        interruptibility_state: InterruptibilityState<'rx, 'intx>,
    ) -> InterruptibleStream<'rx, 'intx, Self>
    where
        Self: Sized + 'rx,
    {
        InterruptibleStream::new(self, interruptibility_state)
    }

    #[cfg(feature = "ctrl_c")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn interruptible_ctrl_c(self) -> InterruptibleStream<'static, 'static, Self>
    where
        Self: Sized,
    {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        tokio::task::spawn(
            #[cfg_attr(coverage_nightly, coverage(off))]
            async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to initialize signal handler for `SIGINT`.");

                let (Ok(()) | Err(SendError(InterruptSignal))) =
                    interrupt_tx.send(InterruptSignal).await;
            },
        );

        InterruptibleStream::new(
            self,
            Interruptibility::Interruptible {
                interrupt_rx: interrupt_rx.into(),
                interrupt_strategy: InterruptStrategy::FinishCurrent,
            }
            .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use futures::{stream, StreamExt};
    use tokio::sync::{
        mpsc::{self, error::SendError},
        oneshot,
    };

    use super::InterruptibleStreamExt;
    use crate::{InterruptSignal, Interruptibility, PollOutcome};

    #[tokio::test]
    async fn interrupt_during_future_overrides_stream_return_value() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (interrupt_ready_tx, interrupt_ready_rx) = oneshot::channel::<()>();
        let (interrupted_tx, interrupted_rx) = oneshot::channel::<()>();

        let mut interruptible_stream = stream::unfold(
            (0u32, Some((interrupt_ready_tx, interrupted_rx))),
            move |(n, channel_tx_rx)| async move {
                if let Some((interrupt_ready_tx, interrupted_rx)) = channel_tx_rx {
                    interrupt_ready_tx
                        .send(())
                        .expect("Expected to send to interrupt ready channel.");
                    let () = interrupted_rx
                        .await
                        .expect("Expected to be notified to return value.");
                }
                Some((n, (n + 1, None)))
            },
        )
        .interruptible(interrupt_rx.into());

        let interrupt_task = async {
            interrupt_ready_rx
                .await
                .expect("Expected `interrupt_ready_rx`. to receive message.");
            interrupt_tx
                .send(InterruptSignal)
                .await
                .expect("Expected to send `InterruptSignal`.");
            interrupted_tx
                .send(())
                .expect("Expected to notify future to return value.");
        };

        let (poll_outcome, ()) = tokio::join!(interruptible_stream.next(), interrupt_task);

        assert_eq!(Some(PollOutcome::Interrupted(Some(0u32))), poll_outcome);
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_finish_current_before_start_returns_interrupt_before_poll() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let mut interruptible_stream = stream::unfold(
            (0u32, Some(ready_rx)),
            #[cfg_attr(coverage_nightly, coverage(off))]
            move |(n, ready_rx)| {
                #[cfg_attr(coverage_nightly, coverage(off))]
                async move {
                    if let Some(ready_rx) = ready_rx {
                        let () = ready_rx
                            .await
                            .expect("Expected to be notified to return value.");
                    }
                    Some((n, (n + 1, None)))
                }
            },
        )
        .interruptible_with(Interruptibility::finish_current(interrupt_rx.into()).into());

        interrupt_tx
            .send(InterruptSignal)
            .await
            .expect("Expected to send `InterruptSignal`.");
        ready_tx
            .send(())
            .expect("Expected to notify future to return value.");

        assert_eq!(
            Some(PollOutcome::Interrupted(None)),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_finish_current_during_future_overrides_stream_return_value() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (interrupt_ready_tx, interrupt_ready_rx) = oneshot::channel::<()>();
        let (interrupted_tx, interrupted_rx) = oneshot::channel::<()>();

        let mut interruptible_stream = stream::unfold(
            (0u32, Some((interrupt_ready_tx, interrupted_rx))),
            move |(n, channel_tx_rx)| async move {
                if let Some((interrupt_ready_tx, interrupted_rx)) = channel_tx_rx {
                    interrupt_ready_tx
                        .send(())
                        .expect("Expected to send to interrupt ready channel.");
                    let () = interrupted_rx
                        .await
                        .expect("Expected to be notified to return value.");
                }
                Some((n, (n + 1, None)))
            },
        )
        .interruptible_with(Interruptibility::finish_current(interrupt_rx.into()).into());

        let interrupt_task = async {
            interrupt_ready_rx
                .await
                .expect("Expected `interrupt_ready_rx`. to receive message.");
            interrupt_tx
                .send(InterruptSignal)
                .await
                .expect("Expected to send `InterruptSignal`.");
            interrupted_tx
                .send(())
                .expect("Expected to notify future to return value.");
        };

        let (poll_outcome, ()) = tokio::join!(interruptible_stream.next(), interrupt_task);

        assert_eq!(Some(PollOutcome::Interrupted(Some(0u32))), poll_outcome);
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_before_start_returns_n_items() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (interrupted_tx, interrupted_rx) = oneshot::channel::<()>();

        let mut interruptible_stream = stream::unfold(
            (0u32, Some(interrupted_rx)),
            move |(n, interrupted_rx)| async move {
                if let Some(interrupted_rx) = interrupted_rx {
                    let () = interrupted_rx
                        .await
                        .expect("Expected to be notified to return value.");
                }
                if n < 3 {
                    Some((n, (n + 1, None)))
                } else {
                    None
                }
            },
        )
        .interruptible_with(Interruptibility::poll_next_n(interrupt_rx.into(), 2).into());

        interrupt_tx
            .send(InterruptSignal)
            .await
            .expect("Expected to send `InterruptSignal`.");
        interrupted_tx
            .send(())
            .expect("Expected to notify future to return value.");

        assert_eq!(
            Some(PollOutcome::NoInterrupt(0u32)),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(PollOutcome::Interrupted(None)),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_returns_n_items_variant_interrupt_before() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let mut interruptible_stream = stream::unfold(0u32, move |n| async move {
            if n < 3 { Some((n, n + 1)) } else { None }
        })
        .interruptible_with(Interruptibility::poll_next_n(interrupt_rx.into(), 2).into());

        interrupt_tx
            .send(InterruptSignal)
            .await
            .expect("Expected to send `InterruptSignal`.");

        // interruption polled here
        assert_eq!(
            Some(PollOutcome::NoInterrupt(0u32)),
            interruptible_stream.next().await
        );
        // interruption also polled here, so `None` is returned
        assert_eq!(
            Some(PollOutcome::Interrupted(None)),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_returns_n_items_variant_interrupt_between() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (interrupt_ready_tx, mut interrupt_ready_rx) = mpsc::channel::<()>(2);
        let (interrupted_tx, interrupted_rx) = oneshot::channel::<()>();

        let mut interruptible_stream = stream::unfold(
            (0u32, Some((interrupt_ready_tx, interrupted_rx))),
            move |(n, channel_tx_rx)| async move {
                if n == 0 {
                    return Some((0, (1, channel_tx_rx)));
                }
                if let Some((interrupt_ready_tx, interrupted_rx)) = channel_tx_rx {
                    interrupt_ready_tx
                        .send(())
                        .await
                        .expect("Expected to send to interrupt ready channel.");
                    let () = interrupted_rx
                        .await
                        .expect("Expected to be notified to return value.");
                }
                if n < 3 {
                    Some((n, (n + 1, None)))
                } else {
                    None
                }
            },
        )
        .interruptible_with(Interruptibility::poll_next_n(interrupt_rx.into(), 2).into());

        let interrupt_task = async {
            interrupt_ready_rx
                .recv()
                .await
                .expect("Expected `interrupt_ready_rx`. to receive message.");
            interrupt_tx
                .send(InterruptSignal)
                .await
                .expect("Expected to send `InterruptSignal`.");
            interrupted_tx
                .send(())
                .expect("Expected to notify future to return value.");
        };

        let ((poll_outcome_first, poll_outcome_second), ()) = tokio::join!(
            async {
                (
                    interruptible_stream.next().await,
                    interruptible_stream.next().await,
                )
            },
            interrupt_task
        );

        // First item is not interrupted.
        assert_eq!(Some(PollOutcome::NoInterrupt(0u32)), poll_outcome_first);
        // Second item is not interrupted, uses 1 of `PollNextN`.
        assert_eq!(Some(PollOutcome::NoInterrupt(1u32)), poll_outcome_second);
        // Third item is `None`, as `PollNextN`'s second value is used up by the
        // interruption.
        assert_eq!(
            Some(PollOutcome::Interrupted(None)),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_returns_no_interrupt_when_not_interrupted() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let mut interruptible_stream = stream::unfold(0u32, move |n| async move {
            if n < 3 { Some((n, n + 1)) } else { None }
        })
        .interruptible_with(Interruptibility::poll_next_n(interrupt_rx.into(), 1).into());

        assert_eq!(
            Some(PollOutcome::NoInterrupt(0u32)),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(PollOutcome::NoInterrupt(1u32)),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(PollOutcome::NoInterrupt(2u32)),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_after_stream_completes_does_not_override_value() {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let mut interruptible_stream = stream::unfold(0u32, move |n| async move {
            if n < 3 { Some((n, n + 1)) } else { None }
        })
        .interruptible(interrupt_rx.into());

        let poll_outcome = interruptible_stream.next().await;

        let (Ok(()) | Err(SendError(InterruptSignal))) = interrupt_tx.send(InterruptSignal).await;

        assert_eq!(Some(PollOutcome::NoInterrupt(0)), poll_outcome);
    }
}
