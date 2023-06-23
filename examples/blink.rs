use firmata_rs::*;
use serialport::*;
use std::{thread, time::Duration};

fn main() {
    tracing_subscriber::fmt::init();

    let port = serialport::new("/dev/ttyACM0", 57_600)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(1000))
        .open()
        .expect("an opened serial port");

    let mut b = firmata_rs::Board::new(Box::new(port)).expect("new board");

    b.retry_set_pin_mode(13, firmata_rs::OUTPUT)
        .expect("pin mode set");

    let mut i = 0;

    loop {
        thread::sleep(Duration::from_millis(400));
        b.retry_digital_write(13, i).expect("digital write");
        i ^= 1;
    }
}
