#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! Stops a future producer or stream from producing values when an interrupt
//! signal is received.
//!
//! For a future that returns either `Result<T, ()>` or `ControlFlow<T, ()>`,
//! calling `fut.interruptible_*(tx)` causes the returned value to be `Err(())`
//! or `Break(T)` if an interruption signal is received *while* that future is
//! executing.
//!
//! This means the future is progressed to completion, but the return value
//! signals the producer to stop yielding futures.
//!
//! For a stream, when the interrupt signal is received, the current future is
//! run to completion, but the stream is not polled for the next item.
//!
//! # Usage
//!
//! Add the following to `Cargo.toml`
//!
//! ```toml
//! interruptible = "0.0.1"
//!
//! # Enables:
//! #
//! # * `InterruptibleFutureExt::{interruptible_control_ctrl_c, interruptible_result_ctrl_c}`
//! # * `InterruptibleStreamExt::interruptible_ctrl_c` if the `"stream"` feature is also enabled.
//! interruptible = { version = "0.0.2", features = ["ctrl_c"] }
//!
//! # Enables `InterruptibleStreamExt`
//! interruptible = { version = "0.0.2", features = ["stream"] }
//! ```
//!
//! # Examples
//!
//! ## `Future<Output = ControlFlow<B, C>>`
//!
//! ```rust
//! use std::ops::ControlFlow;
//!
//! use futures::FutureExt;
//! use tokio::{
//!     join,
//!     sync::{mpsc, oneshot},
//! };
//!
//! use interruptible::{InterruptSignal, InterruptibleFutureExt};
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() {
//!     let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
//!     let (ready_tx, ready_rx) = oneshot::channel::<()>();
//!
//!     let interruptible_control = async {
//!         let () = ready_rx.await.expect("Expected to be notified to start.");
//!         ControlFlow::Continue(())
//!     }
//!     .boxed()
//!     .interruptible_control(&mut interrupt_rx);
//!
//!     let interrupter = async move {
//!         interrupt_tx
//!             .send(InterruptSignal)
//!             .await
//!             .expect("Expected to send `InterruptSignal`.");
//!         ready_tx
//!             .send(())
//!             .expect("Expected to notify sleep to start.");
//!     };
//!
//!     let (control_flow, ()) = join!(interruptible_control, interrupter);
//!
//!     assert_eq!(ControlFlow::Break(InterruptSignal), control_flow);
//! }
//! ```
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
    interrupt_signal::InterruptSignal, interruptible_future_control::InterruptibleFutureControl,
    interruptible_future_ext::InterruptibleFutureExt,
    interruptible_future_result::InterruptibleFutureResult,
};

pub(crate) use owned_or_mut_ref::OwnedOrMutRef;

mod interrupt_signal;
mod interruptible_future_control;
mod interruptible_future_ext;
mod interruptible_future_result;
mod owned_or_mut_ref;

#[cfg(feature = "stream")]
pub use crate::{
    interrupt_strategy::InterruptStrategyT, interruptible_stream::InterruptibleStream,
    interruptible_stream_ext::InterruptibleStreamExt,
};

#[cfg(feature = "stream")]
pub mod interrupt_strategy;
#[cfg(feature = "stream")]
mod interruptible_stream;
#[cfg(feature = "stream")]
mod interruptible_stream_ext;
