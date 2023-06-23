//! This module contains a client implementation of the
//! [Firmata Protocol](https://github.com/firmata/protocol)

use snafu::{OptionExt, ResultExt, Snafu};
use std::io::{Read, Write};

pub const ENCODER_DATA: u8 = 0x61;
pub const ANALOG_MAPPING_QUERY: u8 = 0x69;
pub const ANALOG_MAPPING_RESPONSE: u8 = 0x6A;
pub const CAPABILITY_QUERY: u8 = 0x6B;
pub const CAPABILITY_RESPONSE: u8 = 0x6C;
pub const PIN_STATE_QUERY: u8 = 0x6D;
pub const PIN_STATE_RESPONSE: u8 = 0x6E;
pub const EXTENDED_ANALOG: u8 = 0x6F;
pub const SERVO_CONFIG: u8 = 0x70;
pub const STRING_DATA: u8 = 0x71;
pub const STEPPER_DATA: u8 = 0x72;
pub const ONEWIRE_DATA: u8 = 0x73;
pub const SHIFT_DATA: u8 = 0x75;
pub const I2C_REQUEST: u8 = 0x76;
pub const I2C_REPLY: u8 = 0x77;
pub const I2C_CONFIG: u8 = 0x78;
pub const I2C_MODE_WRITE: u8 = 0x00;
pub const I2C_MODE_READ: u8 = 0x01;
pub const REPORT_FIRMWARE: u8 = 0x79;
pub const PROTOCOL_VERSION: u8 = 0xF9;
pub const SAMPLING_INTERVAL: u8 = 0x7A;
pub const SCHEDULER_DATA: u8 = 0x7B;
pub const SYSEX_NON_REALTIME: u8 = 0x7E;
pub const SYSEX_REALTIME: u8 = 0x7F;
pub const START_SYSEX: u8 = 0xF0;
pub const END_SYSEX: u8 = 0xF7;
pub const PIN_MODE: u8 = 0xF4;
pub const REPORT_DIGITAL: u8 = 0xD0;
pub const REPORT_ANALOG: u8 = 0xC0;
pub const DIGITAL_MESSAGE: u8 = 0x90;
pub const DIGITAL_MESSAGE_BOUND: u8 = 0x9F;
pub const ANALOG_MESSAGE: u8 = 0xE0;
pub const ANALOG_MESSAGE_BOUND: u8 = 0xEF;

pub const INPUT: u8 = 0;
pub const OUTPUT: u8 = 1;
pub const ANALOG: u8 = 2;
pub const PWM: u8 = 3;
pub const SERVO: u8 = 4;
pub const I2C: u8 = 6;
pub const ONEWIRE: u8 = 7;
pub const STEPPER: u8 = 8;
pub const ENCODER: u8 = 9;

/// Firmata error type.
#[derive(Debug, Snafu)]
pub enum Error {
    UnknownSysEx { code: u8 },
    BadByte { byte: u8 },
    StdIoError { source: std::io::Error },
    Utf8Error { source: std::str::Utf8Error },
    MessageTooShort,
    AttemptsExceeded,
    TimeoutExceeded,
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

/// An available pin mode.
#[derive(Debug, Default)]
pub struct Mode {
    pub mode: u8,
    pub resolution: u8,
}

/// The current state and configuration of a pin.
#[derive(Debug, Default)]
pub struct Pin {
    pub modes: Vec<Mode>,
    pub analog: bool,
    pub value: i32,
    pub mode: u8,
}

/// Firmata board functionality.
pub trait Firmata: std::fmt::Debug {
    /// Get the raw I2C replies that have been read from the board.
    fn i2c_data(&mut self) -> &mut Vec<I2CReply>;
    /// Get pins that the board has access to.
    fn pins(&mut self) -> &Vec<Pin>;
    /// Get the current Firmata protocol version.
    fn protocol_version(&mut self) -> &String;
    /// Get the firmware name.
    fn firmware_name(&mut self) -> &String;
    /// Get the firmware version.
    fn firmware_version(&mut self) -> &String;
    /// Query the board for available analog pins.
    fn query_analog_mapping(&mut self) -> Result<()>;
    /// Query the board for all available capabilities.
    fn query_capabilities(&mut self) -> Result<()>;
    /// Query the board for current firmware and protocol information.
    fn query_firmware(&mut self) -> Result<()>;
    /// Configure the `delay` in microseconds for I2C devices that require a delay between when the
    /// register is written to and the data in that register can be read.
    fn i2c_config(&mut self, delay: i32) -> Result<()>;
    /// Read `size` bytes from I2C device at the specified `address`.
    fn i2c_read(&mut self, address: i32, size: i32) -> Result<()>;
    /// Write `data` to the I2C device at the specified `address`.
    fn i2c_write(&mut self, address: i32, data: &[u8]) -> Result<()>;
    /// Set the digital reporting `state` of the specified `pin`.
    fn report_digital(&mut self, pin: i32, state: i32) -> Result<()>;
    /// Set the analog reporting `state` of the specified `pin`.
    fn report_analog(&mut self, pin: i32, state: i32) -> Result<()>;
    /// Write `level` to the analog `pin`.
    fn analog_write(&mut self, pin: i32, level: i32) -> Result<()>;
    /// Write `level` to the digital `pin`.
    fn digital_write(&mut self, pin: i32, level: i32) -> Result<()>;
    /// Set the `mode` of the specified `pin`.
    fn set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<()>;
    /// Read from the Firmata device, parse one Firmata message and return its type.
    fn read_and_decode(&mut self) -> Result<Message>;
}

/// Firmata board functionality that retries and fallible methods.
pub trait RetryFirmata: Firmata {
    /// Backoff strategy.
    fn backoff(&self) -> backoff::ExponentialBackoff {
        backoff::ExponentialBackoff::default()
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
        b.retry_query_firmware()?;
        b.retry_read_and_decode()?;
        b.retry_read_and_decode()?;
        b.retry_query_capabilities()?;
        b.retry_read_and_decode()?;
        b.retry_query_analog_mapping()?;
        b.retry_read_and_decode()?;
        b.retry_report_digital(0, 1)?;
        b.retry_report_digital(1, 1)?;
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

    #[tracing::instrument(skip(self), err)]
    fn query_analog_mapping(&mut self) -> Result<()> {
        self.connection
            .write(&mut [START_SYSEX, ANALOG_MAPPING_QUERY, END_SYSEX])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn query_capabilities(&mut self) -> Result<()> {
        self.connection
            .write(&mut [START_SYSEX, CAPABILITY_QUERY, END_SYSEX])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn query_firmware(&mut self) -> Result<()> {
        self.connection
            .write(&mut [START_SYSEX, REPORT_FIRMWARE, END_SYSEX])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn i2c_config(&mut self, delay: i32) -> Result<()> {
        self.connection
            .write(&mut [
                START_SYSEX,
                I2C_CONFIG,
                (delay & 0xFF) as u8,
                (delay >> 8 & 0xFF) as u8,
                END_SYSEX,
            ])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn i2c_read(&mut self, address: i32, size: i32) -> Result<()> {
        self.connection
            .write(&mut [
                START_SYSEX,
                I2C_REQUEST,
                address as u8,
                I2C_MODE_READ << 3,
                (size as u8) & SYSEX_REALTIME,
                (size >> 7) as u8 & SYSEX_REALTIME,
                END_SYSEX,
            ])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn i2c_write(&mut self, address: i32, data: &[u8]) -> Result<()> {
        let mut buf = vec![];

        buf.push(START_SYSEX);
        buf.push(I2C_REQUEST);
        buf.push(address as u8);
        buf.push(I2C_MODE_WRITE << 3);

        for i in data.iter() {
            buf.push(i & SYSEX_REALTIME);
            buf.push(((*i as i32) >> 7) as u8 & SYSEX_REALTIME);
        }

        buf.push(END_SYSEX);

        self.connection
            .write(&mut buf[..])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn report_digital(&mut self, pin: i32, state: i32) -> Result<()> {
        self.connection
            .write(&mut [REPORT_DIGITAL | pin as u8, state as u8])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn report_analog(&mut self, pin: i32, state: i32) -> Result<()> {
        self.connection
            .write(&mut [REPORT_ANALOG | pin as u8, state as u8])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn analog_write(&mut self, pin: i32, level: i32) -> Result<()> {
        self.pins[pin as usize].value = level;

        self.connection
            .write(&mut [
                ANALOG_MESSAGE | pin as u8,
                level as u8 & SYSEX_REALTIME,
                (level >> 7) as u8 & SYSEX_REALTIME,
            ])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
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

        self.connection
            .write(&mut [
                DIGITAL_MESSAGE | port as u8,
                value as u8 & SYSEX_REALTIME,
                (value >> 7) as u8 & SYSEX_REALTIME,
            ])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err)]
    fn set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<()> {
        self.pins[pin as usize].mode = mode;
        self.connection
            .write(&mut [PIN_MODE, pin as u8, mode])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    #[tracing::instrument(skip(self), err, ret)]
    fn read_and_decode(&mut self) -> Result<Message> {
        let mut buf = vec![0; 3];
        self.connection
            .read_exact(&mut buf)
            .with_context(|_| StdIoSnafu)?;
        match buf[0] {
            PROTOCOL_VERSION => {
                self.protocol_version = format!("{:o}.{:o}", buf[1], buf[2]);
                Ok(Message::ProtocolVersion)
            }
            ANALOG_MESSAGE..=ANALOG_MESSAGE_BOUND => {
                if buf.len() < 3 {
                    return Err(Error::MessageTooShort);
                }
                let value = (buf[1] as i32) | ((buf[2] as i32) << 7);
                let pin = ((buf[0] as i32) & 0x0F) + 14;
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
                    if self.pins.len() as i32 > pin && self.pins[pin as usize].mode == INPUT {
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
                                self.pins[i - 2].analog = true;
                            }
                            i += 1;
                        }
                        Ok(Message::AnalogMappingResponse)
                    }
                    CAPABILITY_RESPONSE => {
                        let mut pin = 0;
                        let mut i = 2;
                        self.pins = vec![];
                        self.pins.push(Pin::default());
                        while i < buf.len() - 1 {
                            if buf[i] == 127u8 {
                                pin += 1;
                                i += 1;
                                self.pins.push(Pin::default());
                                continue;
                            }
                            self.pins[pin].modes.push(Mode {
                                mode: buf[i],
                                resolution: buf[i + 1],
                            });
                            i += 2;
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
                    _ => Err(Error::UnknownSysEx { code: buf[1] }),
                }
            }
            _ => Err(Error::BadByte { byte: buf[0] }),
        }
    }
}
