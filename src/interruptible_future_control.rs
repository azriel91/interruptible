use std::{
    marker::{PhantomData, Unpin},
    ops::ControlFlow,
    pin::Pin,
};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{InterruptSignal, OwnedOrMutRef};

#[derive(Debug)]
pub struct InterruptibleFutureControl<'rx, B, T, Fut> {
    /// Underlying future that returns a value and `ControlFlow`.
    future: Fut,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    /// Marker.
    marker: PhantomData<(B, T)>,
}

impl<'rx, B, T, Fut> InterruptibleFutureControl<'rx, B, T, Fut>
where
    Fut: Future<Output = ControlFlow<B, T>>,
{
    /// Returns a new `InterruptibleFutureControl`, wrapping the provided
    /// future.
    pub(crate) fn new(
        future: Fut,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> Self {
        Self {
            future,
            interrupt_rx,
            marker: PhantomData,
        }
    }
}

// `B` does not need to be `Unpin` as it is only used in `PhantomData`.
impl<'rx, B, T, Fut> Unpin for InterruptibleFutureControl<'rx, B, T, Fut> where Fut: Unpin {}

impl<'rx, B, T, Fut> Future for InterruptibleFutureControl<'rx, B, T, Fut>
where
    Fut: Future<Output = ControlFlow<B, T>> + Unpin,
    B: From<(T, InterruptSignal)>,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx).map(|control_flow| {
            match self.interrupt_rx.try_recv() {
                Ok(InterruptSignal) => {
                    // Interrupt received, return `ControlFlow::Break`, plus whatever the
                    // `control_flow` returned.
                    match control_flow {
                        ControlFlow::Continue(t) => {
                            ControlFlow::Break(B::from((t, InterruptSignal)))
                        }
                        ControlFlow::Break(b) => ControlFlow::Break(b),
                    }
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {
                    // Interrupt not received, return the future's actual `ControlFlow`.
                    control_flow
                }
            }
        })
    }
}
