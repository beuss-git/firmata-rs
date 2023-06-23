# Firmata client library in Rust

Control your [Firmata](https://github.com/firmata/protocol) devices from Rust!

The library comes with a Board struct, which you can initialize with any object that implements
`std:io::{Read, Write}` and `Debug` for formatting purposes. This avoids being locked in to a
specific interface library. I highly recommend `serialport` for your USB connections (used in
examples), but feel free to use `serial` or any other.

The different methods of the `Firmata` trait that return results also have _backoff-able_
counterparts in the `RetryFirmata` trait that utilizes a `ExponentialBackoff` strategy powered by
the `backoff` crate. This may be useful as your Rust program may run "too fast" for your Firmata
device to keep up.

The crate has been set up to utilize `tracing`, which helps in finding where your messages went!
If you set the environment variable `CARGO_LOG=DEBUG` you can capture the most noise.

## Examples

There are quite a couple of examples to try with your Firmata device! You can run each of them like
this:

```bash
cargo run --example blink
```

Where `blink` is the example's filename.

If you want the "full" output you can use:

```bash
RUST_LOG=DEBUG cargo run --example blink
```

## Installing Firmata on a device

Chances are you have an Arduino or other Firmata device lying around since you're here :). You can
go to your favorite Arduino IDE of choice and load the regular "StandardFirmata" onto your device
and start tinkering!

## Finding the right port

You might need to set the USB port to the one that is in use on your machine. Find the right
`port_name` in the list after running:

```bash
cargo run --example available
```

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

## Acknowledgements

This library is largely based on the earlier work by Adrian Zankich over at
https://github.com/zankich/rust-firmata to whom should go out many thanks!
