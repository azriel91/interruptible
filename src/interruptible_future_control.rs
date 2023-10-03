use std::{
    marker::{PhantomData, Unpin},
    ops::ControlFlow,
    pin::Pin,
};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use tokio::sync::oneshot::{self, error::TryRecvError};

use crate::InterruptSignal;

#[derive(Debug)]
pub struct InterruptibleFutureControl<B, T, Fut> {
    /// Underlying future that returns a value and `ControlFlow`.
    future: Fut,
    /// Receiver for interrupt signal.
    interrupt_rx: oneshot::Receiver<InterruptSignal>,
    /// Marker.
    marker: PhantomData<(B, T)>,
}

impl<B, T, Fut> InterruptibleFutureControl<B, T, Fut>
where
    Fut: Future<Output = ControlFlow<B, T>>,
{
    /// Returns a new `InterruptibleFutureControl`, wrapping the provided
    /// future.
    pub(crate) fn new(future: Fut, interrupt_rx: oneshot::Receiver<InterruptSignal>) -> Self {
        Self {
            future,
            interrupt_rx,
            marker: PhantomData,
        }
    }
}

// `B` does not need to be `Unpin` as it is only used in `PhantomData`.
impl<B, T, Fut> Unpin for InterruptibleFutureControl<B, T, Fut> where Fut: Unpin {}

impl<B, T, Fut> Future for InterruptibleFutureControl<B, T, Fut>
where
    Fut: Future<Output = ControlFlow<B, T>> + Unpin,
    B: From<InterruptSignal>,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx).map(|control_flow| {
            match self.interrupt_rx.try_recv() {
                Ok(InterruptSignal) => {
                    // Interrupt received, return `ControlFlow::Break`
                    ControlFlow::Break(B::from(InterruptSignal))
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => {
                    // Interrupt not received, return the future's actual `ControlFlow`.
                    control_flow
                }
            }
        })
    }
}
