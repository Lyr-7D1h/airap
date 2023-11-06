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

pub struct FeatureThreadPool {
    threads: HashMap<String, JoinHandle<()>>,
}

impl FeatureThreadPool {
    pub fn new() -> Self {
        FeatureThreadPool {
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

#[derive(Debug, Clone)]
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
    pool: FeatureThreadPool,
    device: Option<Device>,
    features: HashSet<Feature>,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            pool: FeatureThreadPool::new(),
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
        let (raw_tx, raw_rx) = channel();
        let (moving_average_tx, moving_average_rx) = channel();

        let device = if let Some(d) = &self.device {
            d.clone()
        } else {
            Device::default()?
        };

        if let Some(feature) = self.features.get(&Feature::Raw) {
            let device = device.clone();
            self.pool.add(feature.to_string(), move || {
                raw(device, |e| {
                    raw_tx.send(Event::Raw(e)).unwrap();
                })
                .unwrap();
            })?;
        }
        if let Some(feature) = self.features.get(&Feature::MovingAverage) {
            // let device = device.clone();
            self.pool.add(feature.to_string(), move || {
                moving_average_rx.recv().unwrap();
            })?;
        }

        loop {
            let data = raw_rx.recv().unwrap();

            match data {
                Event::Raw(e) => moving_average_tx.send(e.clone()).unwrap(),
                Event::DefaultDeviceChange => todo!(),
                Event::MovingAverage => todo!(),
            }

            cb(self, data)
        }
    }
}
