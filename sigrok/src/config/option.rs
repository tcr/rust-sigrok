//! Possible configuration options
//!
//! This module contains wrapper types for the possible configuration values

use crate::config::ConfigPointers;
use crate::data::{Mq, MqFlags, MqType};
use crate::util::Variant;
use crate::{SigrokError, TriggerType};
use glib::glib_sys::g_variant_get;
use num_rational::Ratio;
use sigrok_sys::sr_config_list;
use std::convert::TryInto;
use std::ffi::CStr;
use std::ops::RangeInclusive;
use std::os::raw::c_char;
use std::ptr::null_mut;

macro_rules! struct_items {
    // no items
    ($(#[$outer:meta])* struct $name:ident) => { $(#[$outer])* pub struct $name; };
    // tuple struct
    ($(#[$outer:meta])* struct $name:ident ($($vis:vis $ty:ty),+$(,)?)) => { $(#[$outer])* pub struct $name ($($vis $ty),+); };
    // regular {a: i32} struct
    ($(#[$outer:meta])* struct $name:ident $body:tt) => { $(#[$outer])* pub struct $name $body };
    ($(#[$outer:meta])* enum $name:ident $body:tt) => { $(#[$outer])* pub enum $name $body };
}

macro_rules! option {
    (
        $(
	        $(#[$outer:meta])*
	        $item_type:ident
	        $name:ident
	        // Accept regular {a: i32} structs and enums
	        $({$($body:tt)*})?
	        // Accept tuple (i32) structs
	        $(($($vis:vis $tuple_ty:ty),+$(,)?))?
	    ),+$(,)?
    ) => {
        $(
            struct_items!{
                $(#[$outer])*
                #[derive(Clone, PartialEq, Debug)]
                $item_type $name $({$($body)*})?$(($($vis$tuple_ty),+))?
            }
        )+
    };
}

option! {
    /// The configuration expects a [`bool`]. This has no members because it accepts both `true` and
    /// `false`.
    #[derive(Default)]
    struct BoolOption,
    /// The possible [`String`]s that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct StringOption(pub Vec<String>),
    /// The possible [`u64`] ranges that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct U64RangeOption(pub Vec<RangeInclusive<u64>>),
    /// The possible [`f64`] ranges that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct F64RangeOption(pub Vec<RangeInclusive<f64>>),
    /// The possible [rational numbers][Ratio] that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct RationalOption(pub Vec<Ratio<u64>>),
    /// The possible [`u64`]s that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct U64Option(pub Vec<u64>),
    /// The possible sample rates that this may be configured with.
    enum SampleRateOption {
        /// There are a fixed number of sample rates that may be configured.
        Fixed(Vec<u64>),
        /// There are a range of sample rates that may be configured. Any sample rate within the
        /// range with the fixed step are valid. For example:
        ///
        /// ```
        /// use sigrok::config::option::SampleRateOption;
        /// let s = SampleRateOption::Range { range: 1..=10, step: 2 };
        /// match s {
        ///     SampleRateOption::Range {range, step} => {
        ///         assert!(range.step_by(step as usize).eq([1, 3, 5, 7, 9].iter().copied()));
        ///     }
        ///     _ => panic!()
        /// }
        /// ```
        Range { range: RangeInclusive<u64>, step: u64 },
        Unknown,
    },
    /// The possible [`f64`]s that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct F64Option(pub Vec<f64>),
    /// The possible [`i32`]s that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct I32Option(pub Vec<i32>),
    /// The possible [`TriggerType`]s that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct TriggerOption(pub Vec<TriggerType>),
    /// The possible [`Mq`]s that this may be configured with.
    ///
    /// When viewing these docs, be sure to click "Show declaration", as you may access the possible
    /// configuration values with `.0`.
    #[derive(Default)]
    struct MqOption(pub Vec<Mq>),
}

pub(crate) const TUPLE_GVAR_TYPE: *const c_char = b"r\0".as_ptr() as *const c_char;
pub(crate) const MQ_GVAR_TYPE: *const c_char = b"(ut)\0".as_ptr() as *const c_char;

pub(crate) trait GlibTuple: Copy {
    fn get_tt_type() -> *const glib_sys::GRefString;
    fn zero() -> Self;
}

impl GlibTuple for f64 {
    fn get_tt_type() -> *const glib_sys::GRefString {
        b"(dd)\0".as_ptr() as *const _
    }

    fn zero() -> Self {
        0.
    }
}

impl GlibTuple for u64 {
    fn get_tt_type() -> *const glib_sys::GRefString {
        b"(tt)\0".as_ptr() as *const _
    }

    fn zero() -> Self {
        0
    }
}

unsafe fn numeric_option<T: Copy>(value: u32, p: ConfigPointers) -> Option<Vec<T>> {
    let variant = get_variant(value, p)?;
    Some(variant.get_fixed_array()?.iter().copied().collect())
}

unsafe fn numeric_range_iterator<T: GlibTuple>(
    value: u32,
    p: ConfigPointers,
) -> Option<impl Iterator<Item = RangeInclusive<T>>> {
    let variant = get_variant(value, p)?;
    Some(
        (0..variant.num_children())
            .filter_map(move |i| variant.get_child_value(i))
            .map(|child| {
                let mut low = T::zero();
                let mut high = T::zero();
                g_variant_get(child.0, T::get_tt_type(), &mut low, &mut high);
                low..=high
            }),
    )
}

unsafe fn numeric_range_option<T: GlibTuple>(
    value: u32,
    p: ConfigPointers,
) -> Option<Vec<RangeInclusive<T>>> {
    numeric_range_iterator(value, p).map(Iterator::collect)
}

unsafe fn get_variant(value: u32, p: ConfigPointers) -> Option<Variant> {
    let mut variant = null_mut();
    SigrokError::from(sr_config_list(p.driver, p.sdi, p.cg, value, &mut variant)).ok()?;
    Some(Variant(variant))
}

impl BoolOption {
    pub(super) unsafe fn from_sigrok(_value: u32, _p: ConfigPointers) -> Option<Self> {
        Some(BoolOption)
    }
}

impl StringOption {
    #[allow(clippy::let_and_return)]
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        let func = || {
            let variant = get_variant(value, p)?;
            let res = Some(StringOption(
                variant
                    .get_str_array()?
                    .iter()
                    .map(|&t| CStr::from_ptr(t).to_string_lossy().into_owned())
                    .collect(),
            ));
            res
        };
        func().or_else(|| Some(StringOption(vec![])))
    }
}

impl U64RangeOption {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        numeric_range_option(value, p).map(U64RangeOption)
    }
}

impl F64RangeOption {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        numeric_range_option(value, p).map(F64RangeOption)
    }
}

impl F64Option {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        numeric_option(value, p).map(F64Option)
    }
}

impl U64Option {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        numeric_option(value, p).map(U64Option)
    }
}

impl I32Option {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        numeric_option(value, p).map(I32Option)
    }
}

impl SampleRateOption {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        let variant = get_variant(value, p)?;
        if let Some(variant) = variant.lookup_value(b"samplerates\0", b"at\0") {
            Some(SampleRateOption::Fixed(
                variant.get_fixed_array()?.iter().copied().collect(),
            ))
        } else if let Some(variant) = variant.lookup_value(b"samplerate-steps\0", b"at\0") {
            let mut vals = variant.get_fixed_array()?.iter().copied();
            Some(SampleRateOption::Range {
                range: (vals.next()?)..=(vals.next()?),
                step: vals.next()?,
            })
        } else {
            None
        }
    }
}

impl Default for SampleRateOption {
    fn default() -> Self {
        SampleRateOption::Unknown
    }
}

impl TriggerOption {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        let variant = get_variant(value, p)?;
        Some(TriggerOption(
            variant
                .get_fixed_array::<i32>()?
                .iter()
                .filter_map(|&t| t.try_into().ok())
                .collect(),
        ))
    }
}

impl RationalOption {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        Some(RationalOption(
            numeric_range_iterator(value, p)?
                .map(|range| Ratio::new_raw(*range.start(), *range.end()))
                .collect(),
        ))
    }
}

impl MqOption {
    pub(super) unsafe fn from_sigrok(value: u32, p: ConfigPointers) -> Option<Self> {
        let variant = get_variant(value, p)?;
        Some(MqOption(
            (0..variant.num_children())
                .filter_map(move |i| variant.get_child_value(i))
                .map(|variant| {
                    let mut mq = 0u32;
                    let mut mq_flags = 0u64;
                    g_variant_get(variant.0, MQ_GVAR_TYPE, &mut mq, &mut mq_flags);
                    Mq {
                        mq_type: mq.try_into().unwrap_or(MqType::Voltage),
                        flags: MqFlags::from_bits_truncate(mq_flags as u32),
                    }
                })
                .collect(),
        ))
    }
}
