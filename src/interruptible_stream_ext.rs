use futures::stream::Stream;
use tokio::sync::mpsc;

#[cfg(feature = "ctrl_c")]
use tokio::sync::mpsc::error::SendError;

use crate::{
    interrupt_strategy::FinishCurrent, InterruptSignal, InterruptStrategyT, InterruptibleStream,
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
    fn interruptible(
        self,
        interrupt_rx: &mut mpsc::Receiver<InterruptSignal>,
    ) -> InterruptibleStream<'_, Self, FinishCurrent>
    where
        Self: Sized;

    /// Overrides this `Stream`'s poll value when an interrupt signal is
    /// received.
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    /// * `interrupt_strategy`: How to poll the underlying stream when an
    ///   interruption is received.
    fn interruptible_with<IS>(
        self,
        interrupt_rx: &mut mpsc::Receiver<InterruptSignal>,
        interrupt_strategy: IS,
    ) -> InterruptibleStream<'_, Self, IS>
    where
        Self: Sized,
        IS: InterruptStrategyT;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_ctrl_c(self) -> InterruptibleStream<'static, Self, FinishCurrent>
    where
        Self: Sized;
}

impl<S> InterruptibleStreamExt for S
where
    S: Stream,
{
    fn interruptible(
        self,
        interrupt_rx: &mut mpsc::Receiver<InterruptSignal>,
    ) -> InterruptibleStream<'_, Self, FinishCurrent>
    where
        Self: Sized,
    {
        InterruptibleStream::new(self, interrupt_rx.into(), FinishCurrent)
    }

    fn interruptible_with<IS>(
        self,
        interrupt_rx: &mut mpsc::Receiver<InterruptSignal>,
        interrupt_strategy: IS,
    ) -> InterruptibleStream<'_, Self, IS>
    where
        Self: Sized,
        IS: InterruptStrategyT,
    {
        InterruptibleStream::new(self, interrupt_rx.into(), interrupt_strategy)
    }

    #[cfg(feature = "ctrl_c")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn interruptible_ctrl_c(self) -> InterruptibleStream<'static, Self, FinishCurrent>
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

        InterruptibleStream::new(self, interrupt_rx.into(), FinishCurrent)
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
    use crate::{
        interrupt_strategy::{FinishCurrent, PollNextN},
        InterruptSignal, StreamOutcome, StreamOutcomeNRemaining,
    };

    #[tokio::test]
    async fn interrupt_during_future_overrides_stream_return_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
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
        .interruptible(&mut interrupt_rx);

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

        let (stream_outcome, ()) = tokio::join!(interruptible_stream.next(), interrupt_task);

        assert_eq!(
            Some(StreamOutcome::InterruptDuringPoll(0u32)),
            stream_outcome
        );
    }

    #[tokio::test]
    async fn interrupt_with_finish_current_before_start_returns_interrupt_before_poll() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
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
        .interruptible_with(&mut interrupt_rx, FinishCurrent);

        interrupt_tx
            .send(InterruptSignal)
            .await
            .expect("Expected to send `InterruptSignal`.");
        ready_tx
            .send(())
            .expect("Expected to notify future to return value.");

        assert_eq!(
            Some(StreamOutcome::InterruptBeforePoll),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_finish_current_during_future_overrides_stream_return_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
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
        .interruptible_with(&mut interrupt_rx, FinishCurrent);

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

        let (stream_outcome, ()) = tokio::join!(interruptible_stream.next(), interrupt_task);

        assert_eq!(
            Some(StreamOutcome::InterruptDuringPoll(0u32)),
            stream_outcome
        );
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_before_start_returns_n_items() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
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
        .interruptible_with(&mut interrupt_rx, PollNextN(2));

        interrupt_tx
            .send(InterruptSignal)
            .await
            .expect("Expected to send `InterruptSignal`.");
        interrupted_tx
            .send(())
            .expect("Expected to notify future to return value.");

        assert_eq!(
            Some(StreamOutcomeNRemaining::InterruptDuringPoll {
                value: 0u32,
                n_remaining: 1
            }),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(StreamOutcomeNRemaining::InterruptDuringPoll {
                value: 1u32,
                n_remaining: 0
            }),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_overrides_stream_return_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
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
                if n < 3 {
                    Some((n, (n + 1, None)))
                } else {
                    None
                }
            },
        )
        .interruptible_with(&mut interrupt_rx, PollNextN(1));

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

        let (stream_outcome_first, ()) = tokio::join!(interruptible_stream.next(), interrupt_task);

        assert_eq!(
            Some(StreamOutcomeNRemaining::InterruptDuringPoll {
                value: 0u32,
                n_remaining: 1
            }),
            stream_outcome_first
        );
        assert_eq!(
            Some(StreamOutcomeNRemaining::InterruptDuringPoll {
                value: 1u32,
                n_remaining: 0
            }),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_with_poll_next_n_returns_no_interrupt_when_not_interrupted() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let mut interruptible_stream = stream::unfold(0u32, move |n| async move {
            if n < 3 { Some((n, n + 1)) } else { None }
        })
        .interruptible_with(&mut interrupt_rx, PollNextN(1));

        assert_eq!(
            Some(StreamOutcomeNRemaining::NoInterrupt(0u32)),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(StreamOutcomeNRemaining::NoInterrupt(1u32)),
            interruptible_stream.next().await
        );
        assert_eq!(
            Some(StreamOutcomeNRemaining::NoInterrupt(2u32)),
            interruptible_stream.next().await
        );
        assert_eq!(None, interruptible_stream.next().await);
    }

    #[tokio::test]
    async fn interrupt_after_stream_completes_does_not_override_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let mut interruptible_stream = stream::unfold(0u32, move |n| async move {
            if n < 3 { Some((n, n + 1)) } else { None }
        })
        .interruptible(&mut interrupt_rx);

        let stream_outcome = interruptible_stream.next().await;

        let (Ok(()) | Err(SendError(InterruptSignal))) = interrupt_tx.send(InterruptSignal).await;

        assert_eq!(Some(StreamOutcome::NoInterrupt(0)), stream_outcome);
    }
}
