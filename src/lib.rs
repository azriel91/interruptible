#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

//! Stops a future producer or stream from producing values when interrupted.
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
//! interruptible = "0.0.3"
//!
//! # Enables `InterruptibleStreamExt`
//! interruptible = { version = "0.0.3", features = ["stream"] }
//!
//! # Enables:
//! #
//! # * `InterruptibleFutureExt::{interruptible_control_ctrl_c, interruptible_result_ctrl_c}`
//! # * `InterruptibleStreamExt::interruptible_ctrl_c` if the `"stream"` feature is also enabled.
//! interruptible = { version = "0.0.3", features = ["ctrl_c"] }
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
//! ## `InterruptibleStreamExt` with `features = ["stream"]`
//!
//! Stops a stream from producing values when an interrupt signal is received.
//!
//! See the [`interrupt_strategy`] module for different ways the stream
//! interruption can be handled.
//!
//! ```rust
//! # #[cfg(not(feature = "stream"))]
//! # fn main() {}
//! #
//! #[cfg(feature = "stream")]
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() {
//! #
//! # use futures::{stream, StreamExt};
//! # use tokio::sync::mpsc;
//! #
//! # use interruptible::{
//! #     InterruptibleStreamExt, InterruptSignal, Interruptibility, PollOutcome,
//! # };
//! #
//!     let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
//!
//!     let mut interruptible_stream =
//!         stream::unfold(0u32, move |n| async move { Some((n, n + 1)) })
//!             .interruptible(interrupt_rx.into());
//!
//!     interrupt_tx
//!         .send(InterruptSignal)
//!         .await
//!         .expect("Expected to send `InterruptSignal`.");
//!
//!     assert_eq!(
//!         Some(PollOutcome::Interrupted(None)),
//!         interruptible_stream.next().await
//!     );
//!     assert_eq!(None, interruptible_stream.next().await);
//! # }
//! ```
//!
//! [`interrupt_strategy`]: https://docs.rs/interruptible/latest/interrupt_strategy/index.html

pub use crate::{
    interrupt_signal::InterruptSignal, interruptible_future_control::InterruptibleFutureControl,
    interruptible_future_ext::InterruptibleFutureExt,
    interruptible_future_result::InterruptibleFutureResult, owned_or_mut_ref::OwnedOrMutRef,
};

mod interrupt_signal;
mod interruptible_future_control;
mod interruptible_future_ext;
mod interruptible_future_result;
mod owned_or_mut_ref;

#[cfg(feature = "stream")]
pub use crate::{
    interrupt_strategy::InterruptStrategy, interruptibility::Interruptibility,
    interruptibility_state::InterruptibilityState, interruptible_stream::InterruptibleStream,
    interruptible_stream_ext::InterruptibleStreamExt, poll_outcome::PollOutcome,
};

#[cfg(feature = "stream")]
mod interrupt_strategy;
#[cfg(feature = "stream")]
mod interruptibility;
#[cfg(feature = "stream")]
mod interruptibility_state;
#[cfg(feature = "stream")]
mod interruptible_stream;
#[cfg(feature = "stream")]
mod interruptible_stream_ext;
#[cfg(feature = "stream")]
mod poll_outcome;
