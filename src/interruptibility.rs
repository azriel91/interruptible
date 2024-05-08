use own::OwnedOrMutRef;
use tokio::sync::mpsc;

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
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
        /// How to poll the underlying stream when an interruption is received.
        interrupt_strategy: InterruptStrategy,
    },
}

impl<'rx> Interruptibility<'rx> {
    /// Returns a new `Interruptibility::Interruptible`.
    pub fn new(
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
        interrupt_strategy: InterruptStrategy,
    ) -> Self {
        Self::Interruptible {
            interrupt_rx,
            interrupt_strategy,
        }
    }

    /// Returns a new `Interruptibility::Interruptible` using the
    /// `InterruptStrategy::FinishCurrent` strategy.
    pub fn finish_current(
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> Self {
        Self::new(interrupt_rx, InterruptStrategy::FinishCurrent)
    }

    /// Returns a new `Interruptibility::Interruptible` using the
    /// `InterruptStrategy::FinishCurrent` strategy.
    pub fn poll_next_n(
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
        n: u64,
    ) -> Self {
        Self::new(interrupt_rx, InterruptStrategy::PollNextN(n))
    }

    /// Reborrows this `Interruptibility` with a shorter lifetime.
    pub fn reborrow(&mut self) -> Interruptibility<'_> {
        match self {
            Interruptibility::NonInterruptible => Interruptibility::NonInterruptible,
            Interruptibility::Interruptible {
                interrupt_rx,
                interrupt_strategy,
            } => Interruptibility::Interruptible {
                interrupt_rx: interrupt_rx.reborrow(),
                interrupt_strategy: *interrupt_strategy,
            },
        }
    }

    /// Returns the `InterruptStrategy` if present.
    pub fn interrupt_strategy(&self) -> Option<InterruptStrategy> {
        match self {
            Interruptibility::NonInterruptible => None,
            Interruptibility::Interruptible {
                interrupt_rx: _,
                interrupt_strategy,
            } => Some(*interrupt_strategy),
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::Interruptibility;
    use crate::{InterruptSignal, InterruptStrategy};

    #[test]
    fn debug() {
        assert_eq!(
            "NonInterruptible",
            format!("{:?}", Interruptibility::NonInterruptible)
        );
    }

    #[test]
    fn reborrow() {
        let (_interrupt_tx, interrupt_rx) = mpsc::channel(16);
        let interrupt_strategy = InterruptStrategy::PollNextN(3);
        let mut interruptibility = Interruptibility::Interruptible {
            interrupt_rx: interrupt_rx.into(),
            interrupt_strategy,
        };

        let _interruptibility = interruptibility.reborrow();
    }

    #[test]
    fn interrupt_strategy() {
        assert_eq!(
            None,
            Interruptibility::NonInterruptible.interrupt_strategy()
        );

        let (_interrupt_tx, interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interruptibility = Interruptibility::finish_current(interrupt_rx.into());

        assert_eq!(
            Some(InterruptStrategy::FinishCurrent),
            interruptibility.interrupt_strategy()
        );
    }
}
