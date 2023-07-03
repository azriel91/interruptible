use std::{marker::Unpin, pin::Pin};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use tokio::sync::oneshot::{error::TryRecvError, Receiver};

#[derive(Debug)]
pub struct InterruptibleFutureResult<Fut>
where
    Fut: Future<Output = Result<(), ()>>,
{
    /// Underlying future that returns a value and `Result`.
    future: Fut,
    /// Receiver for interrupt signal.
    interrupt_rx: Receiver<()>,
}

impl<Fut> InterruptibleFutureResult<Fut>
where
    Fut: Future<Output = Result<(), ()>>,
{
    /// Returns a new `InterruptibleFutureResult`, wrapping the provided future.
    pub(crate) fn new(future: Fut) -> InterruptibleFutureResult<Fut> {
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

impl<Fut> Future for InterruptibleFutureResult<Fut>
where
    Fut: Future<Output = Result<(), ()>> + Unpin,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx).map(|result| {
            match self.interrupt_rx.try_recv() {
                Ok(()) => {
                    // Interrupt received, return `Result::Err`
                    Result::Err(())
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => {
                    // Interrupt not received, return the future's actual `Result`.
                    result
                }
            }
        })
    }
}
