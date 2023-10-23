use psimple::Simple;
use pulse::sample::{Format, Spec};
use pulse::stream::Direction;

fn main() {
    let spec = Spec {
        format: Format::S16NE,
        channels: 2,
        rate: 44100,
    };
    assert!(spec.is_valid());

    let s = Simple::new(
        None,              // Use the default server
        "AIRAP",           // Our application’s name
        Direction::Record, // We want a playback stream
        None,              // Use the default device
        "Desktop Audio",   // Description of our stream
        &spec,             // Our sample format
        None,              // Use default channel map
        None,              // Use default buffering attributes
    )
    .unwrap();

    let mut buf: [u8; 1000] = [0; 1000];
    loop {
        s.read(&mut buf).unwrap();
        println!("{buf:?}");
    }
    println!("{:?}", s.get_latency());
    loop {}
}
