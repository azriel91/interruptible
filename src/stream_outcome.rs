/// [`InterruptibleStream`] outcome that indicates whether an interruption
/// happened.
#[derive(Debug, PartialEq, Eq)]
pub enum StreamOutcome<T> {
    /// An interrupt signal was received before the stream was polled.
    InterruptBeforePoll,
    /// An interrupt signal was received after the stream was polled at least
    /// once.
    InterruptDuringPoll(T),
    /// No interrupt signal was received.
    NoInterrupt(T),
}

#[cfg(test)]
mod tests {
    use super::StreamOutcome;

    #[test]
    fn debug() {
        assert_eq!(
            "InterruptDuringPoll(1)",
            format!("{:?}", StreamOutcome::InterruptDuringPoll(1))
        );
    }
}
