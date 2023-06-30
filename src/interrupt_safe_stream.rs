use std::pin::Pin;

use futures::{
    stream::Stream,
    task::{Context, Poll},
};

#[derive(Debug)]
pub struct InterruptSafeStream<S> {
    /// Underlying stream that produces values.
    inner_stream: S,
}

impl<S> InterruptSafeStream<S> {
    /// Returns a new `InterruptSafeStream`, wrapping the provided stream.
    pub(crate) fn new(inner_stream: S) -> InterruptSafeStream<S> {
        Self { inner_stream }
    }
}

impl<S> Stream for InterruptSafeStream<S>
where
    S: Stream + std::marker::Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner_stream).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner_stream.size_hint()
    }
}
