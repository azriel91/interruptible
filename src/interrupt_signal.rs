/// Signal signifying an interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptSignal;

impl From<((), InterruptSignal)> for InterruptSignal {
    fn from(_: ((), InterruptSignal)) -> Self {
        InterruptSignal
    }
}

#[cfg(test)]
mod tests {
    use super::InterruptSignal;

    #[test]
    fn debug() {
        assert_eq!("InterruptSignal", format!("{:?}", InterruptSignal))
    }

    #[test]
    fn clone() {
        assert_eq!(InterruptSignal, Clone::clone(&InterruptSignal))
    }
}
