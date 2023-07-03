use std::{marker::Unpin, ops::ControlFlow};

use futures::future::Future;

use crate::{InterruptibleControlFuture, InterruptibleFutureResult};

/// Provides the `.interruptible_control()` and `.interruptible_result()`
/// methods for `Future`s to return [`ControlFlow::Break`] or [`Result::Err`]
/// when an interrupt signal is received.
pub trait FutureExt {
    fn interruptible_control(self) -> InterruptibleControlFuture<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>;

    fn interruptible_result(self) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin;
}

impl<Fut> FutureExt for Fut
where
    Fut: Future,
{
    fn interruptible_control(self) -> InterruptibleControlFuture<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>,
    {
        InterruptibleControlFuture::new(self)
    }

    fn interruptible_result(self) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin,
    {
        InterruptibleFutureResult::new(self)
    }
}
