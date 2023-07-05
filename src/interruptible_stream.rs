use std::pin::Pin;

use futures::{
    stream::Stream,
    task::{Context, Poll},
};
use stream_cancel::Valved;

#[derive(Debug)]
pub struct InterruptibleStream<S> {
    /// Underlying stream that produces values.
    stream_valved: Valved<S>,
}

impl<S> InterruptibleStream<S>
where
    S: Stream,
{
    /// Returns a new `InterruptibleStream`, wrapping the provided stream.
    pub(crate) fn new(stream_valved: Valved<S>) -> InterruptibleStream<S> {
        Self { stream_valved }
    }
}

impl<S> Stream for InterruptibleStream<S>
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
