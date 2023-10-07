use std::{marker::Unpin, ops::ControlFlow};

use futures::future::Future;
use tokio::sync::oneshot;

use crate::{InterruptSignal, InterruptibleFutureControl, InterruptibleFutureResult};

/// Provides the `.interruptible_control()` and `.interruptible_result()`
/// methods for `Future`s to return [`ControlFlow::Break`] or [`Result::Err`]
/// when an interrupt signal is received.
pub trait InterruptibleFutureExt<'rx, B, T> {
    fn interruptible_control(
        self,
        interrupt_rx: &'rx mut oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>;

    fn interruptible_result(
        self,
        interrupt_rx: &'rx mut oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>;

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
        interrupt_rx: &'rx mut oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>,
    {
        InterruptibleFutureControl::new(self, interrupt_rx.into())
    }

    fn interruptible_result(
        self,
        interrupt_rx: &'rx mut oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin,
    {
        InterruptibleFutureResult::new(self, interrupt_rx.into())
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<'rx, B, T, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, T>>,
        B: From<(T, InterruptSignal)>,
    {
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<InterruptSignal>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(InterruptSignal)) = interrupt_tx.send(InterruptSignal);
        });

        InterruptibleFutureControl::new(self, interrupt_rx.into())
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<'rx, T, B, Self>
    where
        Self: Sized + Future<Output = Result<T, B>> + Unpin,
    {
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<InterruptSignal>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(InterruptSignal)) = interrupt_tx.send(InterruptSignal);
        });

        InterruptibleFutureResult::new(self, interrupt_rx.into())
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use futures::FutureExt;
    use tokio::{join, sync::oneshot};

    use super::InterruptibleFutureExt;
    use crate::InterruptSignal;

    #[tokio::test]
    async fn interrupt_overrides_control_future_return_value() {
        let (interrupt_tx, mut interrupt_rx) = oneshot::channel::<InterruptSignal>();
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
        let (interrupt_tx, mut interrupt_rx) = oneshot::channel::<InterruptSignal>();

        let interruptible_control = async { ControlFlow::<InterruptSignal, ()>::Continue(()) }
            .boxed()
            .interruptible_control(&mut interrupt_rx);

        let interrupter = async move {
            let (Ok(()) | Err(InterruptSignal)) = interrupt_tx.send(InterruptSignal);
        };

        let (control_flow, ()) = join!(interruptible_control, interrupter);

        assert_eq!(ControlFlow::Continue(()), control_flow);
    }
}
