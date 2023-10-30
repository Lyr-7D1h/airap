use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, Builder},
    time::Duration,
};

use audio::pulseaudio::PulseAudio;
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
