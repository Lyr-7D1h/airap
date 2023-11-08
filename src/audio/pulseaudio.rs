use log::{debug, warn};
use pulse::callbacks::ListResult;
use pulse::context::introspect::{self, Introspector, SourceInfo};
use pulse::context::{Context, FlagSet as ContextFlagSet};
use pulse::def::{BufferAttr, Retval};
use pulse::mainloop::standard::IterateResult;
use pulse::mainloop::standard::Mainloop;
use pulse::operation::{self, Operation, State};
use pulse::proplist::Proplist;
use pulse::sample::{Format, Spec};
use pulse::stream::{self, FlagSet as StreamFlagSet, PeekResult, Stream};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use crate::error::AirapError;
use crate::{Instant, Latency, RawEvent};

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub spec: Spec,
    pub monitor_of_sink_name: Option<String>,
}

impl<'a> From<&pulse::context::introspect::SourceInfo<'a>> for Device {
    fn from(v: &pulse::context::introspect::SourceInfo) -> Self {
        let mut spec = v.sample_spec;
        spec.channels = 1; // TODO make multi channel
        spec.format = Format::F32le;
        assert!(spec.rate_is_valid());
        assert!(spec.format_is_valid());
        Self {
            name: v.name.clone().map(|n| n.to_string()).unwrap_or("".into()),
            spec,
            monitor_of_sink_name: v.monitor_of_sink_name.clone().map(|n| n.to_string()),
        }
    }
}

/// return pulse audio sources
fn devices<'n>(
    mainloop: &Rc<RefCell<Mainloop>>,
    introspector: &Introspector,
) -> Result<Vec<Device>, AirapError> {
    let sources = Rc::new(RefCell::new(vec![]));
    let sources_clone = sources.clone();
    let op = introspector.get_source_info_list(move |lr| {
        if let ListResult::Item(i) = lr {
            sources_clone.borrow_mut().push(Device::from(i));
        }
    });
    wait_for_operation(mainloop, op)?;
    Ok(Rc::try_unwrap(sources).unwrap().into_inner())
}

impl Device {
    pub fn default() -> Result<Device, AirapError> {
        let mainloop = Rc::new(RefCell::new(
            Mainloop::new().expect("Failed to create mainloop"),
        ));
        let context = get_context(&mainloop)?;

        // get default sink name
        let introspector = context.borrow_mut().introspect();
        let default_sink_name = Rc::new(RefCell::new(None));
        let default_sink_name_ref = default_sink_name.clone();
        let op = introspector.get_server_info(move |info| {
            let name = info.default_sink_name.as_ref().map(|n| n.to_string());
            *default_sink_name_ref.borrow_mut() = name;
        });
        wait_for_operation(&mainloop, op)?;

        let sources = devices(&mainloop, &introspector)?;

        let default_source = if let Some(default_sink_name) = default_sink_name.borrow().clone() {
            sources
                .into_iter()
                .find(|s| {
                    if let Some(mos) = &s.monitor_of_sink_name {
                        *mos == default_sink_name
                    } else {
                        false
                    }
                })
                .ok_or(AirapError::audio("could not find monitor of default sink"))?
        } else {
            sources
                .into_iter()
                .nth(0)
                .ok_or(AirapError::audio("no sources found"))?
        };

        Ok(default_source)
    }
}

fn wait_for_operation<G: ?Sized>(
    mainloop: &Rc<RefCell<Mainloop>>,
    op: Operation<G>,
) -> Result<(), AirapError> {
    loop {
        iterate_mainloop(mainloop)?;
        match op.get_state() {
            State::Done => break,
            State::Running => {}
            State::Cancelled => return Err(AirapError::audio("Operation cancelled")),
        }
    }
    Ok(())
}

fn iterate_mainloop(mainloop: &Rc<RefCell<Mainloop>>) -> Result<(), AirapError> {
    match mainloop.borrow_mut().iterate(false) {
        IterateResult::Success(_) => return Ok(()),
        IterateResult::Err(e) => return Err(e.into()),
        IterateResult::Quit(_) => {
            return Err(AirapError::audio("Operation failed: mainloop quiting"))
        }
    }
}

/// Get context with properties and wait for it to be ready
fn get_context(mainloop: &Rc<RefCell<Mainloop>>) -> Result<Rc<RefCell<Context>>, AirapError> {
    let mut proplist = Proplist::new().unwrap();
    proplist
        .set_str(pulse::proplist::properties::APPLICATION_NAME, "airap")
        .unwrap();
    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), "Airap", &proplist)
            .ok_or(AirapError::audio("Failed to create new context"))?,
    ));
    context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|_| AirapError::audio("Failed to create new context"))?;
    // Wait for context to be ready
    loop {
        iterate_mainloop(&mainloop)?;
        match context.borrow().get_state() {
            pulse::context::State::Ready => {
                break;
            }
            pulse::context::State::Failed | pulse::context::State::Terminated => {
                return Err(AirapError::audio("context failed"));
            }
            _ => {}
        }
    }
    return Ok(context);
}

pub fn raw<'a, F>(device: &Device, cb: F) -> Result<(), AirapError>
where
    F: Fn(RawEvent<'a>),
{
    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("Failed to create mainloop"),
    ));

    let context = get_context(&mainloop)?;

    let stream = Rc::new(RefCell::new(
        Stream::new(
            &mut context.borrow_mut(),
            "Desktop Audio",
            &device.spec,
            None,
        )
        .expect("Failed to create new stream"),
    ));

    // println!("{}", (default_source_spec.borrow().rate * 4) / 1000 * 5);
    let max_length = device.spec.usec_to_bytes(pulse::time::MicroSeconds(5000)) as u32;
    let buff_attr = BufferAttr {
        maxlength: max_length * 4, // absolute max of 20ms
        tlength: 0,                // playback only
        prebuf: 0,                 // playback only
        minreq: 0,                 // playback only
        fragsize: max_length,
    };
    stream
        .borrow_mut()
        .connect_record(
            Some(&device.name),
            Some(&buff_attr),
            StreamFlagSet::DONT_MOVE | StreamFlagSet::ADJUST_LATENCY | StreamFlagSet::START_UNMUTED,
        )
        .expect("Failed to connect record");

    // Wait for stream to be ready
    loop {
        iterate_mainloop(&mainloop)?;
        match stream.borrow().get_state() {
            stream::State::Ready => break,
            stream::State::Failed | stream::State::Terminated => {
                return Err(AirapError::audio("stream state failed"))
            }
            _ => {}
        }
    }

    let mut stream = stream.borrow_mut();
    debug!(
        "stream listening to '{:?}' with spec '{:?}'",
        stream.get_device_name().unwrap_or("".into()),
        stream.get_sample_spec()
    );

    stream.set_overflow_callback(Some(Box::new(|| warn!("buffer overflow"))));
    stream.set_underflow_callback(Some(Box::new(|| warn!("buffer underflow"))));
    debug!("Buffer size: '{:?}'", stream.get_buffer_attr());

    stream.update_timing_info(None);
    loop {
        iterate_mainloop(&mainloop)?;
        if let Some(size) = stream.readable_size() {
            if size > 0 {
                while let PeekResult::Data(bytes) = stream.peek()? {
                    let internal_latency = stream.get_latency()?;
                    // println!("{latency:?}");
                    // println!("{:?}", stream.get_timing_info());
                    stream.update_timing_info(None);
                    // println!("{}", bytes.len());
                    let (prefix, data, suffix) = unsafe { bytes.align_to::<f32>() };
                    // println!("{:?}", data);
                    assert!(prefix.len() == 0);
                    assert!(suffix.len() == 0);

                    cb(RawEvent {
                        data,
                        latency: Latency {
                            internal: internal_latency.into(),
                            airap: Instant::None,
                        },
                    });
                    // println!("{:?}", data);
                    stream.discard()?;
                }
            }
        }

        // TODO const volume and unmute
        // if stream.is_corked().unwrap() {
        //     stream.uncork(None);
        // }
    }
}
