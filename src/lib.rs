use audio::{pulseaudio::PulseAudio, Audio};
use error::AirapError;
use log::debug;

pub mod audio;
pub mod error;

pub enum Feature {
    Raw,
}

pub struct Airap {
    audio: Box<dyn Audio>,
}

impl Airap {
    pub fn new() -> Result<Airap, AirapError> {
        let audio: Box<dyn Audio> = if cfg!(unix) {
            debug!("Creating pulseaudio capturing device");
            Box::from(PulseAudio::new())
        } else if cfg!(windows) {
            panic!("Windows is not supported")
        } else if cfg!(macos) {
            panic!("MacOS is not supported")
        } else {
            panic!("Unsupported os")
        };

        Ok(Airap { audio })
    }
}

impl Audio for Airap {
    fn on_update(&mut self, cb: fn(&[i32])) -> Result<(), AirapError> {
        self.audio.on_update(cb)
    }
}
