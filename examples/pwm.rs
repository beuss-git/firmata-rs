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
