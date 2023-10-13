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
    use std::ops::ControlFlow;

    use futures::{stream, StreamExt};
    use tokio::sync::{
        mpsc::{self, error::SendError},
        oneshot,
    };

    use super::InterruptibleStreamExt;
    use crate::InterruptSignal;

    #[tokio::test]
    async fn interrupt_overrides_stream_return_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let mut interruptible_stream =
            stream::unfold((0u32, Some(ready_rx)), move |(n, ready_rx)| async move {
                if let Some(ready_rx) = ready_rx {
                    let () = ready_rx.await.expect("Expected to be notified to start.");
                }
                if n < 3 {
                    Some((n, (n + 1, None)))
                } else {
                    None
                }
            })
            .interruptible(&mut interrupt_rx);

        interrupt_tx
            .send(InterruptSignal)
            .await
            .expect("Expected to send `InterruptSignal`.");
        ready_tx
            .send(())
            .expect("Expected to notify sleep to start.");

        let control_flow = interruptible_stream.next().await;

        assert_eq!(
            Some(ControlFlow::Break((InterruptSignal, 0u32))),
            control_flow
        );
    }

    #[tokio::test]
    async fn interrupt_after_stream_completes_does_not_override_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let mut interruptible_stream = stream::unfold(0u32, move |n| async move {
            if n < 3 { Some((n, n + 1)) } else { None }
        })
        .interruptible(&mut interrupt_rx);

        let control_flow = interruptible_stream.next().await;

        let (Ok(()) | Err(SendError(InterruptSignal))) = interrupt_tx.send(InterruptSignal).await;

        assert_eq!(Some(ControlFlow::Continue(0)), control_flow);
    }
}
