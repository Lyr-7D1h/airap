use log::{debug, warn};
use pulse::callbacks::ListResult;
use pulse::context::introspect::SinkInfo;
use pulse::context::{Context, FlagSet as ContextFlagSet};
use pulse::def::{BufferAttr, Retval};
use pulse::mainloop::standard::IterateResult;
use pulse::mainloop::standard::Mainloop;
use pulse::operation::{self, Operation, State};
use pulse::proplist::Proplist;
use pulse::sample::{Format, Spec};
use pulse::stream::{self, FlagSet as StreamFlagSet, PeekResult, Stream};
use std::cell::RefCell;
use std::error::Error;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

use crate::error::AirapError;

pub struct PulseAudio {}

impl PulseAudio {
    pub fn new() -> PulseAudio {
        PulseAudio {}
    }

    pub fn on_raw<F>(&mut self, cb: F)
    where
        F: Fn(&[f32]) + Send + 'static,
    {
        thread::Builder::new()
            .name("airap_pulseaudio_on_update".into())
            .spawn(|| on_update_worker(cb).unwrap())
            .unwrap();
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

fn on_update_worker<F>(cb: F) -> Result<(), AirapError>
where
    F: Fn(&[f32]),
{
    let mut proplist = Proplist::new().unwrap();
    proplist
        .set_str(pulse::proplist::properties::APPLICATION_NAME, "airap")
        .unwrap();

    let mainloop = Rc::new(RefCell::new(
        Mainloop::new().expect("Failed to create mainloop"),
    ));

    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(mainloop.borrow().deref(), "Airap", &proplist)
            .expect("Failed to create new context"),
    ));

    context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

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

    // get default sink
    let introspector = context.borrow_mut().introspect();
    let default_sink_name = Rc::new(RefCell::new(None));
    let default_sink_name_ref = default_sink_name.clone();
    let op = introspector.get_server_info(move |info| {
        let name = info.default_sink_name.as_ref().map(|n| n.to_string());
        *default_sink_name_ref.borrow_mut() = name;
    });
    wait_for_operation(&mainloop, op)?;

    // get the name of the monitor for sink
    let default_source_name = Rc::new(RefCell::new(None));
    let default_source_spec = Rc::new(RefCell::new(Spec {
        format: Format::F32le,
        channels: 1, // TODO make multi channel
        rate: 44100,
    }));
    if let Some(default_sink_name) = Rc::try_unwrap(default_sink_name).unwrap().into_inner() {
        let default_sink_name_ref = default_source_name.clone();
        let default_source_spec_ref = default_source_spec.clone();
        let op = introspector.get_source_info_list(move |lr| {
            if let ListResult::Item(i) = lr {
                if let Some(name) = &i.monitor_of_sink_name {
                    if name.to_string() == *default_sink_name {
                        *default_sink_name_ref.borrow_mut() =
                            i.name.as_ref().map(|n| n.to_string());
                        default_source_spec_ref.borrow_mut().as_mut().rate = i.sample_spec.rate;
                        // TODO make sure name exists otherwise return error in callback
                        // match i.name {
                        //     Some(name) => {
                        //     }
                        //     None => {
                        //         return Err(AirapError::audio(
                        //             "Found default source does not have a name",
                        //         ))
                        //     }
                        // }
                    }
                }
            }
        });
        wait_for_operation(&mainloop, op)?;
    }

    let default_source_name = default_source_name.borrow();
    let default_source_name = default_source_name.as_ref().map(|n| n.as_str());

    let spec = &default_source_spec.borrow();

    let stream = Rc::new(RefCell::new(
        Stream::new(&mut context.borrow_mut(), "Desktop Audio", &spec, None)
            .expect("Failed to create new stream"),
    ));

    // let buff_attr = BufferAttr { maxlength: 4194304, tlength: 96000, prebuf: 4294967295, minreq: 4294967295, fragsize: 768000 }

    // half the latency
    let buff_attr = BufferAttr {
        maxlength: u16::MAX as u32 / 10,
        tlength: 96000,
        prebuf: u32::MAX,
        minreq: u32::MAX,
        fragsize: u32::MAX,
    };
    stream
        .borrow_mut()
        .connect_record(
            default_source_name,
            // None,
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
        "stream listening to {:?} with spec {:?}",
        stream.get_device_name().unwrap_or("".into()),
        stream.get_sample_spec()
    );

    stream.set_overflow_callback(Some(Box::new(|| warn!("buffer overflow"))));
    stream.set_underflow_callback(Some(Box::new(|| warn!("buffer underflow"))));
    println!("{:?}", stream.get_buffer_attr());

    stream.update_timing_info(None);
    loop {
        iterate_mainloop(&mainloop)?;
        if let Some(size) = stream.readable_size() {
            if size > 0 {
                // println!("{:?}", stream.get_latency());
                // println!("{:?}", stream.get_timing_info());
                stream.update_timing_info(None);

                // TODO parse format
                while let PeekResult::Data(bytes) = stream.peek()? {
                    // println!("{}", bytes.len());
                    let (prefix, data, suffix) = unsafe { bytes.align_to::<f32>() };
                    assert!(prefix.len() == 0);
                    assert!(suffix.len() == 0);

                    cb(data);
                    // println!("{:?}", data);
                    stream.discard()?;
                }
            }
        }

        // TODO fix volume and unmute
        if stream.is_corked().unwrap() {
            stream.uncork(None);
        }
    }
}
