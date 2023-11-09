#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd"
))]
pub mod pulseaudio;

// pub trait Audio {
//     fn on_update(&self, sender: Sender<&[f32]>) -> Result<(), AirapError>;
// }
