use firmata::*;
use serialport::*;
use std::{thread, time::Duration};

fn main() {
    let port = serialport::new("/dev/ttyACM0", 57_600)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .open()
        .expect("an opened serial port");

    let mut b = firmata::Board::new(Box::new(port)).expect("an initialized board");

    println!("firmware version {}", b.firmware_version());
    println!("firmware name {}", b.firmware_name());
    println!("protocol version {}", b.protocol_version());

    b.set_pin_mode(13, firmata::OUTPUT).expect("pin mode set");

    let mut i = 0;

    loop {
        thread::sleep(Duration::from_millis(400));
        println!("{}", i);
        b.digital_write(13, i).expect("digital write");
        i ^= 1;
    }
}
