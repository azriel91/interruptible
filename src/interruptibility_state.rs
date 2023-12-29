use tokio::sync::mpsc::error::TryRecvError;

use crate::{InterruptSignal, InterruptStrategy, Interruptibility, OwnedOrMutRef};

/// Whether interruptibility is supported, and number of times interrupt signals
/// have been received.
#[derive(Debug)]
pub struct InterruptibilityState<'rx, 'intx> {
    /// Specifies interruptibility support of the application.
    pub(crate) interruptibility: Interruptibility<'rx>,
    /// Number of times an interrupt signal has been received.
    pub(crate) poll_since_interrupt_count: OwnedOrMutRef<'intx, u64>,
    /// Whether previously an interrupt signal has been received.
    pub(crate) interrupt_signal_received: OwnedOrMutRef<'intx, Option<InterruptSignal>>,
}

impl<'rx> InterruptibilityState<'rx, 'static> {
    /// Returns a new `InterruptibilityState`.
    pub fn new(interruptibility: Interruptibility<'rx>) -> Self {
        Self {
            interruptibility,
            poll_since_interrupt_count: OwnedOrMutRef::Owned(0),
            interrupt_signal_received: OwnedOrMutRef::Owned(None),
        }
    }
}

impl<'rx, 'intx> InterruptibilityState<'rx, 'intx> {
    /// Reborrows this `InterruptibilityState` with a shorter lifetime.
    pub fn reborrow(&mut self) -> InterruptibilityState<'_, '_> {
        let interruptibility = match &mut self.interruptibility {
            Interruptibility::NonInterruptible => Interruptibility::NonInterruptible,
            Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy,
            } => Interruptibility::Interruptible {
                interrupt_rx: OwnedOrMutRef::MutRef(&mut *interrupt_rx),
                interrupt_strategy: *interrupt_strategy,
            },
        };

        let poll_since_interrupt_count =
            OwnedOrMutRef::MutRef(&mut *self.poll_since_interrupt_count);
        let interrupt_signal_received = OwnedOrMutRef::MutRef(&mut *self.interrupt_signal_received);

        InterruptibilityState {
            interruptibility,
            poll_since_interrupt_count,
            interrupt_signal_received,
        }
    }

    /// Tests if an item should be interrupted.
    ///
    /// If an interrupt signal has not been received, this returns `None`.
    ///
    /// When an interrupt signal has been received, this may still return `None`
    /// if the interrupt strategy allows for additional items to be completed
    /// before the process should be stopped.
    ///
    /// # Parameters
    ///
    /// * `increment_item_count`: Set this to `true` if this poll is for a new
    ///   item, `false` if polling for interruptions while an item is being
    ///   streamed.
    ///
    /// **Note:** It is important that this is called once per `Stream::Item` or
    /// `Future`, as the invocation of this method is used to track state for
    /// strategies like `PollNextN`.
    pub fn item_interrupt_poll(&mut self, increment_item_count: bool) -> Option<InterruptSignal> {
        match &mut self.interruptibility {
            Interruptibility::NonInterruptible => None,
            Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy,
            } => {
                if self.interrupt_signal_received.is_none() {
                    match interrupt_rx.try_recv() {
                        Ok(interrupt_signal) => {
                            *self.interrupt_signal_received = Some(interrupt_signal);
                        }
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
                    }
                }

                match *self.interrupt_signal_received {
                    Some(interrupt_signal) => {
                        if increment_item_count {
                            *self.poll_since_interrupt_count += 1;
                        }
                        let poll_since_interrupt_count = *self.poll_since_interrupt_count;
                        Self::interrupt_signal_based_on_strategy(
                            interrupt_strategy,
                            interrupt_signal,
                            poll_since_interrupt_count,
                        )
                    }
                    None => None,
                }
            }
        }
    }

    /// Returns the `InterruptSignal` if the strategy's threshold is reached.
    ///
    /// * For `IgnoreInterruptions`, this always returns `None`.
    /// * For `FinishCurrent`, this always returns `Some(interrupt_signal)`.
    /// * For `PollNextN`, this returns `Some(interrupt_signal)` if
    ///   `poll_since_interrupt_count` equals or is greater than `n`.
    fn interrupt_signal_based_on_strategy(
        interrupt_strategy: &mut InterruptStrategy,
        interrupt_signal: InterruptSignal,
        poll_since_interrupt_count: u64,
    ) -> Option<InterruptSignal> {
        match interrupt_strategy {
            InterruptStrategy::IgnoreInterruptions => {
                // Even if we received a signal, don't indicate so.
                None
            }
            InterruptStrategy::FinishCurrent => Some(interrupt_signal),
            InterruptStrategy::PollNextN(n) => {
                if poll_since_interrupt_count == *n {
                    Some(interrupt_signal)
                } else {
                    None
                }
            }
        }
    }

    /// Returns the interruptibility support of the application.
    pub fn interruptibility(&self) -> &Interruptibility<'rx> {
        &self.interruptibility
    }

    /// Returns a mutable reference to the interruptibility support of the
    /// application.
    pub fn interruptibility_mut(&mut self) -> &mut Interruptibility<'rx> {
        &mut self.interruptibility
    }

    /// Returns the number of times `item_interrupt_poll` is called since the
    /// very first interrupt signal was received.
    ///
    /// If the interruption signal has not been received, this returns 0.
    pub fn poll_since_interrupt_count(&self) -> u64 {
        *self.poll_since_interrupt_count
    }
}

impl<'rx> From<Interruptibility<'rx>> for InterruptibilityState<'rx, 'static> {
    /// Returns a new `InterruptibilityState`.
    fn from(interruptibility: Interruptibility<'rx>) -> Self {
        Self::new(interruptibility)
    }
}

#[cfg(test)]
mod tests {
    use super::InterruptibilityState;
    use crate::Interruptibility;

    #[test]
    fn debug() {
        let interruptibility_state = InterruptibilityState::new(Interruptibility::NonInterruptible);

        assert_eq!(
            "InterruptibilityState { \
                interruptibility: NonInterruptible, \
                poll_since_interrupt_count: Owned(0), \
                interrupt_signal_received: Owned(None) \
            }",
            format!("{interruptibility_state:?}")
        );
    }

    #[test]
    fn from() {
        let interruptibility_state =
            InterruptibilityState::from(Interruptibility::NonInterruptible);

        assert_eq!(
            "InterruptibilityState { \
                interruptibility: NonInterruptible, \
                poll_since_interrupt_count: Owned(0), \
                interrupt_signal_received: Owned(None) \
            }",
            format!("{interruptibility_state:?}")
        );
    }
}
