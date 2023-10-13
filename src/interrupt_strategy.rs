//! How to poll an underlying stream when an interruption is received.
use std::fmt::Debug;

/// On interrupt, wait for the current future's to complete and yield its
/// output, but do not poll the underlying stream for any more futures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FinishCurrent;

/// On interrupt, continue polling the stream for the next `n` futures.
///
/// `n` is an upper bound, so fewer than `n` futures may be yielded if the
/// underlying stream ends early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PollNextN(pub u32);

/// Marker trait for interrupt strategy variants.
pub trait InterruptStrategyT: Clone + Copy + Debug + PartialEq + Eq {}

impl InterruptStrategyT for FinishCurrent {}
impl InterruptStrategyT for PollNextN {}
