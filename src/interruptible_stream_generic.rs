use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::stream::Stream;

use crate::{
    interrupt_strategy::PollNextN, InterruptibleStream, StreamOutcome, StreamOutcomeNRemaining,
};

/// Wrapper for any stream to commonize the stream item to a common type.
#[derive(Debug)]
pub struct InterruptibleStreamGeneric<S> {
    /// Underlying stream that produces values.
    stream: S,
}

impl<S> InterruptibleStreamGeneric<S> {
    /// Returns a new `InterruptibleStreamGeneric`.
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<'rx, S> Stream for InterruptibleStreamGeneric<InterruptibleStream<'rx, S, PollNextN>>
where
    S: Stream + Unpin,
{
    type Item = StreamOutcome<S::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx).map(|item_opt| {
            item_opt.map(|item| match item {
                StreamOutcomeNRemaining::InterruptBeforePoll => StreamOutcome::InterruptBeforePoll,
                StreamOutcomeNRemaining::InterruptDuringPoll {
                    value,
                    n_remaining: _,
                } => StreamOutcome::InterruptDuringPoll(value),
                StreamOutcomeNRemaining::NoInterrupt(value) => StreamOutcome::NoInterrupt(value),
            })
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
