# Changelog

## unreleased

* Add `InterruptibilityState::set_fn_interrupt_poll_item` to run arbitrary function when interruption is activated and the underlying `Stream` returns `Poll::Ready(..)`.
* Add tests to prove correctness of `PollNextN` `InterruptStrategy`.


## 0.1.0 (2024-01-02)

* Add `InterruptibilityState::set_fn_interrupt_activate` to run arbitrary function when interruption is activated.


## 0.0.4 (2023-12-30)

* Add `InterruptibilityState::is_interrupted`.
* Add `InterruptibilityState::new_non_interruptible`.
* Add `InterruptibilityState::new_ignore_interruptions`.
* Add `InterruptibilityState::new_finish_current`.
* Add `InterruptibilityState::new_poll_next_n`.


## 0.0.3 (2023-11-28)

* Improve crate quality to be candidate for production use.
* Rewrite`InterruptibleStreamExt` and `InterruptibleStream` to support interrupt strategies.
* Add `InterruptibilityState` to maintain state across different streams.


## 0.0.2 (2023-10-07)

* Update `InterruptibleFutureExt` types to return last value alongside `InterruptSignal`.


## 0.0.1 (2023-08-02)

* Add `InterruptibleFutureExt` that intercepts interrupt signals, and returns `Break` or `Err`.
* Add `InterruptibleStreamExt` that stops a `Stream` from producing values when an interrupt signal is received.
