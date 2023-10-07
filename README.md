# 🗂️ interruptible

[![Crates.io](https://img.shields.io/crates/v/interruptible.svg)](https://crates.io/crates/interruptible)
[![docs.rs](https://img.shields.io/docsrs/interruptible)](https://docs.rs/interruptible)
[![CI](https://github.com/azriel91/interruptible/workflows/CI/badge.svg)](https://github.com/azriel91/interruptible/actions/workflows/ci.yml)
[![Coverage Status](https://codecov.io/gh/azriel91/interruptible/branch/main/graph/badge.svg)](https://codecov.io/gh/azriel91/interruptible)

Stops a future producer or stream from producing values when an interrupt signal is received.

For a future that returns either `Result<T, ()>` or `ControlFlow<T, ()>`, calling `fut.interruptible_*(tx)` causes the returned value to be `Err(())` or `Break(T)` if an interruption signal is received *while* that future is executing.

This means the future is progressed to completion, but the return value signals the producer to stop yielding futures.

For a stream, when the interrupt signal is received, the current future is run to completion, but the stream is not polled for the next item.


## Usage

Add the following to `Cargo.toml`

```toml
interruptible = "0.0.1"

# Enables:
#
# * `InterruptibleFutureExt::{interruptible_control_ctrl_c, interruptible_result_ctrl_c}`
# * `InterruptibleStreamExt::interruptible_ctrl_c` if the `"stream"` feature is also enabled.
interruptible = { version = "0.0.1", features = ["ctrl_c"] }

# Enables `InterruptibleStreamExt`
interruptible = { version = "0.0.1", features = ["stream"] }
```

### Code

#### `Future<Output = ControlFlow<B, C>>`

```rust
use std::ops::ControlFlow;

use futures::FutureExt;
use tokio::{join, sync::oneshot};

use interruptible::{InterruptSignal, InterruptibleFutureExt};

#[tokio::main]
async fn main() {
    let (interrupt_tx, mut interrupt_rx) = oneshot::channel::<InterruptSignal>();
    let (ready_tx, ready_rx) = oneshot::channel::<()>();

    let interruptible_control = async {
        let () = ready_rx.await.expect("Expected to be notified to start.");
        ControlFlow::Continue(())
    }
    .boxed()
    .interruptible_control(&mut interrupt_rx);

    let interrupter = async move {
        interrupt_tx
            .send(InterruptSignal)
            .expect("Expected to send `InterruptSignal`.");
        ready_tx
            .send(())
            .expect("Expected to notify sleep to start.");
    };

    let (control_flow, ()) = join!(interruptible_control, interrupter);

    assert_eq!(ControlFlow::Break(InterruptSignal), control_flow);
}
```


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE] or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT] or <https://opensource.org/licenses/MIT>)

at your option.


### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

[LICENSE-APACHE]: LICENSE-APACHE
[LICENSE-MIT]: LICENSE-MIT
