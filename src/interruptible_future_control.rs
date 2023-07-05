use std::{ops::ControlFlow, pin::Pin};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use tokio::sync::oneshot::{error::TryRecvError, Receiver};

#[derive(Debug)]
pub struct InterruptibleFutureControl<Fut> {
    /// Underlying future that returns a value and `ControlFlow`.
    future: Fut,
    /// Receiver for interrupt signal.
    interrupt_rx: Receiver<()>,
}

impl<Fut> InterruptibleFutureControl<Fut>
where
    Fut: Future<Output = ControlFlow<(), ()>>,
{
    /// Returns a new `InterruptibleFutureControl`, wrapping the provided
    /// future.
    pub(crate) fn new(future: Fut, interrupt_rx: Receiver<()>) -> InterruptibleFutureControl<Fut> {
        Self {
            future,
            interrupt_rx,
        }
    }
}

impl<Fut> Future for InterruptibleFutureControl<Fut>
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
