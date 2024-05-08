use std::{
    fmt,
    marker::{PhantomData, Unpin},
    ops::ControlFlow,
    pin::Pin,
};

use futures::{
    future::Future,
    task::{Context, Poll},
};
use own::OwnedOrMutRef;
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::InterruptSignal;

pub struct InterruptibleFutureControl<'rx, B, T, Fut> {
    /// Underlying future that returns a value and `ControlFlow`.
    future: Pin<Box<Fut>>,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    /// Marker.
    marker: PhantomData<(B, T)>,
}

impl<'rx, B, T, Fut> fmt::Debug for InterruptibleFutureControl<'rx, B, T, Fut> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleFutureControl")
            .field("future", &"..")
            .field("interrupt_rx", &self.interrupt_rx)
            .field("marker", &self.marker)
            .finish()
    }
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
            future: Box::pin(future),
            interrupt_rx,
            marker: PhantomData,
        }
    }
}

// `B` does not need to be `Unpin` as it is only used in `PhantomData`.
impl<'rx, B, T, Fut> Unpin for InterruptibleFutureControl<'rx, B, T, Fut> where Fut: Unpin {}

impl<'rx, B, T, Fut> Future for InterruptibleFutureControl<'rx, B, T, Fut>
where
    Fut: Future<Output = ControlFlow<B, T>>,
    B: From<(T, InterruptSignal)>,
    Self: Unpin,
{
    type Output = Fut::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.as_mut().poll(cx).map(|control_flow| {
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

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use futures::FutureExt;
    use tokio::sync::mpsc::{self};

    use crate::{interruptible_future_ext::InterruptibleFutureExt, InterruptSignal};

    #[test]
    fn debug() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);

        let interruptible_control = {
            #[cfg_attr(coverage_nightly, coverage(off))]
            async {
                ControlFlow::<InterruptSignal>::Continue(())
            }
        }
        .boxed()
        .interruptible_control(&mut interrupt_rx);

        assert!(format!("{interruptible_control:?}").starts_with("InterruptibleFutureControl"));
    }
}
