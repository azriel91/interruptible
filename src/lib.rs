//! Stops a future or stream from producing values when an interrupt signal is
//! received.
//!
//! This wraps a stream with a combination of [`tokio::signal::ctrl_c`] and
//! [`stream-cancel`], and stops a stream from producing values when `Ctrl C` is
//! received.
//!
//! ⚠️ **Important:** On Unix, `future.interruptible_*_ctrl_c()` and
//! `stream.interruptible_ctrl_c()` will set `tokio` to be the handler of all
//! `SIGINT` events, and once a signal handler is registered for a given
//! process, it can never be unregistered.
//!
//! [`stream-cancel`]: https://github.com/jonhoo/stream-cancel
//! [`tokio::signal::ctrl_c`]: https://docs.rs/tokio/latest/tokio/signal/fn.ctrl_c.html

pub use crate::{
    interruptible_future_control::InterruptibleFutureControl,
    interruptible_future_ext::InterruptibleFutureExt,
    interruptible_future_result::InterruptibleFutureResult,
};
#[cfg(feature = "stream")]
pub use crate::{
    interruptible_stream::InterruptibleStream, interruptible_stream_ext::InterruptibleStreamExt,
};

mod interruptible_future_control;
mod interruptible_future_ext;
mod interruptible_future_result;
#[cfg(feature = "stream")]
mod interruptible_stream;
#[cfg(feature = "stream")]
mod interruptible_stream_ext;
