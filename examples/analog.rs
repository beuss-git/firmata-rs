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

    let pin = 14; // A0

    b.set_pin_mode(pin, firmata_rs::PIN_MODE_ANALOG)
        .expect("pin mode set");

    b.report_analog(pin, 1).expect("reporting state");

    loop {
        b.read_and_decode().expect("a message");
        tracing::info!("analog value: {}", b.pins[pin as usize].value);
        thread::sleep(Duration::from_millis(10));
    }
}
