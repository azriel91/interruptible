use std::fmt::{self, Debug};

use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{InterruptSignal, InterruptStrategy, Interruptibility, OwnedOrMutRef, OwnedOrRef};

type FnInterrupt<'intx> = Box<dyn Fn() + 'intx>;

/// Whether interruptibility is supported, and number of times interrupt signals
/// have been received.
pub struct InterruptibilityState<'rx, 'intx> {
    /// Specifies interruptibility support of the application.
    pub(crate) interruptibility: Interruptibility<'rx>,
    /// Number of times an interrupt signal has been received.
    pub(crate) poll_since_interrupt_count: OwnedOrMutRef<'intx, u64>,
    /// Whether previously an interrupt signal has been received.
    pub(crate) interrupt_signal_received: OwnedOrMutRef<'intx, Option<InterruptSignal>>,
    /// Function to run when an interruption is activated.
    ///
    /// For `PollNextN`, this will run on the `n`th poll, rather than when the
    /// `InterruptSignal` is received.
    ///
    /// The function will only run once; subsequent polls will not run the
    /// function again.
    fn_interrupt_activate: Option<OwnedOrRef<'intx, FnInterrupt<'intx>>>,
    /// Function to run the first time the underlying stream is polled after an
    /// interruption is activated.
    ///
    /// For `PollNextN`, this will run on the `n`th poll, rather than when the
    /// `InterruptSignal` is received.
    ///
    /// The function will only run once; subsequent polls will not run the
    /// function again.
    ///
    /// # Notes
    ///
    /// This differs from `fn_interrupt_activate` in that
    /// `fn_interrupt_activate` is run when the interruption is activated, and
    /// any poll occurs that may be advancing a previously polled item. i.e.
    /// `self.stream.poll()` may still return `Poll::Pending`, so
    /// `PollOutcome::Interrupted` is not yet surfaced to the consumer of the
    /// `InterruptibleStream`.
    ///
    /// Whereas `fn_interrupt_poll_item` is run when the poll actually polls a
    /// new item from the underlying stream.
    fn_interrupt_poll_item: Option<OwnedOrRef<'intx, FnInterrupt<'intx>>>,
}

impl<'rx, 'intx> Debug for InterruptibilityState<'rx, 'intx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InterruptibilityState")
            .field("interruptibility", &self.interruptibility)
            .field(
                "poll_since_interrupt_count",
                &self.poll_since_interrupt_count,
            )
            .field("interrupt_signal_received", &self.interrupt_signal_received)
            .field(
                "fn_interrupt_activate",
                if self.fn_interrupt_activate.is_some() {
                    &Some("Box<dyn Fn() + 'intx>")
                } else {
                    &None::<()>
                },
            )
            .field(
                "fn_interrupt_poll_item",
                if self.fn_interrupt_poll_item.is_some() {
                    &Some("Box<dyn Fn() + 'intx>")
                } else {
                    &None::<()>
                },
            )
            .finish()
    }
}

impl<'rx> InterruptibilityState<'rx, 'static> {
    /// Returns a new [`InterruptibilityState`].
    pub fn new(interruptibility: Interruptibility<'rx>) -> Self {
        Self {
            interruptibility,
            poll_since_interrupt_count: OwnedOrMutRef::Owned(0),
            interrupt_signal_received: OwnedOrMutRef::Owned(None),
            fn_interrupt_activate: None,
            fn_interrupt_poll_item: None,
        }
    }

    /// Returns a new `InterruptibilityState` with
    /// [`Interruptibility::NonInterruptible`] support.
    pub fn new_non_interruptible() -> Self {
        Self::new(Interruptibility::NonInterruptible)
    }

    /// Returns a new `InterruptibilityState` with
    /// [`InterruptStrategy::IgnoreInterruptions`].
    pub fn new_ignore_interruptions(
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> Self {
        Self::new(Interruptibility::Interruptible {
            interrupt_rx,
            interrupt_strategy: InterruptStrategy::IgnoreInterruptions,
        })
    }

    /// Returns a new `InterruptibilityState` with
    /// [`InterruptStrategy::FinishCurrent`].
    pub fn new_finish_current(
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
    ) -> Self {
        Self::new(Interruptibility::Interruptible {
            interrupt_rx,
            interrupt_strategy: InterruptStrategy::FinishCurrent,
        })
    }

    /// Returns a new `InterruptibilityState` with
    /// [`InterruptStrategy::PollNextN`].
    pub fn new_poll_next_n(
        interrupt_rx: OwnedOrMutRef<'rx, mpsc::Receiver<InterruptSignal>>,
        n: u64,
    ) -> Self {
        Self::new(Interruptibility::Interruptible {
            interrupt_rx,
            interrupt_strategy: InterruptStrategy::PollNextN(n),
        })
    }
}

impl<'rx, 'intx> InterruptibilityState<'rx, 'intx> {
    /// Sets the function to run when an interruption is activated.
    ///
    /// For `PollNextN`, this will run on the `n`th poll, rather than when the
    /// `InterruptSignal` is received.
    ///
    /// The function will only run once; subsequent polls will not run the
    /// function again.
    pub fn set_fn_interrupt_activate<F>(&mut self, fn_interrupt_activate: Option<F>)
    where
        F: Fn() + 'intx,
    {
        self.fn_interrupt_activate = fn_interrupt_activate
            .map(|f| Box::new(f) as FnInterrupt<'intx>)
            .map(OwnedOrRef::from);
    }

    /// Sets the function to run the first time the underlying stream is polled
    /// after an interruption is activated.
    ///
    /// For `PollNextN`, this will run on the `n`th poll, rather than when the
    /// `InterruptSignal` is received.
    ///
    /// The function will only run once; subsequent polls will not run the
    /// function again.
    ///
    /// # Notes
    ///
    /// This differs from `fn_interrupt_activate` in that
    /// `fn_interrupt_activate` is run when the interruption is activated, and
    /// any poll occurs that may be advancing a previously polled item. i.e.
    /// `self.stream.poll()` may still return `Poll::Pending`, so
    /// `PollOutcome::Interrupted` is not yet surfaced to the consumer of the
    /// `InterruptibleStream`.
    ///
    /// Whereas `fn_interrupt_poll_item` is run when the poll actually polls a
    /// new item from the underlying stream.
    pub fn set_fn_interrupt_poll_item<F>(&mut self, fn_interrupt_poll_item: Option<F>)
    where
        F: Fn() + 'intx,
    {
        self.fn_interrupt_poll_item = fn_interrupt_poll_item
            .map(|f| Box::new(f) as FnInterrupt<'intx>)
            .map(OwnedOrRef::from);
    }

    /// Reborrows this `InterruptibilityState` with a shorter lifetime.
    pub fn reborrow<'rx_local, 'intx_local>(
        &'rx_local mut self,
    ) -> InterruptibilityState<'rx_local, 'intx_local>
    where
        'rx: 'rx_local + 'intx_local,
        'intx: 'rx_local + 'intx_local,
        'rx_local: 'intx_local,
    {
        let interruptibility = self.interruptibility.reborrow();
        let poll_since_interrupt_count = self.poll_since_interrupt_count.reborrow();
        let interrupt_signal_received = self.interrupt_signal_received.reborrow();
        let fn_interrupt_activate = self
            .fn_interrupt_activate
            .as_ref()
            .map(OwnedOrRef::reborrow);
        let fn_interrupt_poll_item = self
            .fn_interrupt_poll_item
            .as_ref()
            .map(OwnedOrRef::reborrow);

        InterruptibilityState {
            interruptibility,
            poll_since_interrupt_count,
            interrupt_signal_received,
            fn_interrupt_activate,
            fn_interrupt_poll_item,
        }
    }

    /// Returns if this `InterruptibilityState` is considered interrupted based
    /// on the chosen strategy.
    ///
    /// * For `IgnoreInterruptions`, this always returns `false`.
    /// * For `FinishCurrent`, this returns `true` after receiving at least one
    ///   `InterruptSignal`.
    /// * For `PollNextN`, this returns `true` if the stream has been polled at
    ///   least `n` times since receiving the `InterruptSignal`.
    pub fn is_interrupted(&self) -> bool {
        match self.interruptibility {
            Interruptibility::NonInterruptible => false,
            Interruptibility::Interruptible {
                interrupt_rx: _,
                interrupt_strategy,
            } => match interrupt_strategy {
                InterruptStrategy::IgnoreInterruptions => false,
                InterruptStrategy::FinishCurrent => self.interrupt_signal_received.is_some(),
                InterruptStrategy::PollNextN(n) => {
                    self.interrupt_signal_received.is_some()
                        && *self.poll_since_interrupt_count >= n
                }
            },
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
                let mut interrupt_signal_first_received = false;
                if self.interrupt_signal_received.is_none() {
                    match interrupt_rx.try_recv() {
                        Ok(interrupt_signal) => {
                            interrupt_signal_first_received = true;
                            *self.interrupt_signal_received = Some(interrupt_signal);
                        }
                        Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
                    }
                }

                match *self.interrupt_signal_received {
                    Some(interrupt_signal) => {
                        match (
                            matches!(interrupt_strategy, InterruptStrategy::PollNextN(_)),
                            increment_item_count,
                            interrupt_signal_first_received,
                        ) {
                            (false, true, _) | (true, _, false) => {
                                *self.poll_since_interrupt_count += 1
                            }
                            _ => {}
                        }
                        let poll_since_interrupt_count = *self.poll_since_interrupt_count;

                        let interrupt_signal = Self::interrupt_signal_based_on_strategy(
                            interrupt_strategy,
                            interrupt_signal,
                            poll_since_interrupt_count,
                        );

                        let fn_interrupt_activate = self
                            .fn_interrupt_activate
                            .as_ref()
                            .map(OwnedOrRef::reborrow);
                        if let Some((_interrupt_signal, fn_interrupt_activate)) =
                            interrupt_signal.zip(fn_interrupt_activate)
                        {
                            Self::fn_interrupt_based_on_strategy(
                                interrupt_strategy,
                                &fn_interrupt_activate,
                            );
                        }

                        interrupt_signal
                    }
                    None => None,
                }
            }
        }
    }

    /// Runs `fn_interrupt_activate` if the strategy's threshold is reached.
    ///
    /// * For `IgnoreInterruptions`, the function is never run.
    /// * For `FinishCurrent`, this runs if `interrupt_signal_first_received` is
    ///   `true`.
    /// * For `PollNextN`, this runs if `poll_since_interrupt_count` equals `n`.
    fn fn_interrupt_based_on_strategy(
        interrupt_strategy: &InterruptStrategy,
        fn_interrupt_activate: &FnInterrupt,
    ) {
        match interrupt_strategy {
            InterruptStrategy::IgnoreInterruptions => {}
            InterruptStrategy::FinishCurrent | InterruptStrategy::PollNextN(_) => {
                (*fn_interrupt_activate)();
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
        interrupt_strategy: &InterruptStrategy,
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
                if poll_since_interrupt_count >= *n {
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

    /// Returns the function to run the first time the underlying stream is
    /// polled after an interruption is activated.
    pub fn fn_interrupt_poll_item(&self) -> Option<&FnInterrupt<'intx>> {
        self.fn_interrupt_poll_item.as_deref()
    }

    /// Returns the `InterruptStrategy` if present.
    pub fn interrupt_strategy(&self) -> Option<InterruptStrategy> {
        self.interruptibility.strategy()
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
    use tokio::sync::mpsc::{self, error::TryRecvError};

    use crate::{InterruptSignal, Interruptibility};

    use super::InterruptibilityState;

    #[test]
    fn reborrow() {
        let mut interruptibility_state = InterruptibilityState::new_non_interruptible();
        let mut reborrow_owned = interruptibility_state.reborrow();
        let _reborrow_reborrow = reborrow_owned.reborrow();

        let (_interrupt_tx, interrupt_rx) = mpsc::channel(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        let mut reborrow_owned = interruptibility_state.reborrow();
        let _reborrow_reborrow = reborrow_owned.reborrow();

        let (_interrupt_tx, interrupt_rx) = mpsc::channel(16);
        let mut interruptibility_state =
            InterruptibilityState::new_finish_current(interrupt_rx.into());
        let mut reborrow_owned = interruptibility_state.reborrow();
        let _reborrow_reborrow = reborrow_owned.reborrow();

        let (_interrupt_tx, interrupt_rx) = mpsc::channel(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        let mut reborrow_owned = interruptibility_state.reborrow();
        let _reborrow_reborrow = reborrow_owned.reborrow();
    }

    #[test]
    fn is_interrupted_returns_false_when_not_interrupted() {
        let interruptibility_state = InterruptibilityState::new_non_interruptible();
        assert!(!interruptibility_state.is_interrupted());

        let (_interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interrupt_rx = &mut interrupt_rx;

        let interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        assert!(!interruptibility_state.is_interrupted());

        let interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        assert!(!interruptibility_state.is_interrupted());

        let interruptibility_state = InterruptibilityState::new_finish_current(interrupt_rx.into());
        assert!(!interruptibility_state.is_interrupted());

        let interruptibility_state = InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        assert!(!interruptibility_state.is_interrupted());
    }

    #[tokio::test]
    async fn is_interrupted_returns_true_when_interrupt_activated()
    -> Result<(), Box<dyn std::error::Error>> {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interrupt_rx = &mut interrupt_rx;

        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());

        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());

        let mut interruptibility_state =
            InterruptibilityState::new_finish_current(interrupt_rx.into());
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());

        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interrupt_tx.send(InterruptSignal).await?;
        // The first item is not counted, since `poll_next_n` wants `n` more items.
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());

        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        interruptibility_state.item_interrupt_poll(false);
        assert!(!interruptibility_state.is_interrupted());
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());

        Ok(())
    }

    #[tokio::test]
    async fn with_fn_interrupt_activate_runs_when_interrupt_activated()
    -> Result<(), Box<dyn std::error::Error>> {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interrupt_rx = &mut interrupt_rx;

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(100)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(101)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_finish_current(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(102)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());
        assert_eq!(Ok(102), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(103)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());
        assert_eq!(Ok(103), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(104)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        interruptibility_state.item_interrupt_poll(false);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());
        assert_eq!(Ok(104), interrupt_activate_rx.try_recv());

        Ok(())
    }

    #[tokio::test]
    async fn set_fn_interrupt_activate_runs_when_interrupt_activated()
    -> Result<(), Box<dyn std::error::Error>> {
        let (interrupt_tx, mut interrupt_rx) = mpsc::channel::<InterruptSignal>(16);
        let interrupt_rx = &mut interrupt_rx;

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(100)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_ignore_interruptions(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(101)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_finish_current(interrupt_rx.into());
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(102)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());
        assert_eq!(Ok(102), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(103)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());
        assert_eq!(Ok(103), interrupt_activate_rx.try_recv());

        let (interrupt_activate_tx, mut interrupt_activate_rx) = mpsc::channel::<u16>(16);
        let mut interruptibility_state =
            InterruptibilityState::new_poll_next_n(interrupt_rx.into(), 2);
        interruptibility_state.set_fn_interrupt_activate(Some(|| {
            interrupt_activate_tx
                .try_send(104)
                .expect("Expected to send value.");
        }));
        interrupt_tx.send(InterruptSignal).await?;
        interruptibility_state.item_interrupt_poll(true);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        interruptibility_state.item_interrupt_poll(false);
        assert!(!interruptibility_state.is_interrupted());
        assert_eq!(Err(TryRecvError::Empty), interrupt_activate_rx.try_recv());
        interruptibility_state.item_interrupt_poll(true);
        assert!(interruptibility_state.is_interrupted());
        assert_eq!(Ok(104), interrupt_activate_rx.try_recv());

        Ok(())
    }

    #[test]
    fn debug() {
        let interruptibility_state = InterruptibilityState::new(Interruptibility::NonInterruptible);

        assert_eq!(
            "InterruptibilityState { \
                interruptibility: NonInterruptible, \
                poll_since_interrupt_count: Owned(0), \
                interrupt_signal_received: Owned(None), \
                fn_interrupt_activate: None, \
                fn_interrupt_poll_item: None \
            }",
            format!("{interruptibility_state:?}")
        );

        let mut interruptibility_state = InterruptibilityState::new_non_interruptible();
        interruptibility_state.set_fn_interrupt_activate(Some(|| {}));
        interruptibility_state.set_fn_interrupt_poll_item(Some(|| {}));

        assert_eq!(
            "InterruptibilityState { \
                interruptibility: NonInterruptible, \
                poll_since_interrupt_count: Owned(0), \
                interrupt_signal_received: Owned(None), \
                fn_interrupt_activate: Some(\"Box<dyn Fn() + 'intx>\"), \
                fn_interrupt_poll_item: Some(\"Box<dyn Fn() + 'intx>\") \
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
                interrupt_signal_received: Owned(None), \
                fn_interrupt_activate: None, \
                fn_interrupt_poll_item: None \
            }",
            format!("{interruptibility_state:?}")
        );
    }
}
