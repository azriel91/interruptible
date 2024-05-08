use std::{
    fmt,
    marker::{PhantomData, Unpin},
    pin::Pin,
};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use own::OwnedOrMutRef;
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::InterruptSignal;

pub struct InterruptibleFutureResult<'rx, T, E, Fut> {
    /// Underlying future that returns a value and `Result`.
    future: Pin<Box<Fut>>,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    /// Marker.
    marker: PhantomData<(T, E)>,
}

impl<'rx, T, E, Fut> fmt::Debug for InterruptibleFutureResult<'rx, T, E, Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleFutureResult")
            .field("future", &"..")
            .field("interrupt_rx", &self.interrupt_rx)
            .finish()
    }
}

impl<'rx, T, E, Fut> InterruptibleFutureResult<'rx, T, E, Fut>
where
    Fut: Future<Output = Result<T, E>>,
{
    /// Returns a new `InterruptibleFutureResult`, wrapping the provided future.
    pub(crate) fn new(
        future: Fut,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> InterruptibleFutureResult<'rx, T, E, Fut> {
        Self {
            future: Box::pin(future),
            interrupt_rx,
            marker: PhantomData,
        }
    }
}

impl<'rx, T, E, Fut> Future for InterruptibleFutureResult<'rx, T, E, Fut>
where
    Fut: Future<Output = Result<T, E>>,
    E: From<(T, InterruptSignal)>,
    Self: Unpin,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx).map(|result| {
            match self.interrupt_rx.try_recv() {
                Ok(InterruptSignal) => {
                    // Interrupt received, return `Result::Err`
                    let e = match result {
                        Ok(t) => E::from((t, InterruptSignal)),
                        Err(e) => e,
                    };
                    Err(e)
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {
                    // Interrupt not received, return the future's actual `Result`.
                    result
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use tokio::sync::mpsc;

    use crate::{interruptible_future_ext::InterruptibleFutureExt, InterruptSignal};

    #[test]
    fn debug() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let interruptible_result = {
            #[cfg_attr(coverage_nightly, coverage(off))]
            async {
                Result::<_, InterruptSignal>::Ok(())
            }
        }
        .boxed()
        .interruptible_result(&mut interrupt_rx);

        assert!(format!("{interruptible_result:?}").starts_with("InterruptibleFutureResult"));
    }
}
