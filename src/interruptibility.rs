use tokio::sync::mpsc;

use crate::{InterruptSignal, InterruptStrategy};

/// Interruptibility parameters for a stream.
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
    /// Reborrows this `Interruptiblity` with a shorter lifetime.
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
}

/// Type state for something non-interruptible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonInterruptible;

/// Type state for something interruptible.
#[derive(Debug)]
pub struct Interruptible<'rx, IS> {
    /// Channel receiver of the interrupt signal.
    pub interrupt_rx: &'rx mut mpsc::Receiver<InterruptSignal>,
    /// How to poll the underlying stream when an interruption is received.
    pub interrupt_strategy: IS,
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::{Interruptibility, Interruptible, NonInterruptible};
    use crate::InterruptStrategy;

    #[test]
    fn debug() {
        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel(16);
        let interrupt_rx = &mut interrupt_rx;
        let interrupt_strategy = InterruptStrategy::PollNextN(3);

        let interruptible = Interruptible {
            interrupt_rx,
            interrupt_strategy,
        };

        assert_eq!(
            "NonInterruptible",
            format!("{:?}", Interruptibility::NonInterruptible)
        );
        assert_eq!("NonInterruptible", format!("{:?}", NonInterruptible));
        assert!(format!("{interruptible:?}").starts_with("Interruptible"));
    }

    #[test]
    fn clone() {
        assert_eq!(NonInterruptible, Clone::clone(&NonInterruptible));
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
