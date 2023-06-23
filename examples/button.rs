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

    let led = 13;
    let button = 2;

    b.retry_set_pin_mode(led, firmata_rs::OUTPUT)
        .expect("pin mode set");
    b.retry_set_pin_mode(button, firmata_rs::INPUT)
        .expect("pin mode set");

    b.retry_report_digital(button, 1)
        .expect("digital reporting mode");

    tracing::info!("Starting loop...");

    loop {
        b.retry_read_and_decode().expect("a message");
        if b.pins()[button as usize].value == 0 {
            tracing::info!("off");
            b.retry_digital_write(led, 0).expect("digital write");
        } else {
            tracing::info!("on");
            b.retry_digital_write(led, 1).expect("digital write");
        }

        thread::sleep(Duration::from_millis(100));
    }
}
