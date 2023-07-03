//! Stops a stream from producing values when an interrupt signal is received.
//!
//! This wraps a stream with a combination of [`tokio::signal::ctrl_c`] and
//! [`stream-cancel`], and stops a stream from producing values when `Ctrl C` is
//! received.
//!
//! ⚠️ **Important:** On Unix, `stream.interruptible()` will cause `tokio` to
//! handle all `SIGINT` events, as once a signal handler is registered for a
//! given process, it can never be unregistered.
//!
//! [`stream-cancel`]: https://github.com/jonhoo/stream-cancel
//! [`tokio::signal::ctrl_c`]: https://docs.rs/tokio/latest/tokio/signal/fn.ctrl_c.html

pub use crate::{
    future_ext::FutureExt, interruptible_control_future::InterruptibleControlFuture,
    interruptible_future_result::InterruptibleFutureResult,
    interruptible_stream::InterruptibleStream, stream_ext::StreamExt,
};

pub(crate) use crate::interrupt_guard::InterruptGuard;

mod future_ext;
mod interrupt_guard;
mod interruptible_control_future;
mod interruptible_future_result;
mod interruptible_stream;
mod stream_ext;
