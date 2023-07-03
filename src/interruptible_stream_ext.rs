use futures::stream::Stream;

use crate::InterruptibleStream;

/// Provides the `.interruptible()` method for `Stream`s to stop producing
/// values when an interrupt signal is received.
pub trait InterruptibleStreamExt {
    fn interruptible(self) -> InterruptibleStream<Self>
    where
        Self: Sized;
}

impl<S> InterruptibleStreamExt for S
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
