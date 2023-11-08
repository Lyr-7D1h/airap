# WIP

# AIRAP - All Inclusive Realtime Audio Processing

The goal of this project is to process audio in as many ways as possible to be used for creative or proffesional? purposes.

## Message passing

## Usage

Run one of the examples

```bash
cargo run --package=plotter-minifb
```

## Development

Run thread sanitizer (https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html#threadsanitizer)

```bash
export RUSTFLAGS=-Zsanitizer=thread RUSTDOCFLAGS=-Zsanitizer=thread
cargo run -Zbuild-std --target x86_64-unknown-linux-gnu --package=plotter-minifb
```

## Architecture

Event based. Creat AIAP instance with selected features then have an event loop where you get events from these features.

## Roadmap
- Add slowmotion to plotter to look for discrepencies
- Windows support
- MacOS support
- Universal data formats
- Accurate latency detection
- Moving average sampling
- Frequency analysis
- Bpm detection
- Non blocking processing
- Multi channel support
- Audio metadata
- Emotion detection

## Resources
https://www.reddit.com/r/rust/comments/jadbzs/realtime_programming_in_rust/
https://crates.io/crates/ringbuf
https://lib.rs/crates/crossbeam-channel (performant channels)
https://man7.org/linux/man-pages/man2/pipe.2.html (pipes are used in cpal)