use futures::stream::Stream;

use crate::InterruptibleStream;

/// Provides the `.interruptible()` method for `Stream`s to stop producing
/// values when an interrupt signal is received.
pub trait StreamExt {
    fn interruptible(self) -> InterruptibleStream<Self>
    where
        Self: Sized;
}

impl<S> StreamExt for S
where
    S: Stream,
{
    fn interruptible(self) -> InterruptibleStream<Self>
    where
        Self: Sized,
    {
        InterruptibleStream::new(self)
    }
}
