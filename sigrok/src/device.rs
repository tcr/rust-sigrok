//! Devices
//!
//! This module contains structs for devices and their channels.

use crate::config::internal::ConfigurablePtr;
use crate::config::ConfigSetGetPointers;
use crate::util::{get_functions, gslist_iter};
use crate::{c_str, DriverContext, Function, SigrokError};
use sigrok_sys::{
    sr_channel, sr_channel_group, sr_dev_channel_enable, sr_dev_inst,
    sr_dev_inst_channel_groups_get, sr_dev_inst_channels_get, sr_dev_inst_connid_get,
    sr_dev_inst_model_get, sr_dev_inst_sernum_get, sr_dev_inst_vendor_get, sr_dev_inst_version_get,
};
use std::borrow::Cow;
use std::ptr::null_mut;

/// A device, as obtained by [`scan`][crate::DriverContext::scan] or
/// [`devices`][crate::DriverContext::devices].
///
/// This is configurable via the [`Configurable`][crate::config::Configurable] trait.
#[derive(Debug, Clone)]
pub struct Device<'a> {
    pub(crate) context: *mut sr_dev_inst,
    pub(crate) driver: &'a DriverContext<'a>,
}

impl<'a> Device<'a> {
    pub fn vendor<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str(sr_dev_inst_vendor_get(self.context)) }
    }
    pub fn model<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str(sr_dev_inst_model_get(self.context)) }
    }
    pub fn version<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str(sr_dev_inst_version_get(self.context)) }
    }
    pub fn serial_number<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str(sr_dev_inst_sernum_get(self.context)) }
    }
    pub fn conn_id<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str(sr_dev_inst_connid_get(self.context)) }
    }

    pub fn channels<'b>(&'b self) -> Vec<Channel<'a>> {
        unsafe {
            let gslist = sr_dev_inst_channels_get(self.context);
            gslist_iter(gslist)
                .map(|data| Channel {
                    context: data as *mut sr_channel,
                    device: self.clone(),
                })
                .collect()
        }
    }

    pub fn channel_groups<'b>(&'b self) -> Vec<ChannelGroup<'a>> {
        unsafe {
            let gslist = sr_dev_inst_channel_groups_get(self.context);
            gslist_iter(gslist)
                .map(|data| ChannelGroup {
                    context: data as *mut sr_channel_group,
                    device: self.clone(),
                })
                .collect()
        }
    }

    pub fn functions(&self) -> Result<Vec<Function>, SigrokError> {
        unsafe { get_functions(self.driver.context, self.context, null_mut()) }
    }
}

impl ConfigurablePtr for Device<'_> {
    fn ptr(&self) -> ConfigSetGetPointers {
        ConfigSetGetPointers {
            driver: self.driver.context,
            sdi: self.context,
            ..Default::default()
        }
    }
}

/// A channel, as obtained by [`channels`][Device::channels].
#[derive(Debug, Clone)]
pub struct Channel<'a> {
    pub(crate) context: *mut sr_channel,
    device: Device<'a>,
}

impl<'a> Channel<'a> {
    pub fn index(&self) -> u32 {
        unsafe { (*self.context).index as u32 }
    }

    pub fn name<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str((*self.context).name) }
    }

    /// Disable the channel.
    pub fn disable(&self) -> Result<(), SigrokError> {
        unsafe { SigrokError::from(sr_dev_channel_enable(self.context, 0)) }
    }

    /// Enable the channel.
    pub fn enable(&self) -> Result<(), SigrokError> {
        unsafe { SigrokError::from(sr_dev_channel_enable(self.context, 1)) }
    }
}

/// A channel group, as obtained by [`channel_groups`][Device::channel_groups].
///
/// This is configurable via the [`Configurable`][crate::config::Configurable] trait.
#[derive(Debug, Clone)]
pub struct ChannelGroup<'a> {
    context: *mut sr_channel_group,
    device: Device<'a>,
}

impl<'a> ChannelGroup<'a> {
    pub fn name<'b>(&'b self) -> Cow<'a, str> {
        unsafe { c_str((*self.context).name) }
    }

    pub fn functions(&self) -> Result<Vec<Function>, SigrokError> {
        unsafe {
            get_functions(
                self.device.driver.context,
                self.device.context,
                self.context,
            )
        }
    }
}

impl ConfigurablePtr for ChannelGroup<'_> {
    fn ptr(&self) -> ConfigSetGetPointers {
        ConfigSetGetPointers {
            driver: self.device.driver.context,
            sdi: self.device.context,
            cg: self.context,
            ..Default::default()
        }
    }
}
