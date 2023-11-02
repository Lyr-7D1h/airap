use std::time::Duration;

use audio::pulseaudio::{PulseAudio, Source};
use error::AirapError;

pub mod audio;
pub mod error;

pub enum Feature {
    Raw,
}

pub struct Options {
    max_latency: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_latency: Duration::from_millis(5),
        }
    }
}

pub struct Runner {}

pub struct Airap {
    // audio: Box<dyn Audio>,
    audio: PulseAudio,
}

impl Airap {
    pub fn new() -> Result<Airap, AirapError> {
        // let audio = if cfg!(unix) {
        //     debug!("Creating pulseaudio capturing device");
        //     Box::from(PulseAudio::new())
        // } else if cfg!(windows) {
        //     panic!("Windows is not supported")
        // } else if cfg!(macos) {
        //     panic!("MacOS is not supported")
        // } else {
        //     panic!("Unsupported os")
        // };

        let audio = PulseAudio::new();
        Ok(Airap { audio })
    }

    pub fn default_device() -> Result<Source, AirapError> {
        Source::default_source()
    }

    pub fn on_default_device() {}

    // pub fn on_moving_average(&mut self, options: Options, cb: F) {}

    /// Send data to a callback
    pub fn on_raw<F>(&mut self, options: Options, cb: F)
    where
        F: Fn(&[f32]) + Send + 'static,
    {
        self.audio.on_raw(cb);
    }

    pub fn start() {
        loop {}
    }
}
