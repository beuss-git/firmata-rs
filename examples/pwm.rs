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

    let pin = 3;

    b.retry_set_pin_mode(pin, firmata_rs::PWM).expect("pin set");
    b.retry_analog_write(pin, 0).expect("pin write");

    tracing::info!("Starting loop...");

    loop {
        for value in (0..255).step_by(5) {
            b.analog_write(pin, value).expect("pin write");
            tracing::info!("{}", value);
            thread::sleep(Duration::from_millis(500));
        }
    }
}
