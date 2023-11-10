use crate::{Interruptibility, OwnedOrMutRef};

/// Whether interruptibility is supported, and number of times interrupt signals
/// have been received.
#[derive(Debug)]
pub struct InterruptibilityState<'rx: 'intx, 'intx> {
    /// Specifies interruptibility support of the application.
    pub(crate) interruptibility: Interruptibility<'rx>,
    /// Number of times an interrupt signal has been received.
    pub(crate) poll_count: OwnedOrMutRef<'intx, u32>,
}

impl<'rx> InterruptibilityState<'rx, 'static> {
    /// Returns a new `InterruptibilityState`.
    pub fn new(interruptibility: Interruptibility<'rx>) -> Self {
        Self {
            interruptibility,
            poll_count: OwnedOrMutRef::Owned(0),
        }
    }
}

impl<'rx> From<Interruptibility<'rx>> for InterruptibilityState<'rx, 'static> {
    /// Returns a new `InterruptibilityState`.
    fn from(interruptibility: Interruptibility<'rx>) -> Self {
        Self::new(interruptibility)
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
                interrupt_rx,
                interrupt_strategy: *interrupt_strategy,
            },
        };

        let poll_count = OwnedOrMutRef::MutRef(&mut *self.poll_count);

        InterruptibilityState {
            interruptibility,
            poll_count,
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

    /// Returns the number of times an interrupt signal has been received.
    pub fn poll_count(&self) -> u32 {
        *self.poll_count
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
            "InterruptibilityState { interruptibility: NonInterruptible, poll_count: Owned(0) }",
            format!("{interruptibility_state:?}")
        );
    }

    #[test]
    fn from() {
        let interruptibility_state =
            InterruptibilityState::from(Interruptibility::NonInterruptible);

        assert_eq!(
            "InterruptibilityState { interruptibility: NonInterruptible, poll_count: Owned(0) }",
            format!("{interruptibility_state:?}")
        );
    }
}
