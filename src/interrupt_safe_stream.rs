use std::pin::Pin;

use futures::{
    stream::Stream,
    task::{Context, Poll},
};
use stream_cancel::Valved;

use crate::InterruptGuard;

#[derive(Debug)]
pub struct InterruptSafeStream<S> {
    /// Underlying stream that produces values.
    stream_valved: Valved<S>,
}

impl<S> InterruptSafeStream<S>
where
    S: Stream,
{
    /// Returns a new `InterruptSafeStream`, wrapping the provided stream.
    pub(crate) fn new(stream_valved: S) -> InterruptSafeStream<S> {
        let (stream_trigger, stream_valved) = Valved::new(stream_valved);
        let interrupt_guard = InterruptGuard::new(stream_trigger);
        tokio::task::spawn(interrupt_guard.wait_for_signal());

        Self { stream_valved }
    }
}

impl<S> Stream for InterruptSafeStream<S>
where
    S: Stream + std::marker::Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream_valved).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream_valved.size_hint()
    }
}
