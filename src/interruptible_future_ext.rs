use std::{marker::Unpin, ops::ControlFlow};

use futures::future::Future;
use tokio::sync::mpsc;

#[cfg(feature = "ctrl_c")]
use tokio::sync::mpsc::error::SendError;

use crate::{InterruptSignal, InterruptibleFutureControl, InterruptibleFutureResult};

/// Provides the `.interruptible_control()` and `.interruptible_result()`
/// methods for `Future`s to return [`ControlFlow::Break`] or [`Result::Err`]
/// when an interrupt signal is received.
pub trait InterruptibleFutureExt<'rx, B, T> {
    /// Overrides this `Future`'s control flow when an interrupt signal is
    /// received.
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    fn interruptible_control(
        self,
        interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>;

    /// Overrides this `Future`'s result when an interrupt signal is received.
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    fn interruptible_result(
        self,
        interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin;

    /// Overrides this `Future`'s control flow when an interrupt signal is
    /// received through `Ctrl C`.
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>;

    /// Overrides this `Future`'s result when an interrupt signal is received
    /// through `Ctrl C`.
    ///
    /// # Parameters
    ///
    /// * `interrupt_rx`: Channel receiver of the interrupt signal.
    #[cfg(feature = "ctrl_c")]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin;
}

impl<'rx, B, T, Fut> InterruptibleFutureExt<'rx, B, T> for Fut
where
    Fut: Future,
{
    fn interruptible_control(
        self,
        interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>,
    {
        InterruptibleFutureControl::new(self, interrupt_rx.into())
    }

    fn interruptible_result(
        self,
        interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin,
    {
        InterruptibleFutureResult::new(self, interrupt_rx.into())
    }

    #[cfg(feature = "ctrl_c")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>,
    {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        tokio::task::spawn(
            #[cfg_attr(coverage_nightly, coverage(off))]
            async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to initialize signal handler for SIGINT");

                let (Ok(()) | Err(SendError(InterruptSignal))) =
                    interrupt_tx.send(InterruptSignal).await;
            },
        );

        InterruptibleFutureControl::new(self, interrupt_rx.into())
    }

    #[cfg(feature = "ctrl_c")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin,
    {
        let (interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        tokio::task::spawn(
            #[cfg_attr(coverage_nightly, coverage(off))]
            async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to initialize signal handler for SIGINT");

                let (Ok(()) | Err(SendError(InterruptSignal))) =
                    interrupt_tx.send(InterruptSignal).await;
            },
        );

        InterruptibleFutureResult::new(self, interrupt_rx.into())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use futures::FutureExt;
    use tokio::{
        join,
        sync::{
            mpsc::{self, error::SendError},
            oneshot,
        },
    };

    use super::InterruptibleFutureExt;
    use crate::InterruptSignal;

    #[tokio::test]
    async fn interrupt_overrides_control_future_return_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let interruptible_control = async {
            let () = ready_rx.await.expect("Expected to be notified to start.");
            ControlFlow::Continue(())
        }
        .boxed()
        .interruptible_control(&mut interrupt_rx);

        let interrupter = async move {
            interrupt_tx
                .send(InterruptSignal)
                .await
                .expect("Expected to send `InterruptSignal`.");
            ready_tx
                .send(())
                .expect("Expected to notify sleep to start.");
        };

        let (control_flow, ()) = join!(interruptible_control, interrupter);

        assert_eq!(ControlFlow::Break(InterruptSignal), control_flow);
    }

    #[tokio::test]
    async fn interrupt_after_control_future_completes_does_not_override_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let interruptible_control = async { ControlFlow::<InterruptSignal, ()>::Continue(()) }
            .boxed()
            .interruptible_control(&mut interrupt_rx);

        let interrupter = async move {
            let (Ok(()) | Err(SendError(InterruptSignal))) =
                interrupt_tx.send(InterruptSignal).await;
        };

        let (control_flow, ()) = join!(interruptible_control, interrupter);

        assert_eq!(ControlFlow::Continue(()), control_flow);
    }

    #[tokio::test]
    async fn interrupt_overrides_result_future_return_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let interruptible_result = async {
            let () = ready_rx.await.expect("Expected to be notified to start.");
            Ok(())
        }
        .boxed()
        .interruptible_result(&mut interrupt_rx);

        let interrupter = async move {
            interrupt_tx
                .send(InterruptSignal)
                .await
                .expect("Expected to send `InterruptSignal`.");
            ready_tx
                .send(())
                .expect("Expected to notify sleep to start.");
        };

        let (result_flow, ()) = join!(interruptible_result, interrupter);

        assert_eq!(Err(InterruptSignal), result_flow);
    }

    #[tokio::test]
    async fn interrupt_after_result_future_completes_does_not_override_value() {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let interruptible_result = async { Result::<(), InterruptSignal>::Ok(()) }
            .boxed()
            .interruptible_result(&mut interrupt_rx);

        let interrupter = async move {
            let (Ok(()) | Err(SendError(InterruptSignal))) =
                interrupt_tx.send(InterruptSignal).await;
        };

        let (result_flow, ()) = join!(interruptible_result, interrupter);

        assert_eq!(Ok(()), result_flow);
    }
}
