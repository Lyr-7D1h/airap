use airap::Airap;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().env().init().unwrap();
    let airap = Airap::new();
}
