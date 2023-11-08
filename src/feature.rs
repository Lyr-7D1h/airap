use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    iter::FilterMap,
    slice::{Iter, IterMut},
    sync::mpsc::{Receiver, Sender},
};

use log::info;

use crate::{audio::pulseaudio::raw, error::AirapError, Event, ThreadContext};

#[derive(Debug, Clone)]
pub struct RawFeature {
    /// For what latency should we aim in micro seconds (eg 5000 = 5ms)
    pub buffer_latency: u32,
}
impl Default for RawFeature {
    fn default() -> Self {
        Self {
            buffer_latency: 5000,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Feature {
    Raw { buffer_latency: u32 },
    DefaultDeviceChange,
    MovingAverage,
}
impl Feature {
    pub fn default(flag: u32) -> Self {
        match flag {
            feature_flags::RAW => Feature::Raw {
                buffer_latency: 5000,
            },
            feature_flags::DEFAULT_DEVICE_CHANGE => Feature::DefaultDeviceChange,
            feature_flags::MOVING_AVERAGE => Feature::MovingAverage,
            _ => panic!("feature flag '{flag}' does not have a default implementation"),
        }
    }
    pub fn dependencies(&self) -> u32 {
        match self {
            Feature::Raw { .. } => feature_flags::NONE,
            Feature::DefaultDeviceChange => feature_flags::NONE,
            Feature::MovingAverage => feature_flags::RAW,
        }
    }
    pub fn to_flag(&self) -> u32 {
        match self {
            Feature::Raw { .. } => feature_flags::RAW,
            Feature::DefaultDeviceChange => feature_flags::DEFAULT_DEVICE_CHANGE,
            Feature::MovingAverage => feature_flags::MOVING_AVERAGE,
        }
    }
}
impl ToString for Feature {
    fn to_string(&self) -> String {
        match self {
            Feature::Raw { .. } => "raw",
            Feature::DefaultDeviceChange => "default_device_change",
            Feature::MovingAverage => "moving_average",
        }
        .into()
    }
}

pub mod feature_flags {
    pub const NONE: u32 = 0x00;
    pub const RAW: u32 = 0x01;
    pub const DEFAULT_DEVICE_CHANGE: u32 = 0x02;
    pub const MOVING_AVERAGE: u32 = 0x04;
}

#[derive(Debug, Clone)]
pub struct FeatureStore {
    store: HashMap<u32, Feature>,
    enabled_features: u32,
}

impl FeatureStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
            enabled_features: 0,
        }
    }

    pub fn set_features(&mut self, features: &[Feature]) {
        self.store.clear();
        self.enabled_features = 0;

        for f in features.into_iter() {
            let flag = f.to_flag();
            self.enabled_features |= flag | f.dependencies();
            self.store.insert(flag, f.clone());
        }

        for i in 1..self.enabled_features.ilog2() {
            let flag = i.pow(2);
            if self.enabled_features & flag > 0 && !self.store.contains_key(&flag) {
                info!("Adding dependency with default settings ({flag})");
                self.store.insert(flag, Feature::default(flag));
            }
        }
    }

    pub fn get(&self, flag: &u32) -> Option<&Feature> {
        self.store.get(flag)
    }

    #[inline]
    pub fn contains(&self, flag: u32) -> bool {
        self.enabled_features & flag > 0
    }
}

// pub struct FeatureStore {
//     state: HashMap<u8, Box<dyn FeatureImpl>>,
//     pub enabled_features: u32,
// }

// impl<'a> FeatureStore {
//     pub fn new() -> Self {
//         let mut features = vec![];
//         for _ in 0..32 {
//             features.push(None);
//         }
//         FeatureStore {
//             state: features,
//             enabled_features: 0,
//         }
//     }

//     fn insert(&mut self, flag: u32, value: Box<dyn FeatureImpl>) {
//         self.state.insert(flag.ilog2() as usize, Some(value));
//     }

//     pub fn set_features(
//         &mut self,
//         features: &[impl FeatureImpl + Default + Clone + 'static],
//     ) -> Result<(), AirapError> {
//         // reset if features set
//         if self.enabled_features != 0 {
//             self.state.clear();
//             self.enabled_features = 0;
//         }

//         for f in features.into_iter().cloned() {
//             let id = f.id();
//             self.enabled_features |= id | f.dependencies();
//             self.insert(id, Box::new(f));
//         }

//         for i in 0..self.enabled_features.ilog2() {
//             let flag = (i + 1).pow(2);
//             if flag & self.enabled_features > 0 && self.state[i as usize].is_none() {
//                 info!("Adding dependency ({flag}) with default options");
//                 let feature = match flag {
//                     feature_flags::RAW => RawFeature::default(),
//                     _ => {
//                         return Err(AirapError::feature(format!(
//                             "no default implementation found for {flag}"
//                         )))
//                     }
//                 };
//                 self.insert(flag, Box::new(feature));
//             }
//         }

//         Ok(())
//     }

//     pub fn features(self) -> Vec<Box<dyn FeatureImpl>> {
//         self.state.into_iter().filter_map(|f| f).collect()
//     }

//     #[inline]
//     pub fn contains(&self, feature: u32) -> bool {
//         self.enabled_features & feature > 0
//     }

//     pub fn get(&self, flag: u32) -> Option<&Box<dyn FeatureImpl>> {
//         if flag & self.enabled_features > 0 {
//             let index = flag.ilog2() as usize;
//             Some(self.state[index].as_ref().unwrap())
//         } else {
//             None
//         }
//     }
// }
