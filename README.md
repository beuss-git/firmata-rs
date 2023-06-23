# Firmata client library in Rust

Control your [Firmata](https://github.com/firmata/protocol) devices from Rust!

The library comes with a Board struct, which you can initialize with any object that implements
`std:io::{Read, Write}`. This avoids being locked in to a specific interface library. I highly
recommend `serialport` for your USB connections (used in examples), but feel free to use `serial` or
any other.

The different methods of the `Firmata` trait that return results also have _backoff-able_
counterparts in the `RetryFirmata` trait that utilizes a `ExponentialBackoff` strategy powered by
the `backoff` crate. This may be useful as your Rust program may run "too fast" for your Arduino
device to keep up.

## Acknowledgements

Original code for this library was written by Adrian Zankich, but most methods have been re-written
into infallible (no panic) code.
