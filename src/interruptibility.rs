use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{InterruptSignal, InterruptStrategy};

/// Specifies interruptibility support of the application.
///
/// This is the dynamic / non-type-parameterized version of interruptibility.
#[derive(Debug)]
pub enum Interruptibility<'rx> {
    /// Interruptions are not supported.
    NonInterruptible,
    /// Interruptions are supported.
    Interruptible {
        /// Channel receiver of the interrupt signal.
        interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
        /// How to poll the underlying stream when an interruption is received.
        interrupt_strategy: InterruptStrategy,
    },
}

impl<'rx> Interruptibility<'rx> {
    /// Reborrows this `Interruptibility` with a shorter lifetime.
    pub fn reborrow(&mut self) -> Interruptibility<'_> {
        match self {
            Interruptibility::NonInterruptible => Interruptibility::NonInterruptible,
            Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy,
            } => Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy: *interrupt_strategy,
            },
        }
    }

    /// Polls if any `InterruptSignal`s have been sent since the last poll.
    ///
    /// This method is intended for checking for interruptions during regular
    /// control flow that is not tied to a stream.
    ///
    /// # Design Note
    ///
    /// The current implementation does not use the interrupt strategy, and
    /// simply checks if an `InterruptSignal` has been sent.
    ///
    /// A possible evolution of this crate is to store the interrupt strategy's
    /// state with the `Interruptibility`, and use that state across different
    /// streams / interruptibility checks.
    pub fn interrupt_poll(&mut self) -> Option<InterruptSignal> {
        match self {
            Interruptibility::NonInterruptible => None,
            Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy: _,
            } => match interrupt_rx.try_recv() {
                Ok(interrupt_signal) => Some(interrupt_signal),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::Interruptibility;
    use crate::InterruptStrategy;

    #[test]
    fn debug() {
        assert_eq!(
            "NonInterruptible",
            format!("{:?}", Interruptibility::NonInterruptible)
        );
    }

    #[test]
    fn reborrow() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel(16);
        let interrupt_rx = &mut interrupt_rx;
        let interrupt_strategy = InterruptStrategy::PollNextN(3);
        let mut interruptibility = Interruptibility::Interruptible {
            interrupt_rx,
            interrupt_strategy,
        };

        let _interruptibility = interruptibility.reborrow();
    }
}
