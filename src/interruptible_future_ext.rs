use std::{marker::Unpin, ops::ControlFlow};

use futures::future::Future;
use tokio::sync::oneshot;

use crate::{InterruptSignal, InterruptibleFutureControl, InterruptibleFutureResult};

/// Provides the `.interruptible_control()` and `.interruptible_result()`
/// methods for `Future`s to return [`ControlFlow::Break`] or [`Result::Err`]
/// when an interrupt signal is received.
pub trait InterruptibleFutureExt<B> {
    fn interruptible_control(
        self,
        interrupt_rx: oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureControl<B, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, ()>>,
        B: From<InterruptSignal>;

    fn interruptible_result(
        self,
        interrupt_rx: oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureResult<B, Self>
    where
        Self: Sized + Future<Output = Result<B, ()>> + Unpin;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<B, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, ()>>,
        B: From<InterruptSignal>;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<B, Self>
    where
        Self: Sized + Future<Output = Result<B, ()>> + Unpin;
}

impl<B, Fut> InterruptibleFutureExt<B> for Fut
where
    Fut: Future,
{
    fn interruptible_control(
        self,
        interrupt_rx: oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureControl<B, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, ()>>,
        B: From<InterruptSignal>,
    {
        InterruptibleFutureControl::new(self, interrupt_rx)
    }

    fn interruptible_result(
        self,
        interrupt_rx: oneshot::Receiver<InterruptSignal>,
    ) -> InterruptibleFutureResult<B, Self>
    where
        Self: Sized + Future<Output = Result<B, ()>> + Unpin,
    {
        InterruptibleFutureResult::new(self, interrupt_rx)
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<B, Self>
    where
        Self: Sized + Future<Output = ControlFlow<B, ()>>,
        B: From<InterruptSignal>,
    {
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<InterruptSignal>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(InterruptSignal)) = interrupt_tx.send(InterruptSignal);
        });

        InterruptibleFutureControl::new(self, interrupt_rx)
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<B, Self>
    where
        Self: Sized + Future<Output = Result<B, ()>> + Unpin,
    {
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<InterruptSignal>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(InterruptSignal)) = interrupt_tx.send(InterruptSignal);
        });

        InterruptibleFutureResult::new(self, interrupt_rx)
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
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<InterruptSignal>();
        let (ready_tx, ready_rx) = oneshot::channel::<()>();

        let interruptible_control = async {
            let () = ready_rx.await.expect("Expected to be notified to start.");
            ControlFlow::Continue(())
        }
        .boxed()
        .interruptible_control(interrupt_rx);

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
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<InterruptSignal>();

        let interruptible_control = async { ControlFlow::<InterruptSignal, ()>::Continue(()) }
            .boxed()
            .interruptible_control(interrupt_rx);

        let interrupter = async move {
            let (Ok(()) | Err(InterruptSignal)) = interrupt_tx.send(InterruptSignal);
        };

        let (control_flow, ()) = join!(interruptible_control, interrupter);

        assert_eq!(ControlFlow::Continue(()), control_flow);
    }
}
