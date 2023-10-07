/// Signal signifying an interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptSignal;

impl From<((), InterruptSignal)> for InterruptSignal {
    fn from(_: ((), InterruptSignal)) -> Self {
        InterruptSignal
    }
}
