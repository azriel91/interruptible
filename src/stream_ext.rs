use futures::stream::Stream;

use crate::InterruptSafeStream;

/// Provides the `.interrupt_safe()` method for `Stream`s to stop producing
/// values when an interrupt signal is received.
pub trait StreamExt {
    fn interrupt_safe(self) -> InterruptSafeStream<Self>
    where
        Self: Sized;
}

impl<S> StreamExt for S
where
    S: Stream,
{
    fn interrupt_safe(self) -> InterruptSafeStream<Self>
    where
        Self: Sized,
    {
        InterruptSafeStream::new(self)
    }
}
