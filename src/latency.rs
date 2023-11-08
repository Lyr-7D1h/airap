#[derive(Debug, Clone)]
pub struct MicroSeconds(pub u64);
#[derive(Debug, Clone)]
pub enum Instant {
    /// No latency.
    None,
    /// A positive (greater than zero) amount of latency.
    Positive(MicroSeconds),
    /// A negative (less than zero) amount of latency.
    Negative(MicroSeconds),
}
impl From<pulse::stream::Latency> for Instant {
    fn from(value: pulse::stream::Latency) -> Self {
        match value {
            pulse::stream::Latency::None => Instant::None,
            pulse::stream::Latency::Positive(s) => Instant::Positive(MicroSeconds(s.0)),
            pulse::stream::Latency::Negative(s) => Instant::Negative(MicroSeconds(s.0)),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Latency {
    /// Latency from recording to airap
    pub internal: Instant,
    pub airap: Instant,
}
