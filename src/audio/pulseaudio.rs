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

        let default_port_name = Rc::new(RefCell::new(None));
        let name_ref = default_port_name.clone();
        let op = introspector.get_sink_info_list(move |sink_list| {
            if let ListResult::Item(item) = sink_list {
                if let Some(port) = &item.active_port {
                    if let Some(name) = &port.name {
                        *name_ref.borrow_mut() = Some(name.to_string().as_str());
                    }
                }
                println!("{item:?}");
            }
        });
        self.wait_for_operation(op)?;

        self.stream
            .borrow_mut()
            .connect_record(
                default_port_name.borrow().clone(),
                None,
                StreamFlagSet::NOFLAGS,
            )
            .expect("Failed to connect record");

        debug!("Listening on {:?}", self.stream.borrow().get_device_name());

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

        // Our main logic (to output a stream of audio data)
        let drained = Rc::new(RefCell::new(false));
        loop {
            // match self.mainloop.borrow_mut().iterate(false) {
            //     IterateResult::Quit(_) | IterateResult::Err(_) => {
            //         eprintln!("Iterate state was not success, quitting...");
            //         return;
            //     }
            //     IterateResult::Success(_) => {}
            // }

            // // Write some data with stream.write()
            // //

            // if self.stream.borrow().is_corked().unwrap() {
            //     self.stream.borrow_mut().uncork(None);
            // }

            // // Wait for our data to be played
            // let _o = {
            //     let drain_state_ref = Rc::clone(&drained);
            //     self.stream
            //         .borrow_mut()
            //         .drain(Some(Box::new(move |_success: bool| {
            //             *drain_state_ref.borrow_mut() = true;
            //         })))
            // };
            // while *drained.borrow_mut() == false {
            //     match self.mainloop.borrow_mut().iterate(false) {
            //         IterateResult::Quit(_) | IterateResult::Err(_) => {
            //             eprintln!("Iterate state was not success, quitting...");
            //             return;
            //         }
            //         IterateResult::Success(_) => {}
            //     }
            // }
            // *drained.borrow_mut() = false;

            // Remember to break out of the loop once done writing all data (or whatever).
        }
    }
}
