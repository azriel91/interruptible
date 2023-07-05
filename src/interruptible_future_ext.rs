use std::{marker::Unpin, ops::ControlFlow};

use futures::future::Future;
use tokio::sync::oneshot;

use crate::{InterruptibleFutureControl, InterruptibleFutureResult};

/// Provides the `.interruptible_control()` and `.interruptible_result()`
/// methods for `Future`s to return [`ControlFlow::Break`] or [`Result::Err`]
/// when an interrupt signal is received.
pub trait InterruptibleFutureExt {
    fn interruptible_control(
        self,
        interrupt_rx: oneshot::Receiver<()>,
    ) -> InterruptibleFutureControl<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>;

    fn interruptible_result(
        self,
        interrupt_rx: oneshot::Receiver<()>,
    ) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin;
}

impl<Fut> InterruptibleFutureExt for Fut
where
    Fut: Future,
{
    fn interruptible_control(
        self,
        interrupt_rx: oneshot::Receiver<()>,
    ) -> InterruptibleFutureControl<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>,
    {
        InterruptibleFutureControl::new(self, interrupt_rx)
    }

    fn interruptible_result(
        self,
        interrupt_rx: oneshot::Receiver<()>,
    ) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin,
    {
        InterruptibleFutureResult::new(self, interrupt_rx)
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_control_ctrl_c(self) -> InterruptibleFutureControl<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>,
    {
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<()>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(())) = interrupt_tx.send(());
        });

        InterruptibleFutureControl::new(self, interrupt_rx)
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_result_ctrl_c(self) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin,
    {
        let (interrupt_tx, interrupt_rx) = oneshot::channel::<()>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(())) = interrupt_tx.send(());
        });

        InterruptibleFutureResult::new(self, interrupt_rx)
    }
}
