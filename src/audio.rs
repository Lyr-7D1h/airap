use std::sync::mpsc::Sender;

use crate::error::AirapError;

pub mod pulseaudio;

// pub trait Audio {
//     fn on_update(&self, sender: Sender<&[f32]>) -> Result<(), AirapError>;
// }
