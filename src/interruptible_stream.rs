use std::{ops::ControlFlow, pin::Pin};

use futures::{
    stream::Stream,
    task::{Context, Poll},
};
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{InterruptSignal, OwnedOrMutRef};

#[derive(Debug)]
pub struct InterruptibleStream<'rx, S> {
    /// Underlying stream that produces values.
    stream: Pin<Box<S>>,
    /// Receiver for interrupt signal.
    interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
}

impl<'rx, S> InterruptibleStream<'rx, S>
where
    S: Stream,
{
    /// Returns a new `InterruptibleStream`, wrapping the provided stream.
    pub(crate) fn new(
        stream: S,
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> InterruptibleStream<S> {
        Self {
            stream: Box::pin(stream),
            interrupt_rx,
        }
    }
}

impl<'rx, S> Stream for InterruptibleStream<'rx, S>
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
