//! This module contains a client implementation of the
//! [Firmata Protocol](https://github.com/firmata/protocol)

use snafu::{OptionExt, ResultExt, Snafu};
use std::io::{Read, Write};
use std::str;
use std::time::Duration;

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
pub trait Firmata {
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
    fn query_analog_mapping(&mut self) -> Result<(), Error>;
    /// Query the board for all available capabilities.
    fn query_capabilities(&mut self) -> Result<(), Error>;
    /// Query the board for current firmware and protocol information.
    fn query_firmware(&mut self) -> Result<(), Error>;
    /// Configure the `delay` in microseconds for I2C devices that require a delay between when the
    /// register is written to and the data in that register can be read.
    fn i2c_config(&mut self, delay: i32) -> Result<(), Error>;
    /// Read `size` bytes from I2C device at the specified `address`.
    fn i2c_read(&mut self, address: i32, size: i32) -> Result<(), Error>;
    /// Write `data` to the I2C device at the specified `address`.
    fn i2c_write(&mut self, address: i32, data: &[u8]) -> Result<(), Error>;
    /// Set the digital reporting `state` of the specified `pin`.
    fn report_digital(&mut self, pin: i32, state: i32) -> Result<(), Error>;
    /// Set the analog reporting `state` of the specified `pin`.
    fn report_analog(&mut self, pin: i32, state: i32) -> Result<(), Error>;
    /// Write `level` to the analog `pin`.
    fn analog_write(&mut self, pin: i32, level: i32) -> Result<(), Error>;
    /// Write `level` to the digital `pin`.
    fn digital_write(&mut self, pin: i32, level: i32) -> Result<(), Error>;
    /// Set the `mode` of the specified `pin`.
    fn set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<(), Error>;
    /// Read from the Firmata device, parse one Firmata message and return its type.
    fn read_and_decode(&mut self) -> Result<Message, Error>;
    /// Read messages until one with the given SysEx passes.
    fn read_and_decode_backoff(&mut self, back_off: BackOff) -> Result<(), Error>;
}

/// Back-off instructions.
#[derive(Debug, Default)]
pub struct BackOff {
    /// Message to look out for. Retry with another attempt if it doesn't match.
    pub message: Option<Message>,
    /// Timeout duration.
    pub timeout: Option<Duration>,
    /// Maximum number of attempts.
    pub attempts: Option<usize>,
}

/// A Firmata board representation.
pub struct Board<T: Read + Write> {
    pub connection: Box<T>,
    pub pins: Vec<Pin>,
    pub i2c_data: Vec<I2CReply>,
    pub protocol_version: String,
    pub firmware_name: String,
    pub firmware_version: String,
    pub back_off: BackOff,
}

impl<T: Read + Write> Board<T> {
    /// Creates a new `Board` given a `Read+Write`.
    pub fn new(connection: Box<T>, back_off: BackOff) -> Result<Board<T>, Error> {
        let mut b = Board {
            connection,
            firmware_name: String::new(),
            firmware_version: String::new(),
            protocol_version: String::new(),
            pins: vec![],
            i2c_data: vec![],
            back_off,
        };

        b.query_firmware()?;
        b.read_and_decode()?;
        b.read_and_decode()?;
        b.query_capabilities()?;
        b.read_and_decode()?;
        b.query_analog_mapping()?;
        b.read_and_decode()?;
        b.report_digital(0, 1)?;
        b.report_digital(1, 1)?;

        Ok(b)
    }
}

impl<T: Read + Write> Firmata for Board<T> {
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
    fn query_analog_mapping(&mut self) -> Result<(), Error> {
        self.connection
            .write(&mut [START_SYSEX, ANALOG_MAPPING_QUERY, END_SYSEX])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    fn query_capabilities(&mut self) -> Result<(), Error> {
        self.connection
            .write(&mut [START_SYSEX, CAPABILITY_QUERY, END_SYSEX])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    fn query_firmware(&mut self) -> Result<(), Error> {
        self.connection
            .write(&mut [START_SYSEX, REPORT_FIRMWARE, END_SYSEX])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    fn i2c_config(&mut self, delay: i32) -> Result<(), Error> {
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

    fn i2c_read(&mut self, address: i32, size: i32) -> Result<(), Error> {
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

    fn i2c_write(&mut self, address: i32, data: &[u8]) -> Result<(), Error> {
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

    fn report_digital(&mut self, pin: i32, state: i32) -> Result<(), Error> {
        self.connection
            .write(&mut [REPORT_DIGITAL | pin as u8, state as u8])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    fn report_analog(&mut self, pin: i32, state: i32) -> Result<(), Error> {
        self.connection
            .write(&mut [REPORT_ANALOG | pin as u8, state as u8])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    fn analog_write(&mut self, pin: i32, level: i32) -> Result<(), Error> {
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

    fn digital_write(&mut self, pin: i32, level: i32) -> Result<(), Error> {
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

    fn set_pin_mode(&mut self, pin: i32, mode: u8) -> Result<(), Error> {
        self.pins[pin as usize].mode = mode;
        self.connection
            .write(&mut [PIN_MODE, pin as u8, mode])
            .map(|_| ())
            .with_context(|_| StdIoSnafu)
    }

    fn read_and_decode(&mut self) -> Result<Message, Error> {
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
                            self.firmware_name = str::from_utf8(&buf[4..buf.len() - 1])
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
