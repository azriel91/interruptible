use std::{marker::Unpin, ops::ControlFlow};

use futures::future::Future;

use crate::{InterruptibleFutureControl, InterruptibleFutureResult};

/// Provides the `.interruptible_control()` and `.interruptible_result()`
/// methods for `Future`s to return [`ControlFlow::Break`] or [`Result::Err`]
/// when an interrupt signal is received.
pub trait InterruptibleFutureExt {
    fn interruptible_control(self) -> InterruptibleFutureControl<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>;

    fn interruptible_result(self) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin;
}

impl<Fut> InterruptibleFutureExt for Fut
where
    Fut: Future,
{
    fn interruptible_control(self) -> InterruptibleFutureControl<Self>
    where
        Self: Sized + Future<Output = ControlFlow<(), ()>>,
    {
        InterruptibleFutureControl::new(self)
    }

    fn interruptible_result(self) -> InterruptibleFutureResult<Self>
    where
        Self: Sized + Future<Output = Result<(), ()>> + Unpin,
    {
        InterruptibleFutureResult::new(self)
    }
}
