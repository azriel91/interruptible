use std::{marker::Unpin, pin::Pin};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use tokio::sync::oneshot::{self, error::TryRecvError};

use crate::{InterruptSignal, OwnedOrMutRef};

#[derive(Debug)]
pub struct InterruptibleFutureResult<'rx, T, E, Fut>
where
    Fut: Future<Output = Result<T, E>>,
{
    /// Underlying future that returns a value and `Result`.
    future: Fut,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, oneshot::Receiver<InterruptSignal>>,
}

impl<'rx, T, E, Fut> InterruptibleFutureResult<'rx, T, E, Fut>
where
    Fut: Future<Output = Result<T, E>>,
{
    /// Returns a new `InterruptibleFutureResult`, wrapping the provided future.
    pub(crate) fn new(
        future: Fut,
        interrupt_rx: OwnedOrMutRef<'rx, oneshot::Receiver<InterruptSignal>>,
    ) -> InterruptibleFutureResult<'rx, T, E, Fut> {
        Self {
            future,
            interrupt_rx,
        }
    }
}

impl<'rx, T, E, Fut> Future for InterruptibleFutureResult<'rx, T, E, Fut>
where
    Fut: Future<Output = Result<T, E>> + Unpin,
    E: From<(T, InterruptSignal)>,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx).map(|result| {
            match self.interrupt_rx.try_recv() {
                Ok(InterruptSignal) => {
                    // Interrupt received, return `Result::Err`
                    let e = match result {
                        Ok(t) => E::from((t, InterruptSignal)),
                        Err(e) => e,
                    };
                    Err(e)
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => {
                    // Interrupt not received, return the future's actual `Result`.
                    result
                }
            }
        })
    }
}
