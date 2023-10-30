/// [`InterruptibleStream`] outcome that indicates whether an interruption
/// happened.
#[derive(Debug, PartialEq, Eq)]
pub enum PollOutcome<T> {
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
    use super::PollOutcome;

    #[test]
    fn debug() {
        assert_eq!(
            "InterruptDuringPoll(1)",
            format!("{:?}", PollOutcome::InterruptDuringPoll(1))
        );
    }
}
