use crate::error::AirapError;

pub mod pulseaudio;

pub trait Audio {
    fn on_update(&mut self, cb: fn(&[i32])) -> Result<(), AirapError>;
}
