//! # Firmata client library in Rust
//!
//! Control your [Firmata](https://github.com/firmata/protocol) devices from Rust!
//!
//! The library comes with a Board struct, which you can initialize with any object that implements
//! `std:io::{Read, Write}` and `Debug` for formatting purposes. This avoids being locked in to a
//! specific interface library. I highly recommend [`serialport`] for your USB connections (used in
//! examples), but feel free to use [`serial`] or any other.
//!
//! The different methods of the [`Firmata`] trait that return results also have _backoff-able_
//! counterparts in the [`RetryFirmata`] trait that utilizes a [`backoff::ExponentialBackoff`]
//! strategy powered by the [`backoff`] crate. This may be useful as your Rust program may run "too
//! fast" for your Firmata device to keep up.
//!
//! The crate has been set up to utilize [`tracing`], which helps in finding where your messages
//! went! If you set the environment variable `CARGO_LOG=DEBUG` you can capture the most noise.
//!
//! ## Examples
//!
//! There are quite a couple of examples to try with your Firmata device! You can run each of them
//! like this:
//!
//! ```bash
//! cargo run --example blink
//! ```
//!
//! Where `blink` is the example's filename.
//!
//! If you want the "full" output you can use:
//!
//! ```bash
//! RUST_LOG=DEBUG cargo run --example blink
//! ```
//!
//! ## Installing Firmata on a device
//!
//! Chances are you have an Arduino or other Firmata device lying around since you're here :). You
//! can go to your favorite Arduino IDE of choice and load the regular "StandardFirmata" onto your
//! device and start tinkering!
//!
//! ## Finding the right port
//!
//! You might need to set the USB port to the one that is in use on your machine. Find the right
//! `port_name` in the list after running:
//!
//! ```bash
//! cargo run --example available
//! ```
//!
//! ## Acknowledgements
//!
//! This library is largely based on the earlier work by Adrian Zankich over at
//! https://github.com/zankich/rust-firmata to whom should go out many thanks!

use snafu::prelude::*;
use std::io::{Read, Write};
use std::time::Duration;
mod constants;
pub use constants::*;

/// Firmata error type.
#[derive(Debug, Snafu)]
pub enum Error {
    /// Unknown SysEx code: {code}
    UnknownSysEx { code: u8 },
    /// Received a bad byte: {byte}
    BadByte { byte: u8 },
    /// I/O error: {source}
    StdIoError { source: std::io::Error },
    /// UTF8 error: {source}
    Utf8Error { source: std::str::Utf8Error },
    /// Message was too short.
    MessageTooShort,
    /// Pin out of bounds: {pin} ({len}).
    PinOutOfBounds { pin: u8, len: usize },
}
impl From<backoff::Error<Error>> for Error {
    fn from(value: backoff::Error<Error>) -> Self {
        match value {
            backoff::Error::Permanent(err) => err,
            backoff::Error::Transient { err, .. } => err,
        }
    }
}
/// Result type with Firmata Error.
pub type Result<T> = std::result::Result<T, Error>;

/// Received Firmata message
#[derive(Clone, Debug)]
pub enum Message {
    ProtocolVersion,
    Analog,
    Digital,
    EmptyResponse,
    AnalogMappingResponse,
    CapabilityResponse,
    PinStateResponse,
    ReportFirmware,
    I2CReply,
}

/// An I2C reply.
#[derive(Debug, Default)]
pub struct I2CReply {
    pub address: i32,
    pub register: i32,
    pub data: Vec<u8>,
}

/// The current state and configuration of a pin.
#[derive(Debug)]
pub struct Pin {
    /// Currently configured mode.
    pub mode: u8,
    /// Current resolution.
    pub resolution: u8,
    /// All pin modes.
    pub modes: Vec<u8>,
    /// Pin value.
    pub value: i32,
}
impl Default for Pin {
    fn default() -> Self {
        Self {
            mode: PIN_MODE_ANALOG,
            modes: vec![PIN_MODE_ANALOG],
            resolution: DEFAULT_ANALOG_RESOLUTION,
            value: 0,
        }
    }
}

/// Firmata board functionality.
pub trait Firmata: std::fmt::Debug {
    /// Write `level` to the analog `pin`.
    fn analog_write(&mut self, pin: i32, level: i32) -> Result<()>;
    /// Write `level` to the digital `pin`.
    fn digital_write(&mut self, pin: i32, level: i32) -> Result<()>;
    /// Get the firmware name.
    fn firmware_name(&mut self) -> &String;
    /// Get the firmware version.
    fn firmware_version(&mut self) -> &String;
    /// Configure the `delay` in microseconds for I2C devices that require a delay between when the
    /// register is written to and the data in that register can be read.
    fn i2c_config(&mut self, delay: i32) -> Result<()>;
    /// Get the raw I2C replies that have been read from the board.
    fn i2c_data(&mut self) -> &mut Vec<I2CReply>;
    /// Read `size` bytes from I2C device at the specified `address`.
    fn i2c_read(&mut self, address: i32, size: i32) -> Result<()>;
    /// Write `data` to the I2C device at the specified `address`.
    fn i2c_write(&mut self, address: i32, data: &[u8]) -> Result<()>;
    /// Get pins that the board has access to.
    fn pins(&mut self) -> &Vec<Pin>;
    /// Get the current Firmata protocol version.
    fn protocol_version(&mut self) -> &String;
    /// Query the board for available analog pins.
    fn query_analog_mapping(&mut self) -> Result<()>;
    /// Query the board for all available capabilities.
    fn query_capabilities(&mut self) -> Result<()>;
    /// Query the board for current firmware and protocol information.
    fn query_firmware(&mut self) -> Result<()>;
    /// Read from the Firmata device, parse one Firmata message and return its type.
    fn read_and_decode(&mut self) -> Result<Message>;
    /// Set the analog reporting `state` of the specified `pin`.
    fn report_analog(&mut self, pin: i32, state: i32) -> Result<()>;
    /// Set the digital reporting `state` of the specified `pin`.
    fn report_digital(&mut self, pin: i32, state: i32) -> Result<()>;
    /// Set the `mode` of the specified `pin`.
    fn set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<()>;
}

/// Firmata board functionality that retries and fallible methods.
pub trait RetryFirmata: Firmata {
    /// Backoff strategy.
    fn backoff(&self) -> backoff::ExponentialBackoff {
        backoff::ExponentialBackoff {
            max_interval: Duration::from_millis(5_000),
            ..Default::default()
        }
    }
    /// Write `level` to the analog `pin`.
    fn retry_analog_write(&mut self, pin: i32, level: i32) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.analog_write(pin, level)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Write `level` to the digital `pin`.
    fn retry_digital_write(&mut self, pin: i32, level: i32) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.digital_write(pin, level)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Configure the `delay` in microseconds for I2C devices that require a delay between when the
    /// register is written to and the data in that register can be read.
    fn retry_i2c_config(&mut self, delay: i32) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.i2c_config(delay).map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Read `size` bytes from I2C device at the specified `address`.
    fn retry_i2c_read(&mut self, address: i32, size: i32) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.i2c_read(address, size)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Write `data` to the I2C device at the specified `address`.
    fn retry_i2c_write(&mut self, address: i32, data: &[u8]) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.i2c_write(address, data)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Query the board for available analog pins.
    fn retry_query_analog_mapping(&mut self) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.query_analog_mapping()
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Query the board for all available capabilities.
    fn retry_query_capabilities(&mut self) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.query_capabilities().map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Query the board for current firmware and protocol information.
    fn retry_query_firmware(&mut self) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.query_firmware().map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Read from the Firmata device, parse one Firmata message and return its type.
    fn retry_read_and_decode(&mut self) -> Result<Message> {
        backoff::retry(self.backoff(), || {
            self.read_and_decode().map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Set the analog reporting `state` of the specified `pin`.
    fn retry_report_analog(&mut self, pin: i32, state: i32) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.report_analog(pin, state)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Set the digital reporting `state` of the specified `pin`.
    fn retry_report_digital(&mut self, pin: i32, state: i32) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.report_digital(pin, state)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
    /// Set the `mode` of the specified `pin`.
    fn retry_set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<()> {
        backoff::retry(self.backoff(), || {
            self.set_pin_mode(pin, mode)
                .map_err(backoff::Error::transient)
        })
        .map_err(|e| e.into())
    }
}

impl<T> RetryFirmata for T where T: Firmata {}

/// A Firmata board representation.
#[derive(Debug)]
pub struct Board<T: Read + Write + std::fmt::Debug> {
    pub connection: Box<T>,
    pub pins: Vec<Pin>,
    pub i2c_data: Vec<I2CReply>,
    pub protocol_version: String,
    pub firmware_name: String,
    pub firmware_version: String,
}
impl<T: Read + Write + std::fmt::Debug> std::fmt::Display for Board<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Board {{ firmware={}, version={}, protocol={}, connection={:?} }}",
            self.firmware_name, self.firmware_version, self.protocol_version, self.connection
        )
    }
}
impl<T: Read + Write + std::fmt::Debug> Board<T> {
    /// Write on the internal connection.
    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn write(&mut self, buf: &[u8]) -> Result<()> {
        self.connection
            .write(buf)
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }
}

impl<T: Read + Write + std::fmt::Debug> Board<T> {
    fn initialize_board(&mut self) -> Result<()> {
        self.query_firmware()?;
        self.query_capabilities()?;
        self.query_analog_mapping()?;

        // Wait a little for the messages to queue up
        std::thread::sleep(std::time::Duration::from_millis(1000));

        let mut received_firmware = false;
        let mut received_capabilities = false;
        let mut received_analog_mapping = false;

        while !received_firmware || !received_capabilities || !received_analog_mapping {
            match self.read_and_decode() {
                Ok(Message::ReportFirmware) => received_firmware = true,
                Ok(Message::CapabilityResponse) => received_capabilities = true,
                Ok(Message::AnalogMappingResponse) => received_analog_mapping = true,
                Ok(_) => {} // Received some other message, continue waiting
                Err(e) => return Err(e),
            }
        }

        self.report_digital(0, 1)?;
        self.report_digital(1, 1)?;

        Ok(())
    }
    /// Creates a new `Board` given a `Read+Write`.
    #[tracing::instrument(err, ret(Display))]
    pub fn new(connection: Box<T>) -> Result<Board<T>> {
        let mut b = Board {
            connection,
            firmware_name: String::new(),
            firmware_version: String::new(),
            protocol_version: String::new(),
            pins: vec![],
            i2c_data: vec![],
        };
        b.initialize_board()?;
        Ok(b)
    }
    /// Tries to create a new `Board` given a `Read+Write`.
    #[tracing::instrument(err, ret(Display))]
    pub fn retry_new(connection: Box<T>) -> Result<Board<T>> {
        let mut b = Board {
            connection,
            firmware_name: String::new(),
            firmware_version: String::new(),
            protocol_version: String::new(),
            pins: vec![],
            i2c_data: vec![],
        };
        b.initialize_board()?;
        Ok(b)
    }
}

impl<T: Read + Write + std::fmt::Debug> Firmata for Board<T> {
    fn pins(&mut self) -> &Vec<Pin> {
        &self.pins
    }
    fn protocol_version(&mut self) -> &String {
        &self.protocol_version
    }
    fn firmware_name(&mut self) -> &String {
        &self.firmware_name
    }
    fn firmware_version(&mut self) -> &String {
        &self.firmware_version
    }
    fn i2c_data(&mut self) -> &mut Vec<I2CReply> {
        &mut self.i2c_data
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn query_analog_mapping(&mut self) -> Result<()> {
        self.write(&[START_SYSEX, ANALOG_MAPPING_QUERY, END_SYSEX])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn query_capabilities(&mut self) -> Result<()> {
        self.write(&[START_SYSEX, CAPABILITY_QUERY, END_SYSEX])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn query_firmware(&mut self) -> Result<()> {
        self.write(&[START_SYSEX, REPORT_FIRMWARE, END_SYSEX])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn i2c_config(&mut self, delay: i32) -> Result<()> {
        self.write(&[
            START_SYSEX,
            I2C_CONFIG,
            (delay & 0xFF) as u8,
            (delay >> 8 & 0xFF) as u8,
            END_SYSEX,
        ])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn i2c_read(&mut self, address: i32, size: i32) -> Result<()> {
        self.write(&[
            START_SYSEX,
            I2C_REQUEST,
            address as u8,
            I2C_READ << 3,
            (size as u8) & SYSEX_REALTIME,
            (size >> 7) as u8 & SYSEX_REALTIME,
            END_SYSEX,
        ])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn i2c_write(&mut self, address: i32, data: &[u8]) -> Result<()> {
        let mut buf = vec![START_SYSEX, I2C_REQUEST, address as u8, I2C_WRITE << 3];

        for i in data.iter() {
            buf.push(i & SYSEX_REALTIME);
            buf.push(((*i as i32) >> 7) as u8 & SYSEX_REALTIME);
        }

        buf.push(END_SYSEX);

        self.write(&buf)
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn report_digital(&mut self, pin: i32, state: i32) -> Result<()> {
        self.write(&[REPORT_DIGITAL | pin as u8, state as u8])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn report_analog(&mut self, pin: i32, state: i32) -> Result<()> {
        self.write(&[REPORT_ANALOG | pin as u8, state as u8])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn analog_write(&mut self, pin: i32, level: i32) -> Result<()> {
        self.pins[pin as usize].value = level;
        self.write(&[
            ANALOG_MESSAGE | pin as u8,
            level as u8 & SYSEX_REALTIME,
            (level >> 7) as u8 & SYSEX_REALTIME,
        ])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn digital_write(&mut self, pin: i32, level: i32) -> Result<()> {
        let port = (pin as f64 / 8f64).floor() as usize;
        let mut value = 0i32;
        let mut i = 0;

        self.pins[pin as usize].value = level;

        while i < 8 {
            if self.pins[8 * port + i].value != 0 {
                value |= 1 << i
            }
            i += 1;
        }

        self.write(&[
            DIGITAL_MESSAGE | port as u8,
            value as u8 & SYSEX_REALTIME,
            (value >> 7) as u8 & SYSEX_REALTIME,
        ])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<()> {
        self.pins[pin as usize].modes = vec![mode];
        self.write(&[SET_PIN_MODE, pin as u8, mode])
    }

    #[tracing::instrument(skip(self), err, ret, level = "DEBUG")]
    fn read_and_decode(&mut self) -> Result<Message> {
        let mut buf = vec![0; 3];
        self.connection
            .read_exact(&mut buf)
            .with_context(|_| StdIoSnafu)?;
        match buf[0] {
            REPORT_VERSION => {
                self.protocol_version = format!("{:o}.{:o}", buf[1], buf[2]);
                Ok(Message::ProtocolVersion)
            }
            ANALOG_MESSAGE..=ANALOG_MESSAGE_BOUND => {
                if buf.len() < 3 {
                    return Err(Error::MessageTooShort);
                }
                let pin = ((buf[0] as i32) & 0x0F) + 14;
                let value = (buf[1] as i32) | ((buf[2] as i32) << 7);
                if self.pins.len() as i32 > pin {
                    self.pins[pin as usize].value = value;
                }
                Ok(Message::Analog)
            }
            DIGITAL_MESSAGE..=DIGITAL_MESSAGE_BOUND => {
                if buf.len() < 3 {
                    return Err(Error::MessageTooShort);
                }
                let port = (buf[0] as i32) & 0x0F;
                let value = (buf[1] as i32) | ((buf[2] as i32) << 7);

                for i in 0..8 {
                    let pin = (8 * port) + i;
                    let mode: u8 = self.pins[pin as usize].mode;
                    if self.pins.len() as i32 > pin && mode == PIN_MODE_INPUT {
                        self.pins[pin as usize].value = (value >> (i & 0x07)) & 0x01;
                    }
                }
                Ok(Message::Digital)
            }
            START_SYSEX => {
                loop {
                    // Read until END_SYSEX.
                    let mut byte = [0];
                    self.connection
                        .read_exact(&mut byte)
                        .with_context(|_| StdIoSnafu)?;
                    buf.push(byte[0]);
                    if byte[0] == END_SYSEX {
                        break;
                    }
                }
                match buf[1] {
                    END_SYSEX => Ok(Message::EmptyResponse),
                    ANALOG_MAPPING_RESPONSE => {
                        let mut i = 2;
                        // Also break before pins indexing is out of bounds.
                        let upper = (buf.len() - 1).min(self.pins.len() + 2);
                        while i < upper {
                            if buf[i] != 127u8 {
                                let pin = &mut self.pins[i - 2];
                                pin.mode = PIN_MODE_ANALOG;
                                pin.modes = vec![PIN_MODE_ANALOG];
                                pin.resolution = DEFAULT_ANALOG_RESOLUTION;
                            }
                            i += 1;
                        }
                        Ok(Message::AnalogMappingResponse)
                    }
                    CAPABILITY_RESPONSE => {
                        let mut i = 2;
                        self.pins = vec![];
                        self.pins.push(Pin::default()); // 0 is unused.
                        let mut modes = vec![];
                        let mut resolution = None;
                        while i < buf.len() - 1 {
                            // Completed a pin, push and continue.
                            if buf[i] == 127u8 {
                                self.pins.push(Pin {
                                    mode: *modes.first().expect("pin mode"),
                                    modes: modes.drain(..).collect(),
                                    resolution: resolution.take().expect("pin resolution"),
                                    value: 0,
                                });

                                i += 1;
                            } else {
                                modes.push(buf[i]);
                                if resolution.is_none() {
                                    // Only keep the first.
                                    resolution.replace(buf[i + 1]);
                                }
                                i += 2;
                            }
                        }
                        Ok(Message::CapabilityResponse)
                    }
                    REPORT_FIRMWARE => {
                        let major = buf.get(2).with_context(|| MessageTooShortSnafu)?;
                        let minor = buf.get(3).with_context(|| MessageTooShortSnafu)?;
                        self.firmware_version = format!("{:o}.{:o}", major, minor);
                        if 4 < buf.len() - 1 {
                            self.firmware_name = std::str::from_utf8(&buf[4..buf.len() - 1])
                                .with_context(|_| Utf8Snafu)?
                                .to_string();
                        }
                        Ok(Message::ReportFirmware)
                    }
                    I2C_REPLY => {
                        let len = buf.len();
                        if len < 8 {
                            return Err(Error::MessageTooShort);
                        }
                        let mut reply = I2CReply {
                            address: (buf[2] as i32) | ((buf[3] as i32) << 7),
                            register: (buf[4] as i32) | ((buf[5] as i32) << 7),
                            data: vec![buf[6] | buf[7] << 7],
                        };
                        let mut i = 8;

                        while i < len - 1 {
                            if buf[i] == 0xF7 {
                                break;
                            }
                            if i + 2 > len {
                                break;
                            }
                            reply.data.push(buf[i] | buf[i + 1] << 7);
                            i += 2;
                        }
                        self.i2c_data.push(reply);
                        Ok(Message::I2CReply)
                    }
                    PIN_STATE_RESPONSE => {
                        let pin = buf[2];
                        if buf[3] == END_SYSEX {
                            return Ok(Message::PinStateResponse);
                        }
                        let pin = &mut self.pins[pin as usize];
                        pin.modes = vec![buf[3]];
                        // TODO: Extended values.
                        pin.value = buf[4] as i32;

                        Ok(Message::PinStateResponse)
                    }
                    _ => Err(Error::UnknownSysEx { code: buf[1] }),
                }
            }
            _ => Err(Error::BadByte { byte: buf[0] }),
        }
    }
}
