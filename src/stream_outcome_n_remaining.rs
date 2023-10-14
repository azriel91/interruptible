/// [`InterruptibleStream`] outcome that indicates whether an interruption
/// happened.
#[derive(Debug, PartialEq, Eq)]
pub enum StreamOutcomeNRemaining<T> {
    /// An interrupt signal was received before the stream was polled.
    InterruptBeforePoll,
    /// An interrupt signal was received after the stream was polled at least
    /// once.
    InterruptDuringPoll {
        /// The value returned by the underlying stream.
        value: T,
        /// The maximum number of remaining items that may be returned by the
        /// underlying stream.
        n_remaining: u32,
    },
    /// No interrupt signal was received.
    NoInterrupt(T),
}

#[cfg(test)]
mod tests {
    use super::StreamOutcomeNRemaining;

    #[test]
    fn debug() {
        assert_eq!(
            "InterruptDuringPoll { value: 1, n_remaining: 2 }",
            format!(
                "{:?}",
                StreamOutcomeNRemaining::InterruptDuringPoll {
                    value: 1,
                    n_remaining: 2,
                }
            )
        );
    }
}
