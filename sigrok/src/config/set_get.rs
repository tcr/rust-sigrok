use crate::config::option::{GlibTuple, MQ_GVAR_TYPE, TUPLE_GVAR_TYPE};
use crate::data::{Mq, MqFlags, MqType};
use crate::util::Variant;
use crate::SigrokError;
use glib::glib_sys::{
    g_variant_get, g_variant_get_boolean, g_variant_get_double, g_variant_get_int32,
    g_variant_get_string, g_variant_get_uint64, g_variant_is_of_type, g_variant_n_children,
    g_variant_new_boolean, g_variant_new_double, g_variant_new_int32, g_variant_new_string,
    g_variant_new_tuple, g_variant_new_uint32, g_variant_new_uint64, GVariant,
};
use num_rational::Ratio;
use sigrok_sys::{sr_channel_group, sr_config_get, sr_config_set, sr_dev_driver, sr_dev_inst};
use std::convert::TryInto;
use std::ffi::CString;
use std::ops::RangeInclusive;
use std::ptr::{null, null_mut};
use std::{slice, str};

#[derive(Copy, Clone, Debug)]
pub struct ConfigSetGetPointers {
    pub driver: *const sr_dev_driver,
    pub sdi: *const sr_dev_inst,
    pub cg: *const sr_channel_group,
    pub key: u32,
}

impl Default for ConfigSetGetPointers {
    fn default() -> Self {
        ConfigSetGetPointers {
            driver: null(),
            sdi: null(),
            cg: null(),
            key: 0,
        }
    }
}

pub trait SetConfig {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError>;
}

pub trait GetConfig: Sized {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError>;
}

unsafe fn set(config: ConfigSetGetPointers, data: *mut GVariant) -> Result<(), SigrokError> {
    SigrokError::from(sr_config_set(config.sdi, config.cg, config.key, data))
}

unsafe fn get(config: ConfigSetGetPointers) -> Result<Variant, SigrokError> {
    let mut variant = null_mut();
    SigrokError::from(sr_config_get(
        config.driver,
        config.sdi,
        config.cg,
        config.key,
        &mut variant,
    ))?;
    Ok(Variant(variant))
}

unsafe fn get_numeric_range<T: GlibTuple>(variant: *mut GVariant) -> RangeInclusive<T> {
    let mut low = T::zero();
    let mut high = T::zero();
    g_variant_get(variant, T::get_tt_type(), &mut low, &mut high);
    low..=high
}

impl SetConfig for bool {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        set(config, g_variant_new_boolean(if *self { 1 } else { 0 }))
    }
}

impl GetConfig for bool {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        Ok(g_variant_get_boolean(c.0) == 1)
    }
}

impl SetConfig for u64 {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        set(config, g_variant_new_uint64(*self))
    }
}

impl GetConfig for u64 {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        Ok(g_variant_get_uint64(c.0))
    }
}

impl SetConfig for f64 {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        set(config, g_variant_new_double(*self))
    }
}

impl GetConfig for f64 {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        Ok(g_variant_get_double(c.0))
    }
}

impl SetConfig for i32 {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        set(config, g_variant_new_int32(*self))
    }
}

impl GetConfig for i32 {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        Ok(g_variant_get_int32(c.0))
    }
}

impl SetConfig for str {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        let string = CString::new(self.as_bytes())?;
        set(config, g_variant_new_string(string.as_ptr()))
    }
}

impl GetConfig for String {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        let mut length = 0;
        let s = g_variant_get_string(c.0, &mut length);
        // GLib guarantees that strings are valid UTF-8
        Ok(str::from_utf8_unchecked(slice::from_raw_parts(s as *const u8, length)).to_owned())
    }
}

impl SetConfig for RangeInclusive<f64> {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        let mut range = [null_mut(); 2];
        range[0] = g_variant_new_double(*self.start());
        range[1] = g_variant_new_double(*self.end());
        set(config, g_variant_new_tuple(range.as_ptr(), range.len()))
    }
}

impl GetConfig for RangeInclusive<f64> {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        Ok(get_numeric_range(c.0))
    }
}

impl SetConfig for RangeInclusive<u64> {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        let mut range = [null_mut(); 2];
        range[0] = g_variant_new_uint64(*self.start());
        range[1] = g_variant_new_uint64(*self.end());
        set(config, g_variant_new_tuple(range.as_ptr(), range.len()))
    }
}

impl GetConfig for RangeInclusive<u64> {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        Ok(get_numeric_range(c.0))
    }
}

impl SetConfig for Ratio<u64> {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        (*self.numer()..=*self.denom()).set_config(config)
    }
}

impl GetConfig for Ratio<u64> {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        let range = get_numeric_range(c.0);
        Ok(Ratio::new_raw(*range.start(), *range.end()))
    }
}

impl SetConfig for Mq {
    unsafe fn set_config(&self, config: ConfigSetGetPointers) -> Result<(), SigrokError> {
        let mut range = [null_mut(); 2];
        range[0] = g_variant_new_uint32(self.mq_type.into());
        range[1] = g_variant_new_uint64(self.flags.bits() as u64);
        set(config, g_variant_new_tuple(range.as_ptr(), range.len()))
    }
}

impl GetConfig for Mq {
    unsafe fn get_config(config: ConfigSetGetPointers) -> Result<Self, SigrokError> {
        let c = get(config)?;
        if g_variant_is_of_type(c.0, TUPLE_GVAR_TYPE as *const _) != 0
            && g_variant_n_children(c.0) == 2
        {
            let mut mq = 0u32;
            let mut mq_flags = 0u64;
            g_variant_get(c.0, MQ_GVAR_TYPE, &mut mq, &mut mq_flags);
            Ok(Mq {
                mq_type: mq.try_into().unwrap_or(MqType::Voltage),
                flags: MqFlags::from_bits_truncate(mq_flags as u32),
            })
        } else {
            Err(SigrokError::Arg)
        }
    }
}
