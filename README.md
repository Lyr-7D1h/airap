# WIP

# AIRAP - All Inclusive Realtime Audio Processing

The goal of this project is to process audio in as many ways as possible to be used for creative or proffesional? purposes.

## Usage

Run one of the examples

```bash
cargo run --package=average_sampling
```

## Architecture

Event based. Creat AIAP instance with selected features then have an event loop where you get events from these features.

## Roadmap
- Moving average sampling
- Frequency analysis
- Bpm detection
- Non blocking processing
- Multi channel support
- Audio metadata
- Emotion detection
