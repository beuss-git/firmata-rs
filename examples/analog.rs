extern crate firmata;
extern crate serial;

use firmata::*;
use serial::*;
use std::{thread, time::Duration};

fn main() {
    let mut sp = serial::open("/dev/ttyACM0").unwrap();

    sp.reconfigure(&|settings| {
        settings.set_baud_rate(Baud57600).unwrap();
        settings.set_char_size(Bits8);
        settings.set_parity(ParityNone);
        settings.set_stop_bits(Stop1);
        settings.set_flow_control(FlowNone);
        Ok(())
    })
    .unwrap();

    let mut b = firmata::Board::new(Box::new(sp)).unwrap();

    let pin = 14; // A0

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    b.set_pin_mode(pin, firmata::ANALOG).unwrap();

    b.report_analog(pin, 1).unwrap();

    loop {
        b.read_and_decode().unwrap();
        println!("analog value: {}", b.pins[pin as usize].value);
        thread::sleep(Duration::from_millis(10));
    }
}
