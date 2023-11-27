# Changelog

## 0.0.3 (2023-11-28)

* Improve crate quality to be candidate for production use.
* Rewrite`InterruptibleStreamExt` and `InterruptibleStream` to support interrupt strategies.
* Add `InterruptibilityState` to maintain state across different streams.


## 0.0.2 (2023-10-07)

* Update `InterruptibleFutureExt` types to return last value alongside `InterruptSignal`.


## 0.0.1 (2023-08-02)

* Add `InterruptibleFutureExt` that intercepts interrupt signals, and returns `Break` or `Err`.
* Add `InterruptibleStreamExt` that stops a `Stream` from producing values when an interrupt signal is received.
