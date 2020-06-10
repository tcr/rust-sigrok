//! # Sigrok
//!
//! This crate provides high-level, memory-safe bindings to Sigrok. Start by creating an instance
//! of [`Sigrok`], the main struct in this crate. From there, you may [list][Sigrok::drivers] the
//! available [`Driver`]s.
//!
//! ```
//! use sigrok::config::{config_items, Configurable};
//! use sigrok::data::{Datafeed, Logic};
//! use sigrok::{Session, Sigrok};
//! use std::error::Error;
//!
//! fn main() -> Result<(), Box<dyn Error>> {
//!     // Print out available drivers.
//!     let ctx = Sigrok::new()?;
//!
//!     let ses = Session::new(&ctx)?;
//!
//!     let driver = ctx
//!         .drivers()
//!         .into_iter()
//!         .find(|x| x.name() == "demo")
//!         .unwrap();
//!
//!     // Initialize driver.
//!     let driver = driver.init()?;
//!     // Scan for devices.
//!     for device in driver.scan(None)? {
//!         // Attach device.
//!         ses.add_device(&device)?;
//!         device.config_set(config_items::LimitSamples, &64)?;
//!
//!         // Set pattern mode on digital outputs.
//!         if let Some(group) = device.channel_groups().get(0) {
//!             group.config_set(config_items::PatternMode, "sigrok")?;
//!         }
//!
//!         // Set sample rate.
//!         device.config_set(config_items::SampleRate, &1_000_000)?;
//!     }
//!
//!     // Register callback, start session and loop endlessly.
//!     ses.start(None, |_, data| match data {
//!         Datafeed::Logic(Logic { unit_size, data }) => {
//!             let _ = unit_size;
//!             for byte in data {
//!                 println!(
//!                     "{}",
//!                     format!("{:08b}", byte).replace("1", " ").replace("0", "█")
//!                 );
//!             }
//!         }
//!         _ => {}
//!     })?;
//!
//!     Ok(())
//! }
//! ```
use std::borrow::Cow;
use std::ops::Deref;
use std::ptr::null_mut;

use glib_sys::{g_slist_free, GSList};
use sigrok_sys::{
    sr_config, sr_context, sr_dev_driver, sr_dev_inst, sr_dev_list, sr_driver_init, sr_driver_list,
    sr_driver_scan, sr_exit, sr_init,
};

pub use enums::{Function, TriggerType, Unit};
pub use error::SigrokError;
pub use session::*;

use crate::config::ScanOption;
use crate::util::get_functions;
use device::Device;
use glib::glib_sys::{g_malloc, g_slist_append, g_variant_new_string, g_variant_new_uint64};
use std::ffi::CString;
use std::mem::size_of;
use util::{c_str, gslist_iter, null_list_count};

#[macro_use]
mod util;
pub mod config;
pub mod device;
mod enums;
mod error;
pub mod log;
mod session;
#[cfg(test)]
mod test;

/// The main Sigrok instance.
#[derive(Debug)]
pub struct Sigrok {
    context: *mut sr_context,
}

impl Sigrok {
    /// Create a new Sigrok instance.
    pub fn new() -> Result<Sigrok, SigrokError> {
        unsafe {
            let mut ctx: Sigrok = Sigrok {
                context: null_mut(),
            };
            SigrokError::from(sr_init(&mut ctx.context))?;
            Ok(ctx)
        }
    }

    /// List all the drivers available.
    pub fn drivers(&self) -> Vec<Driver> {
        unsafe {
            let mut driver_list: *mut *mut sr_dev_driver = sr_driver_list(self.context);
            let mut drivers =
                Vec::with_capacity(null_list_count(driver_list as *const *const sr_dev_driver));
            while !(*driver_list).is_null() {
                drivers.push(Driver {
                    context: *driver_list,
                    sigrok: self,
                });
                driver_list = driver_list.add(1);
            }
            drivers
        }
    }
}

impl Drop for Sigrok {
    fn drop(&mut self) {
        unsafe {
            SigrokError::from(sr_exit(self.context)).expect("Failed on sigrok context destructor");
        }
    }
}

/// A driver, as obtained by [`Sigrok::drivers`].
#[derive(Debug, Clone)]
pub struct Driver<'a> {
    context: *mut sr_dev_driver,
    sigrok: &'a Sigrok,
}

impl<'a> Driver<'a> {
    /// Initialize the driver. It is an error to call this multiple times unless the returned
    /// [`DriverContext`] has been dropped. If that happens, this function will [panic][panic!].
    pub fn init<'b>(&'b self) -> Result<DriverContext<'a>, SigrokError> {
        unsafe {
            if !(*self.context).context.is_null() {
                panic!(
                    r#"Driver "{}" ({}) already initialized"#,
                    self.name(),
                    self.long_name()
                );
            }
            SigrokError::from(sr_driver_init(self.sigrok.context, self.context))?;
            Ok(DriverContext(self.clone()))
        }
    }

    /// The name of the driver.
    pub fn name<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str((*self.context).name) }
    }

    pub fn long_name<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str((*self.context).longname) }
    }

    pub fn api_version(&self) -> i32 {
        unsafe { (*self.context).api_version as i32 }
    }

    pub fn functions(&self) -> Result<Vec<Function>, SigrokError> {
        unsafe { get_functions(self.context, null_mut(), null_mut()) }
    }
}

/// An initialized driver, as obtained by [`Driver::init`].
#[derive(Debug)]
pub struct DriverContext<'a>(Driver<'a>);

impl<'a> Deref for DriverContext<'a> {
    type Target = Driver<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DriverContext<'a> {
    /// Scan for devices. You may optionally pass an iterator of [`ScanOption`]s to set the scan
    /// configuration. However, if you would like to skip that, you may simply pass [`None`].
    ///
    /// ```
    /// use sigrok::{Sigrok, Session};
    /// use sigrok::config::ScanOption;
    /// let ctx = Sigrok::new().unwrap();
    ///
    /// let ses = Session::new(&ctx).unwrap();
    ///
    /// let driver = ctx
    ///     .drivers()
    ///     .into_iter()
    ///     .find(|x| x.name() == "demo")
    ///     .unwrap();
    ///
    /// // Initialize driver.
    /// let driver = driver.init().unwrap();
    ///
    ///
    /// // No scan options
    /// let _devices = driver.scan(None).unwrap();
    ///
    /// // Some scan options
    /// let _devices = driver.scan(&[ScanOption::Connection("1d6b.0001")]).unwrap();
    /// ```
    pub fn scan<'b>(
        &self,
        scan_options: impl IntoIterator<Item = &'b ScanOption<'b>>,
    ) -> Result<Vec<Device>, SigrokError> {
        unsafe {
            let mut list = null_mut();
            scan_options
                .into_iter()
                .try_for_each::<_, Result<(), SigrokError>>(|opt: &ScanOption| {
                    let src = g_malloc(size_of::<sr_config>()) as *mut sr_config;
                    (*src).key = opt.into();
                    (*src).data = match opt {
                        ScanOption::Connection(string) | ScanOption::SerialComm(string) => {
                            let string = CString::new(string.as_bytes())?;
                            g_variant_new_string(string.as_ptr())
                        }
                        ScanOption::ModbusAddr(num) => g_variant_new_uint64(*num),
                    };
                    list = g_slist_append(list, src as *mut _);
                    Ok(())
                })?;
            let gslist = sr_driver_scan(self.0.context, list);
            Ok(self.enumerate_devices(gslist))
        }
    }

    /// List the already scanned devices.
    pub fn devices(&self) -> Vec<Device> {
        unsafe {
            let gslist = sr_dev_list(self.0.context);
            self.enumerate_devices(gslist)
        }
    }

    fn enumerate_devices(&self, gslist: *mut GSList) -> Vec<Device> {
        unsafe {
            let devices = gslist_iter(gslist)
                .map(|data| Device {
                    context: data as *mut sr_dev_inst,
                    driver: self,
                })
                .collect();
            g_slist_free(gslist);
            devices
        }
    }
}

impl<'a> Drop for DriverContext<'a> {
    fn drop(&mut self) {
        unsafe {
            if let Some(cleanup) = (*self.0.context).cleanup {
                cleanup(self.0.context);
            }
            (*self.0.context).context = null_mut();
        }
    }
}
