use firmata::*;
use serialport::*;
use std::{thread, time::Duration};

fn main() {
    let port = serialport::new("/dev/ttyACM0", 57_600)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(1000))
        .open()
        .expect("an opened serial port");

    let mut b = firmata::Board::new(Box::new(port)).expect("an initialized board");

    let pin = 14; // A0

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    b.set_pin_mode(pin, firmata::ANALOG).expect("pin mode set");

    b.report_analog(pin, 1).expect("reporting state");

    loop {
        b.read_and_decode().expect("a message");
        println!("analog value: {}", b.pins[pin as usize].value);
        thread::sleep(Duration::from_millis(10));
    }
}
