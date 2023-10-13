use std::{fmt, ops::ControlFlow, pin::Pin};

use futures::{
    stream::Stream,
    task::{Context, Poll},
};
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{
    interrupt_strategy::FinishCurrent, InterruptSignal, InterruptStrategyT, OwnedOrMutRef,
};

/// Wrapper around a `Stream` that adds interruptible behaviour.
pub struct InterruptibleStream<'rx, S, IS> {
    /// Underlying stream that produces values.
    stream: Pin<Box<S>>,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    /// How to poll the underlying stream when an interruption is received.
    strategy: IS,
}

impl<'rx, S, IS> fmt::Debug for InterruptibleStream<'rx, S, IS>
where
    IS: InterruptStrategyT,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibleStream")
            .field("stream", &"..")
            .field("interrupt_rx", &self.interrupt_rx)
            .field("strategy", &self.strategy)
            .finish()
    }
}

impl<'rx, S, IS> InterruptibleStream<'rx, S, IS>
where
    S: Stream,
    IS: InterruptStrategyT,
{
    /// Returns a new `InterruptibleStream`, wrapping the provided stream.
    pub(crate) fn new(
        stream: S,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
        strategy: IS,
    ) -> Self {
        Self {
            stream: Box::pin(stream),
            interrupt_rx,
            strategy,
        }
    }
}

impl<'rx, S> Stream for InterruptibleStream<'rx, S, FinishCurrent>
where
    S: Stream,
{
    type Item = ControlFlow<(InterruptSignal, S::Item), S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx).map(|item_opt| {
            item_opt.map(|item| match self.interrupt_rx.try_recv() {
                Ok(InterruptSignal) => ControlFlow::Break((InterruptSignal, item)),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => {
                    ControlFlow::Continue(item)
                }
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[cfg(test)]
mod tests {
    use futures::{future, stream, Stream};
    use tokio::sync::mpsc;

    use crate::{InterruptSignal, InterruptibleStreamExt};

    #[test]
    fn debug() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interruptible_stream = stream::unfold(
            0u32,
            #[cfg_attr(coverage_nightly, coverage(off))]
            |_| future::ready(None::<(u32, u32)>),
        )
        .interruptible(&mut interrupt_rx);

        assert!(format!("{interruptible_stream:?}").starts_with("InterruptibleStream"));
    }

    #[test]
    fn size_hint() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interruptible_stream = stream::iter([1, 2, 3]).interruptible(&mut interrupt_rx);

        assert_eq!((3, Some(3)), interruptible_stream.size_hint());
    }
}
