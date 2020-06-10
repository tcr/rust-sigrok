use crate::util::AutoDrop;
use crate::{device, Device, Driver, DriverContext, Sigrok, SigrokError, TriggerType, Unit};

use data::*;
use futures::channel::oneshot::{channel, Sender};
use futures::{select_biased, FutureExt};
use glib::{MainContext, MainLoop};
use num_rational::Ratio;
use sigrok_sys::{
    sr_datafeed_analog, sr_datafeed_header, sr_datafeed_logic, sr_datafeed_packet, sr_dev_inst,
    sr_dev_inst_driver_get, sr_dev_open, sr_packettype, sr_session,
    sr_session_datafeed_callback_add, sr_session_datafeed_callback_remove_all, sr_session_destroy,
    sr_session_dev_add, sr_session_new, sr_session_run, sr_session_start, sr_session_stop,
    sr_session_stopped_callback_set, sr_session_trigger_set, sr_trigger, sr_trigger_free,
    sr_trigger_match_add, sr_trigger_new, sr_trigger_stage, sr_trigger_stage_add,
};
use std::borrow::Borrow;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::os::raw::c_void;
use std::ptr::null_mut;
use std::time::Duration;
use std::{ptr, slice};

pub mod data {
    //! Data values
    pub use crate::enums::{Mq, MqFlags, MqType};
    use crate::Unit;
    use num_rational::Ratio;
    use std::time::Duration;

    /// A header with information about each session run.
    pub struct Header {
        pub feed_version: i32,
        pub start_time: Duration,
    }

    /// A logic capture.
    pub struct Logic<'a> {
        /// The number of bytes in each sample. So if `unit_size` is 4 and `data.len() == 16`, then
        /// there are 4 samples.
        pub unit_size: u16,
        pub data: &'a [u8],
    }

    bitflags::bitflags! {
        pub struct AnalogFlags: u8 {
            const SIGNED = 1;
            const FLOATING_POINT = 2;
            const BIG_ENDIAN = 4;
            const DECIMAL_DIGITS = 8;
        }
    }

    /// An analog capture.
    pub struct Analog<'a> {
        /// The number of bytes in each sample. So if `unit_size` is 4 and `data.len() == 16`, then
        /// there are 4 samples.
        pub unit_size: u8,
        pub data: &'a [u8],
        pub mq: Mq,
        pub scale: Ratio<i64>,
        pub offset: Ratio<i64>,
        pub channels: (),
        pub flags: AnalogFlags,
        /// Number of significant digits after the decimal point if positive, or number of
        /// non-significant digits before the decimal point if negative (refers to the value we
        /// actually read on the wire).
        pub digits: i8,
        pub unit: Unit,
    }

    /// A feed of data from the session.
    pub enum Datafeed<'a> {
        Header(Header),
        Logic(Logic<'a>),
        Analog(Analog<'a>),
        /// The trigger matched at this point in the data feed. For some reason, it doesn't tell
        /// you *which* trigger stage triggered this.
        Trigger,
        /// Beginning of frame
        FrameBegin,
        /// End of frame
        FrameEnd,
        /// End of stream
        End,
    }
}

/// A specific trigger.
///
/// Internally, this is known as a "trigger match" in Sigrok. It describes what type of event to
/// match.
pub struct Trigger<'a> {
    /// The channel that this trigger should wait for.
    pub channel: device::Channel<'a>,
    /// The state of the channel that should trigger this Trigger.
    pub trigger_match: TriggerType,
    /// If the trigger match is one of [`Over`][TriggerType::Over] or
    /// [`Under`][TriggerType::Under], this is the value to compare against.
    ///
    /// This is used for analog channels to trigger when the voltage goes above
    /// ([`Over`][TriggerType::Over]) or below ([`Under`][TriggerType::Under]) a specific voltage.
    pub value: f32,
}

/// A list of trigger stages.
///
/// In Sigrok, triggers are defined as a list of *trigger stages*. There may be
/// multiple trigger stages in one [`Session`]. All [`Trigger`]s within one trigger stage must occur
/// at the same time for that trigger stage to activate. For example, if you have a waveform that
/// looks like:
///
/// ```text
/// D1:                         🠗
/// ┌───┐   ┌───┐   ┌───┐   ┌───┐   ┌───┐   ┌───┐   ┌──
///     └───┘   └───┘   └───┘   └───┘   └───┘   └───┘
/// D2:                         ↕
///             ┌───────┐       ┌──────────────────────
/// ────────────┘       └───────┘
///                             🠕
/// ```
/// Setting a [`TriggerType::Rising`] for both D1 and D2 would only trigger at the indicated point,
/// because although both lines rose at some point, they did not do so at the same time.
///
/// For example:
/// ```
/// use sigrok::{Sigrok, Session, Trigger, Triggers, TriggerType};
/// use sigrok::data::{Datafeed, Logic};
///
/// # fn main() -> Result<(), sigrok::SigrokError> {
/// let ctx = Sigrok::new()?;
/// let sess = Session::new(&ctx)?;
/// let demo_driver = ctx.drivers().iter().find(|x| x.name() == "demo").unwrap().init()?;
/// let mut triggers = vec![];
/// let device = &demo_driver.scan(None)?[0];
/// for c in device
///     .channels()
///     .into_iter()
///     .filter(|c| c.name() == "D0" || c.name() == "D1")
/// {
///     triggers.push(Trigger {
///         channel: c.clone(),
///         trigger_match: TriggerType::Falling,
///         value: 0.0,
///     });
/// }
/// sess.add_device(device)?;
/// let mut triggered = false;
/// sess.start(
///     Some(&Triggers::new(&mut [triggers.iter()]).unwrap()),
///     |_, data| match data {
///         Datafeed::Logic(Logic { data, .. }) => {
///             for byte in data {
///                 println!(
///                     "{}",
///                     format!("{:08b}", byte).replace("1", " ").replace("0", "█")
///                 );
///             }
///         }
///         Datafeed::Trigger => {
///             assert!(!triggered, "Received multiple triggers!");
///             triggered = true;
///         },
///         _ => {}
///     },
/// )?;
/// # Ok(())
/// # }
/// ```
pub struct Triggers<'a>(AutoDrop<sr_trigger>, PhantomData<Trigger<'a>>);

/// Create new triggers.
impl<'a> Triggers<'a> {
    pub fn new<F: IntoIterator<Item = T>, T: Borrow<Trigger<'a>>>(
        trigger_stages: impl IntoIterator<Item = F>,
    ) -> Result<Self, SigrokError> {
        // https://sourceforge.net/p/sigrok/mailman/message/36950036/
        unsafe {
            let mut raw_trigger =
                AutoDrop::new(sr_trigger_new(ptr::null()), |tr| sr_trigger_free(tr))?;
            trigger_stages.into_iter().for_each(|trigger_stage: F| {
                let raw_trigger_stage = sr_trigger_stage_add(&mut *raw_trigger);
                trigger_stage.into_iter().for_each(|trigger: T| {
                    let trigger = trigger.borrow();
                    sr_trigger_match_add(
                        raw_trigger_stage,
                        trigger.channel.context,
                        trigger.trigger_match.into(),
                        trigger.value,
                    );
                })
            });
            if (*raw_trigger).stages.is_null()
                || (*((*raw_trigger.stages).data as *mut sr_trigger_stage))
                    .matches
                    .is_null()
            {
                Ok(Triggers(AutoDrop::null(), PhantomData))
            } else {
                Ok(Triggers(raw_trigger, PhantomData))
            }
        }
    }
}

unsafe extern "C" fn sr_session_callback(
    inst: *const sr_dev_inst,
    packet: *const sr_datafeed_packet,
    data: *mut c_void,
) {
    // See session.c in sigrok-cli line 186
    let kind = (*packet).type_;

    let data: &mut SessionData = &mut *(data as *mut SessionData);
    let instance_driver = sr_dev_inst_driver_get(inst);
    if instance_driver.is_null() {
        panic!("Failed to get instance driver!");
    }
    let raw_driver = Driver {
        context: instance_driver,
        sigrok: data.sigrok,
    };
    let driver_context = ManuallyDrop::new(DriverContext(raw_driver));
    let driver = Device {
        context: inst as *mut _,
        driver: &driver_context,
    };
    let mut cb = |feed| (data.callback)(&driver, feed);
    if kind == (sr_packettype::SR_DF_HEADER as u16) {
        let header = (*packet).payload as *const sr_datafeed_header;

        cb(Datafeed::Header(Header {
            feed_version: (*header).feed_version as i32,
            start_time: Duration::new(
                (*header).starttime.tv_sec as u64,
                ((*header).starttime.tv_usec as u32) * 1000,
            ),
        }));
    } else if kind == (sr_packettype::SR_DF_LOGIC as u16) {
        let logic = (*packet).payload as *const sr_datafeed_logic;
        let parts = slice::from_raw_parts((*logic).data as *const u8, (*logic).length as usize);

        cb(Datafeed::Logic(Logic {
            unit_size: (*logic).unitsize,
            data: parts,
        }));
    } else if kind == (sr_packettype::SR_DF_ANALOG as u16) {
        let analog = (*packet).payload as *const sr_datafeed_analog;
        let encoding = *(*analog).encoding;
        let meaning = *(*analog).meaning;
        let unit_size = encoding.unitsize;

        let mut flags: AnalogFlags = AnalogFlags::empty();
        flags.set(AnalogFlags::SIGNED, encoding.is_signed != 0);
        flags.set(AnalogFlags::FLOATING_POINT, encoding.is_float != 0);
        flags.set(AnalogFlags::BIG_ENDIAN, encoding.is_bigendian != 0);
        flags.set(AnalogFlags::DECIMAL_DIGITS, encoding.is_digits_decimal != 0);

        cb(Datafeed::Analog(Analog {
            unit_size,
            data: slice::from_raw_parts(
                (*analog).data as *const u8,
                (*analog).num_samples as usize * unit_size as usize,
            ),
            mq: Mq {
                mq_type: (meaning.mq as u32).try_into().unwrap_or(MqType::Voltage),
                flags: MqFlags::from_bits_truncate(meaning.mqflags.0),
            },
            scale: Ratio::new_raw(encoding.scale.p, encoding.scale.q as i64),
            offset: Ratio::new_raw(encoding.offset.p, encoding.offset.q as i64),
            channels: (),
            flags,
            digits: encoding.digits,
            unit: (meaning.unit as u32).try_into().unwrap_or(Unit::Volt),
        }));
    } else if kind == (sr_packettype::SR_DF_END as u16) {
        cb(Datafeed::End);
    } else if kind == (sr_packettype::SR_DF_META as u16) {
        println!("TODO: meta");
    } else if kind == (sr_packettype::SR_DF_TRIGGER as u16) {
        cb(Datafeed::Trigger);
    } else if kind == (sr_packettype::SR_DF_FRAME_BEGIN as u16) {
        cb(Datafeed::FrameBegin);
    } else if kind == (sr_packettype::SR_DF_FRAME_END as u16) {
        cb(Datafeed::FrameEnd);
    }
}

unsafe extern "C" fn quit_loop(main_loop: *mut c_void) {
    let main_loop: &mut Option<Sender<()>> = &mut *(main_loop as *mut Option<Sender<()>>);
    if let Some(cancel) = main_loop.take() {
        let _ = cancel.send(());
    }
}

struct SessionData<'a> {
    callback: Box<dyn FnMut(&Device, Datafeed) + 'a>,
    sigrok: &'a Sigrok,
}

/// A Sigrok session that handles
pub struct Session<'a> {
    context: *mut sr_session,
    sigrok: &'a Sigrok,
}

impl<'a> Session<'a> {
    /// Create a new session.
    pub fn new(ctx: &Sigrok) -> Result<Session, SigrokError> {
        unsafe {
            let mut session = Session {
                context: null_mut(),
                sigrok: ctx,
            };
            SigrokError::from(sr_session_new(ctx.context, &mut session.context)).map(|_| session)
        }
    }

    /// Add and initialize a device.
    pub fn add_device(&self, instance: &Device) -> Result<(), SigrokError> {
        unsafe {
            match SigrokError::from(sr_dev_open(instance.context)) {
                Ok(()) => Ok(()),
                // "If the device instance is already open (sdi->status == SR_ST_ACTIVE),
                // SR_ERR will be returned and no re-opening of the device will be attempted."
                Err(SigrokError::Err) => Ok(()),
                e => e,
            }?;
            SigrokError::from(sr_session_dev_add(self.context, instance.context))
        }
    }

    /// Stop acquiring data. This is only useful if you'd like to cancel data acquisition from the
    /// callback of [`start`][Self::start]. Otherwise, use
    /// [`start_with_cancel`][Self::start_with_cancel] to cancel from another thread.
    pub fn stop(&self) -> Result<(), SigrokError> {
        unsafe { SigrokError::from(sr_session_stop(self.context)) }
    }

    fn set_triggers(&self, triggers: Option<&Triggers>) -> Result<(), SigrokError> {
        unsafe {
            if let Some(triggers) = triggers {
                SigrokError::from(sr_session_trigger_set(
                    self.context,
                    // I *think* that it doesn't modify the triggers
                    &*triggers.0 as *const sr_trigger as *mut sr_trigger,
                ))
            } else {
                SigrokError::from(sr_session_trigger_set(self.context, null_mut()))
            }
        }
    }

    /// Start acquiring data. This function will block until it is complete. Use
    /// [`start_with_cancel`][Self::start_with_cancel] if you'd like to be able to cancel data
    /// acquisition from another thread.
    pub fn start(
        &self,
        triggers: Option<&Triggers>,
        cb: impl FnMut(&Device, Datafeed),
    ) -> Result<(), SigrokError> {
        self.set_triggers(triggers)?;
        let mut data = SessionData {
            callback: Box::new(cb),
            sigrok: self.sigrok,
        };
        unsafe {
            SigrokError::from(sr_session_datafeed_callback_add(
                self.context,
                Some(sr_session_callback),
                &mut data as *mut SessionData as *mut c_void,
            ))?;
            SigrokError::from(sr_session_stopped_callback_set(
                self.context,
                None,
                null_mut(),
            ))?;
            SigrokError::from(sr_session_start(self.context))?;
            SigrokError::from(sr_session_run(self.context))?;
            SigrokError::from(sr_session_datafeed_callback_remove_all(self.context))
        }
    }

    /// Start acquiring data with the ability to cancel it later. This function will block until it
    /// is complete.
    ///
    /// You may pass a function into `context` which will receive a
    /// [`Sender`] as an argument. You may send that to another thread and call `.send(())`, or
    /// call `.is_canceled()` to see if the event loop finished. Attempting to cancel after the
    /// [`Session`] was dropped will not cause undefined behavior, it will merely return an [`Err`].
    pub fn start_with_cancel(
        &self,
        triggers: Option<&Triggers>,
        context: impl FnOnce(Sender<()>),
        cb: impl FnMut(&Device, Datafeed),
    ) -> Result<(), SigrokError> {
        self.set_triggers(triggers)?;
        let mut data = SessionData {
            callback: Box::new(cb),
            sigrok: self.sigrok,
        };
        unsafe {
            SigrokError::from(sr_session_datafeed_callback_add(
                self.context,
                Some(sr_session_callback),
                &mut data as *mut SessionData as *mut c_void,
            ))?;
            SigrokError::from(sr_session_start(self.context))?;
            let main_context = MainContext::default();
            let (end_sender, end_receiver) = channel::<()>();
            let mut end_sender = Some(end_sender);
            let main_loop = MainLoop::new(Some(&main_context), false);
            let main_loop_cancel = main_loop.clone();
            if !main_context.acquire() {
                return Err(SigrokError::GlibAcquireError);
            }
            let (cancel, cancel_recv) = channel();
            context(cancel);
            let ctx = self.context;
            main_context.spawn_local(async move {
                let mut end_receiver = end_receiver.fuse();
                let keep_waiting = select_biased! {
                    _ = end_receiver => false,
                    res = cancel_recv.fuse() => {
                        match res {
                            Ok(()) => { unsafe { sr_session_stop(ctx); } },
                            _ => {},
                        }
                        true
                    }
                };
                // If we received a signal to cancel, wait until Sigrok tells us it's ok to
                // stop the event loop
                // This stops a memory leak where this function stops getting polled
                if keep_waiting {
                    let _ = end_receiver.await;
                } else {
                    drop(end_receiver);
                }
                main_loop_cancel.quit();
            });
            SigrokError::from(sr_session_stopped_callback_set(
                self.context,
                Some(quit_loop),
                &mut end_sender as *mut Option<Sender<()>> as *mut c_void,
            ))?;
            main_loop.run();
            // No clue if this is necessary or if it happens on Drop but let's do it anyways
            main_context.release();
            SigrokError::from(sr_session_datafeed_callback_remove_all(self.context))
        }
    }
}

impl<'a> Drop for Session<'a> {
    fn drop(&mut self) {
        unsafe {
            SigrokError::from(sr_session_destroy(self.context))
                .expect("Failed on sigrok session destructor");
        }
    }
}
