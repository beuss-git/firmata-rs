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

    let pin = 3;

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    b.set_pin_mode(pin, firmata::PWM).expect("pin set");
    b.analog_write(pin, 0).expect("pin write");
    println!("Starting....");
    thread::sleep(Duration::from_millis(3_000));

    loop {
        for value in (0..255).step_by(5) {
            b.analog_write(pin, value).expect("pin write");
            println!("{}", value);
            thread::sleep(Duration::from_millis(500));
        }
    }
}
