use futures::stream::Stream;
use stream_cancel::Valved;
use tokio::sync::oneshot;

use crate::InterruptibleStream;

/// Provides the `.interruptible()` method for `Stream`s to stop producing
/// values when an interrupt signal is received.
pub trait InterruptibleStreamExt {
    fn interruptible(self, interrupt_rx: oneshot::Receiver<()>) -> InterruptibleStream<Self>
    where
        Self: Sized;

    #[cfg(feature = "ctrl_c")]
    fn interruptible_ctrl_c(self) -> InterruptibleStream<Self>
    where
        Self: Sized;
}

impl<S> InterruptibleStreamExt for S
where
    S: Stream,
{
    fn interruptible(self, interrupt_rx: oneshot::Receiver<()>) -> InterruptibleStream<Self>
    where
        Self: Sized,
    {
        let (stream_trigger, stream_valved) = Valved::new(self);
        tokio::task::spawn(async move {
            let (Ok(()) | Err(_)) = interrupt_rx.await;

            drop(stream_trigger);
        });

        InterruptibleStream::new(stream_valved)
    }

    #[cfg(feature = "ctrl_c")]
    fn interruptible_ctrl_c(self) -> InterruptibleStream<Self>
    where
        Self: Sized,
    {
        let (stream_trigger, stream_valved) = Valved::new(self);
        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to initialize signal handler for `SIGINT`.");

            drop(stream_trigger);
        });

        InterruptibleStream::new(stream_valved)
    }
}
