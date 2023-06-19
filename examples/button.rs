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

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    let led = 13;
    let button = 2;

    b.set_pin_mode(led, firmata::OUTPUT).expect("pin mode set");
    b.set_pin_mode(button, firmata::INPUT)
        .expect("pin mode set");

    b.report_digital(button, 1).expect("digital reporting mode");

    loop {
        b.read_and_decode().expect("a message");
        if b.pins()[button as usize].value == 0 {
            println!("off");
            b.digital_write(led, 0).expect("digital write");
        } else {
            println!("on");
            b.digital_write(led, 1).expect("digital write");
        }

        thread::sleep(Duration::from_millis(100));
    }
}
