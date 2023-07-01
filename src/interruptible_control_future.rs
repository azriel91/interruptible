use std::{ops::ControlFlow, pin::Pin};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use tokio::sync::oneshot::{error::TryRecvError, Receiver};

#[derive(Debug)]
pub struct InterruptibleControlFuture<Fut> {
    /// Underlying future that returns a value and `ControlFlow`.
    future: Fut,
    /// Receiver for interrupt signal.
    interrupt_rx: Receiver<()>,
}

impl<Fut> InterruptibleControlFuture<Fut>
where
    Fut: Future<Output = ControlFlow<(), ()>>,
{
    /// Returns a new `InterruptibleControlFuture`, wrapping the provided
    /// future.
    pub(crate) fn new(future: Fut) -> InterruptibleControlFuture<Fut> {
        let (interrupt_tx, interrupt_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for SIGINT");

            let (Ok(()) | Err(())) = interrupt_tx.send(());
        });

        Self {
            future,
            interrupt_rx,
        }
    }
}

impl<Fut> Future for InterruptibleControlFuture<Fut>
where
    Fut: Future<Output = ControlFlow<(), ()>> + std::marker::Unpin,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx).map(|control_flow| {
            match self.interrupt_rx.try_recv() {
                Ok(()) => {
                    // Interrupt received, return `ControlFlow::Break`
                    ControlFlow::Break(())
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => {
                    // Interrupt not received, return the future's actual `ControlFlow`.
                    control_flow
                }
            }
        })
    }
}
