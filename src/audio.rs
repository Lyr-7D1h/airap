use crate::error::AirapError;

pub mod pulseaudio;

pub trait Audio {
    fn on_update(&mut self, op: fn(u16)) -> Result<(), AirapError>;
}
