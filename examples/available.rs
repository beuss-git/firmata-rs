fn main() {
    tracing_subscriber::fmt::init();

    match serialport::available_ports() {
        Ok(ports) => tracing::info!("{:?}", ports),
        Err(err) => tracing::error!("{:?}", err),
    }
}
