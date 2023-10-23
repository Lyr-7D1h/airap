#![feature(unsize)]
use audio::{pulseaudio::PulseAudio, Audio};
use error::AirapError;
use log::debug;

pub mod audio;
pub mod error;

pub struct Airap {
    audio: Box<dyn Audio>,
}

impl Airap {
    pub fn new() -> Result<Airap, AirapError> {
        let mut audio: Box<dyn Audio> = if cfg!(unix) {
            debug!("Creating pulseaudio capturing device");
            Box::from(PulseAudio::new())
        } else if cfg!(windows) {
            panic!("Windows is not supported")
        } else if cfg!(macos) {
            panic!("MacOS is not supported")
        } else {
            panic!("Unsupported os")
        };

        audio.on_update(|data| {});

        Ok(Airap { audio })
    }
}
