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
pub mod feature;
mod latency;
pub use audio::pulseaudio::Device;
use feature::{feature_flags, Feature, FeatureStore};
use latency::{Instant, Latency};
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

pub struct FeatureThread<'a> {
    handle: JoinHandle<()>,
    signal_tx: Sender<Event<'a>>,
}
pub struct FeatureThreadPool<'a, 'b> {
    threads: HashMap<u32, FeatureThread<'a>>,
    context: ThreadContext,
    feature_store: &'b FeatureStore,
    event_rx: Receiver<Event<'a>>,
}
impl<'a: 'static, 'b> FeatureThreadPool<'a, 'b> {
    pub fn new(context: ThreadContext, feature_store: &'b FeatureStore) -> Self {
        let (event_tx, event_rx) = channel::<Event<'a>>();

        let mut threads = HashMap::new();

        if let Some(f) = feature_store.get(&feature_flags::RAW) {
            let (signal_tx, signal_rx) = channel();
            let event_tx = event_tx.clone();
            let context = context.clone();
            let handle = thread::Builder::new()
                .name(f.to_string())
                .spawn(move || {
                    raw(&context.device, |e| {
                        event_tx.send(Event::Raw(e)).unwrap();
                    })
                    .unwrap();
                })
                .unwrap();

            threads.insert(feature_flags::RAW, FeatureThread { handle, signal_tx });
        }

        if let Some(f) = feature_store.get(&feature_flags::MOVING_AVERAGE) {
            let (signal_tx, signal_rx) = channel();
            let event_tx = event_tx.clone();
            let handle = thread::Builder::new()
                .name(f.to_string())
                .spawn(move || loop {
                    let e = signal_rx.recv().unwrap();
                    match e {
                        Event::Raw(r) => {
                            let sum: f32 = r.data.iter().sum();
                            event_tx
                                .send(Event::MovingAverage(MovingAverageEvent {
                                    average: vec![sum / r.data.len() as f32],
                                    latency: Latency {
                                        internal: Instant::None,
                                        airap: Instant::None,
                                    },
                                }))
                                .unwrap();
                        }
                        Event::DefaultDeviceChange => todo!(),
                        _ => {}
                    }
                })
                .unwrap();

            threads.insert(
                feature_flags::MOVING_AVERAGE,
                FeatureThread { handle, signal_tx },
            );
        }

        FeatureThreadPool {
            threads,
            context,
            feature_store,
            event_rx,
        }
    }

    pub fn run<F>(&self, cb: F)
    where
        F: Fn(Event) + Send + 'static,
    {
        loop {
            let event = self.event_rx.recv().unwrap();

            match event {
                Event::Raw(_) => {
                    if self.feature_store.contains(feature_flags::MOVING_AVERAGE) {
                        self.threads
                            .get(&feature_flags::MOVING_AVERAGE)
                            .unwrap()
                            .signal_tx
                            .send(event.clone())
                            .unwrap();
                    }
                }
                Event::DefaultDeviceChange => {}
                Event::MovingAverage(_) => {}
            }

            cb(event)
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawEvent<'a> {
    pub data: &'a [f32],
    pub latency: Latency,
}

#[derive(Debug, Clone)]
pub struct MovingAverageEvent {
    pub average: Vec<f32>,
    pub latency: Latency,
}

#[derive(Debug, Clone)]
pub enum Event<'a> {
    Raw(RawEvent<'a>),
    DefaultDeviceChange,
    MovingAverage(MovingAverageEvent),
}

#[derive(Debug, Clone)]
pub struct ThreadContext {
    device: Device,
}

pub struct Runner {
    device: Option<Device>,
    feature_store: FeatureStore,
}

impl Runner {
    pub fn new() -> Self {
        Self {
            device: None,
            feature_store: FeatureStore::new(),
        }
    }

    pub fn set_device(&mut self, device: Device) {
        self.device = Some(device);
    }

    pub fn subscribe(
        &mut self,
        features: &[Feature],
        // features: &[impl FeatureImpl + Default + Clone + 'static],
    ) -> Result<&mut Self, AirapError> {
        self.feature_store.set_features(features);
        Ok(self)
    }

    pub fn listen<F>(&mut self, cb: F) -> Result<(), AirapError>
    where
        F: Fn(Event) + Send + 'static,
    {
        // let (raw_tx, raw_rx) = channel(); // TODO make bounded and error on too many and too late message
        // let (moving_average_tx, moving_average_rx) = channel();

        let device = if let Some(d) = &self.device {
            d.clone()
        } else {
            Device::default()?
        };

        let context: ThreadContext = ThreadContext { device };

        let pool = FeatureThreadPool::new(context, &self.feature_store);
        pool.run(move |e| cb(e));
        Ok(())

        // if let Some(feature) = self.feature_store.get_mut(Feature::RAW) {
        //     pool.add(feature.to_string(), feature)?;
        // }

        // if let Some(feature) = self.feature_store.get(Feature::MOVING_AVERAGE) {
        //     // let device = device.clone();
        //     let raw_tx = raw_tx.clone();
        //     pool.add(feature.to_string(), move || {
        //         let e = moving_average_rx.recv().unwrap();
        //         match e {
        //             Event::Raw(r) => {
        //                 let sum: f32 = r.data.iter().sum();
        //                 raw_tx
        //                     .send(Event::MovingAverage(MovingAverageEvent {
        //                         average: sum / r.data.len() as f32,
        //                         latency: Latency {
        //                             internal: Instant::None,
        //                             airap: Instant::None,
        //                         },
        //                     }))
        //                     .unwrap();
        //             }
        //             Event::DefaultDeviceChange => todo!(),
        //             _ => {}
        //         }
        //     })?;
        // }

        // loop {
        //     let data = raw_rx.recv().unwrap();

        // match data {
        //     Event::Raw(_) => {
        //         if self.features.contains(&Feature::MovingAverage) {
        //             moving_average_tx.send(data.clone()).unwrap()
        //         }
        //     }
        //     Event::DefaultDeviceChange => {}
        //     Event::MovingAverage(_) => {}
        // }

        //     cb(self, data)
        // }
    }
}
