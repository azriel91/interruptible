/// [`InterruptibleStream`] outcome that indicates whether an interruption
/// happened.
#[derive(Debug, PartialEq, Eq)]
pub enum PollOutcome<T> {
    /// An interrupt signal was received.
    ///
    /// The item is `None` in the following cases:
    ///
    /// * The interruption happened before the stream was polled.
    /// * The interruption happened after a `Poll::Ready`, but before a
    ///   subsequent poll.
    Interrupted(Option<T>),
    /// No interrupt signal was received.
    NoInterrupt(T),
}

#[cfg(test)]
mod tests {
    use super::PollOutcome;

    #[test]
    fn debug() {
        assert_eq!(
            "Interrupted(Some(1))",
            format!("{:?}", PollOutcome::Interrupted(Some(1)))
        );
    }
}
