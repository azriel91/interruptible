# Changelog

## unreleased

* Update `InterruptibleFutureExt` types to return last value alongside `InterruptSignal`.


## 0.0.1 (2023-08-02)

* Add `InterruptibleFutureExt` that intercepts interrupt signals, and returns `Break` or `Err`.
* Add `InterruptibleStreamExt` that stops a `Stream` from producing values when an interrupt signal is received.
