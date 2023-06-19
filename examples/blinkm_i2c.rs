use firmata::*;
use serialport::*;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn init<T: firmata::Firmata>(board: &Arc<Mutex<T>>) {
    let mut b = board.lock().expect("lock");
    b.i2c_config(0).expect("i2c delay set");
    b.i2c_write(0x09, "o".as_bytes()).expect("i2c write");
    thread::sleep(Duration::from_millis(10));
}

fn set_rgb<T: firmata::Firmata>(board: &Arc<Mutex<T>>, rgb: [u8; 3]) {
    let mut b = board.lock().expect("lock");
    b.i2c_write(0x09, "n".as_bytes()).expect("i2c write");
    b.i2c_write(0x09, &rgb).expect("i2c write");
}

fn read_rgb<T: firmata::Firmata>(board: &Arc<Mutex<T>>) -> Vec<u8> {
    {
        let mut b = board.lock().expect("lock");
        b.i2c_write(0x09, "g".as_bytes()).expect("i2c write");
        b.i2c_read(0x09, 3).expect("i2c read");
    }
    loop {
        {
            let mut b = board.lock().expect("lock");
            if b.i2c_data().iter().count() > 0 {
                return b.i2c_data().pop().expect("i2c data").data;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn main() {
    let port = serialport::new("/dev/ttyACM0", 57_600)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .open()
        .expect("an opened serial port");

    let board = Arc::new(Mutex::new(firmata::Board::new(Box::new(port)).unwrap()));

    {
        let b = board.clone();
        thread::spawn(move || loop {
            b.lock().unwrap().read_and_decode().expect("a message");
            b.lock()
                .unwrap()
                .query_firmware()
                .expect("firmware and protocol info");
            thread::sleep(Duration::from_millis(10));
        });
    }

    init(&board);

    set_rgb(&board, [255, 0, 0]);
    println!("rgb: {:?}", read_rgb(&board));
    thread::sleep(Duration::from_millis(1000));

    set_rgb(&board, [0, 255, 0]);
    println!("rgb: {:?}", read_rgb(&board));
    thread::sleep(Duration::from_millis(1000));

    set_rgb(&board, [0, 0, 255]);
    println!("rgb: {:?}", read_rgb(&board));
    thread::sleep(Duration::from_millis(1000));
}
