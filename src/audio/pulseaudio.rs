use log::debug;
use pulse::callbacks::ListResult;
use pulse::context::introspect::SinkInfo;
use pulse::context::{Context, FlagSet as ContextFlagSet};
use pulse::def::Retval;
use pulse::mainloop::standard::IterateResult;
use pulse::mainloop::standard::Mainloop;
use pulse::operation::{self, Operation, State};
use pulse::proplist::Proplist;
use pulse::sample::{Format, Spec};
use pulse::stream::{self, FlagSet as StreamFlagSet, Stream};
use std::cell::RefCell;
use std::error::Error;
use std::ops::Deref;
use std::rc::Rc;

use crate::error::AirapError;

use super::Audio;

pub struct PulseAudio {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
    stream: Rc<RefCell<Stream>>,
}

impl PulseAudio {
    pub fn new() -> PulseAudio {
        let spec = Spec {
            format: Format::S16NE,
            channels: 2,
            rate: 44100,
        };
        let mut proplist = Proplist::new().unwrap();
        proplist
            .set_str(pulse::proplist::properties::APPLICATION_NAME, "airap")
            .unwrap();

        let mainloop = Rc::new(RefCell::new(
            Mainloop::new().expect("Failed to create mainloop"),
        ));

        let context = Rc::new(RefCell::new(
            Context::new_with_proplist(mainloop.borrow().deref(), "AirapContext", &proplist)
                .expect("Failed to create new context"),
        ));

        context
            .borrow_mut()
            .connect(None, ContextFlagSet::NOFLAGS, None)
            .expect("Failed to connect context");

        let stream = Rc::new(RefCell::new(
            Stream::new(&mut context.borrow_mut(), "Airap", &spec, None)
                .expect("Failed to create new stream"),
        ));

        PulseAudio {
            mainloop,
            context,
            stream,
        }
    }

    pub fn iterate_mainloop(&self) -> Result<(), AirapError> {
        match self.mainloop.borrow_mut().iterate(false) {
            IterateResult::Success(_) => return Ok(()),
            IterateResult::Err(e) => return Err(e.into()),
            IterateResult::Quit(_) => {
                return Err(AirapError::audio("Operation failed: mainloop quiting"))
            }
        }
    }

    pub fn wait_for_operation<G: ?Sized>(&mut self, op: Operation<G>) -> Result<(), AirapError> {
        loop {
            self.iterate_mainloop()?;
            match op.get_state() {
                State::Done => break,
                State::Running => {}
                State::Cancelled => return Err(AirapError::audio("Operation cancelled")),
            }
        }
        Ok(())
    }
}

impl Drop for PulseAudio {
    fn drop(&mut self) {
        self.mainloop.borrow_mut().quit(Retval(0));
        // self.stream.borrow_mut().disconnect().unwrap();
    }
}

impl Audio for PulseAudio {
    fn on_update(&mut self, op: fn(u16)) -> Result<(), AirapError> {
        // Wait for context to be ready
        loop {
            self.iterate_mainloop()?;
            match self.context.borrow().get_state() {
                pulse::context::State::Ready => {
                    break;
                }
                pulse::context::State::Failed | pulse::context::State::Terminated => {
                    return Err(AirapError::audio("context failed"));
                }
                _ => {}
            }
        }

        let introspector = self.context.borrow_mut().introspect();
        let default_sink_name = Rc::new(RefCell::new(None));
        let default_sink_name_ref = default_sink_name.clone();
        let op = introspector.get_server_info(move |info| {
            let name = info.default_sink_name.as_ref().map(|n| n.to_string());
            *default_sink_name_ref.borrow_mut() = name;
        });
        self.wait_for_operation(op)?;

        let default_source_name = Rc::new(RefCell::new(None));
        if let Some(default_sink_name) = Rc::try_unwrap(default_sink_name).unwrap().into_inner() {
            let default_sink_name_ref = default_source_name.clone();
            let op = introspector.get_source_info_list(move |lr| {
                if let ListResult::Item(i) = lr {
                    if let Some(name) = &i.monitor_of_sink_name {
                        if name.to_string() == *default_sink_name {
                            *default_sink_name_ref.borrow_mut() =
                                i.name.as_ref().map(|n| n.to_string())
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
            self.wait_for_operation(op)?;
        }

        let default_source_name = default_source_name.borrow();
        let default_source_name = default_source_name.as_ref().map(|n| n.as_str());

        debug!("Listening to {:?}", default_source_name);
        self.stream
            .borrow_mut()
            .connect_record(default_source_name, None, StreamFlagSet::NOFLAGS)
            .expect("Failed to connect record");

        // Wait for stream to be ready
        loop {
            self.iterate_mainloop()?;
            match self.stream.borrow().get_state() {
                stream::State::Ready => break,
                stream::State::Failed | stream::State::Terminated => {
                    return Err(AirapError::audio("stream state failed"))
                }
                _ => {}
            }
        }

        debug!("Reading from stream");
        let mut stream = self.stream.borrow_mut();
        loop {
            self.iterate_mainloop()?;
            if let Some(size) = stream.readable_size() {
                if size > 0 {
                    let read = stream.peek()?;
                    println!("{:?}", read);
                    stream.discard()?;
                }
            }

            if stream.is_corked().unwrap() {
                stream.uncork(None);
            }
        }
    }
}
