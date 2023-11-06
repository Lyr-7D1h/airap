use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex, RwLock,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use audio::pulseaudio::raw;
use error::AirapError;

mod audio;
pub use audio::pulseaudio::Device;
pub mod error;

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

pub struct ThreadPool {
    threads: HashMap<String, JoinHandle<()>>,
}

impl ThreadPool {
    pub fn new() -> Self {
        ThreadPool {
            threads: HashMap::new(),
        }
    }

    pub fn add<F>(&mut self, name: String, f: F) -> Result<(), io::Error>
    where
        F: FnOnce() -> (),
        F: Send + 'static,
    {
        let handle = thread::Builder::new().name(name.clone()).spawn(f)?;
        self.threads.insert(name, handle);
        Ok(())
    }
}

#[derive(Debug)]
pub struct MicroSeconds(pub u64);
#[derive(Debug)]
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
#[derive(Debug)]
pub struct Latency {
    /// Latency from recording to airap
    pub internal: Instant,
    pub airap: Instant,
}

pub struct RawEvent<'a> {
    pub data: &'a [f32],
    pub latency: Latency,
}

pub enum Event<'a> {
    Raw(RawEvent<'a>),
    DefaultDeviceChange,
    MovingAverage,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Feature {
    Raw,
    DefaultDeviceChange,
    MovingAverage,
}

impl ToString for Feature {
    fn to_string(&self) -> String {
        match self {
            Feature::Raw => "raw",
            Feature::DefaultDeviceChange => "default_device_change",
            Feature::MovingAverage => "average",
        }
        .into()
    }
}

pub struct Runner {
    pool: ThreadPool,
    device: Option<Device>,
    features: HashSet<Feature>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            pool: ThreadPool::new(),
            device: None,
            features: HashSet::new(),
        }
    }

    pub fn set_device(&mut self, device: Device) {
        self.device = Some(device);
    }

    pub fn subscribe(&mut self, features: &[Feature]) -> &mut Self {
        self.features = HashSet::from_iter(features.iter().cloned());
        self
    }

    pub fn listen<F>(&mut self, cb: F) -> Result<(), AirapError>
    where
        F: Fn(&mut Self, Event) + Send + 'static,
    {
        let (sender, receiver) = channel();

        let device = if let Some(d) = &self.device {
            d.clone()
        } else {
            Device::default()?
        };

        for feature in self.features.iter() {
            match feature {
                Feature::Raw => {
                    let device = device.clone();
                    let sender = sender.clone();
                    self.pool.add(feature.to_string(), move || {
                        raw(device, |e| {
                            // mlc(Event::Raw(RawEvent { data: e }))
                            sender.send(Event::Raw(e)).unwrap();
                        })
                        .unwrap();
                    })?;
                }
                Feature::DefaultDeviceChange => {}
                Feature::MovingAverage => {
                    let receiver = receiver.clone();
                    self.pool.add(feature.to_string(), move || {
                        receiver.recv().unwrap();
                    });
                }
            }
        }

        loop {
            let data = receiver.recv().unwrap();
            cb(self, data)
        }
    }
}
